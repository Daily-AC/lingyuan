use crate::{
    db::DbWrite,
    state::{AppState, SpectatorAgent, SpectatorEntity, SpectatorView, TickFrame},
};
use std::time::Duration;
use tracing::{debug, info, warn};
use world::World;

fn collect_spectator_entities(w: &World) -> Vec<SpectatorEntity> {
    let mut out = Vec::with_capacity(w.entities.len() + w.buildings.len());
    for (pos, e) in &w.entities {
        match e {
            world::Entity::Plant { plant } => {
                let kind = format!("plant:{}", serde_plain(&plant.kind));
                out.push(SpectatorEntity {
                    pos: *pos,
                    kind,
                    label: None,
                    id: None,
                });
            }
            world::Entity::ItemDrop { stack, .. } => {
                out.push(SpectatorEntity {
                    pos: *pos,
                    kind: format!("drop:{}", serde_plain(&stack.item)),
                    label: Some(format!("×{}", stack.n)),
                    id: None,
                });
            }
        }
    }
    for (pos, b) in &w.buildings {
        out.push(SpectatorEntity {
            pos: *pos,
            kind: format!("building:{}", serde_plain(&b.kind)),
            label: None,
            id: None,
        });
    }
    for c in w.creatures.values() {
        out.push(SpectatorEntity {
            pos: c.pos,
            kind: format!("creature:{}", serde_plain(&c.kind)),
            label: Some(format!("{}/{}", c.hp, c.kind.max_hp())),
            id: Some(c.id),
        });
    }
    for (pos, sign) in &w.signs {
        // 截 30 字预览
        let preview: String = sign.text.chars().take(30).collect();
        out.push(SpectatorEntity {
            pos: *pos,
            kind: "sign:default".into(),
            label: Some(preview),
            id: None,
        });
    }
    out
}

/// 一个事件是否和某 agent 直接相关（被打/打人/失败/死/重生 等）。
/// 不含全局事件（boss spawn / 季节切换），那些走 spectator 流。
fn event_involves_agent(e: &world::TickEvent, aid: &world::AgentId) -> bool {
    use world::TickEvent::*;
    match e {
        AgentJoined { agent, .. }
        | AgentLeft { agent, .. }
        | AgentMoved { agent, .. }
        | AgentMoveFailed { agent, .. }
        | AgentGathered { agent, .. }
        | AgentGatherFailed { agent, .. }
        | AgentAte { agent, .. }
        | AgentCrafted { agent, .. }
        | AgentCraftFailed { agent, .. }
        | AgentPlaced { agent, .. }
        | AgentPickedUp { agent, .. }
        | AgentDropped { agent, .. }
        | AgentDied { agent, .. }
        | AgentRespawned { agent, .. }
        | AgentWroteSign { agent, .. }
        | AgentAttackFailed { agent, .. } => agent == aid,
        AgentAttackedAgent { attacker, target, .. } => attacker == aid || target == aid,
        AgentAttackedCreature { attacker, .. } => attacker == aid,
        CreatureAttackedAgent { target, .. } => target == aid,
        AgentSentMail { from, .. } => from == aid,
        _ => false,
    }
}

fn serde_plain<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into())
}

pub async fn run(state: AppState) {
    use std::sync::atomic::Ordering;
    let mut cur_ms = state.tick_ms.load(Ordering::Relaxed);
    let mut ticker = tokio::time::interval(Duration::from_millis(cur_ms));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;
        let new_ms = state.tick_ms.load(Ordering::Relaxed);
        if new_ms != cur_ms && new_ms >= 50 && new_ms <= 5000 {
            cur_ms = new_ms;
            ticker = tokio::time::interval(Duration::from_millis(cur_ms));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            info!(tick_ms = cur_ms, "tick rate changed");
        }

        // 排空 pending 表：每个 agent 至多一个 action 落地。
        let actions: Vec<_> = {
            let mut pending = state.pending.lock().await;
            pending
                .drain()
                .map(|(aid, p)| (aid, p.action))
                .collect()
        };

        let mut w = state.world.lock().await;
        let events = w.step(actions);

        // 把每个 agent 相关的事件落到共享缓存里，observe 时读出来。
        // 完整覆写而不是追加：只暴露"上一 tick 发生的事"，避免无限堆。
        {
            let mut by_agent = state.recent_events_by_agent.lock().await;
            by_agent.clear();
            for aid in w.agents.keys() {
                let rel: Vec<world::TickEvent> = events
                    .iter()
                    .filter(|e| event_involves_agent(e, aid))
                    .cloned()
                    .collect();
                if !rel.is_empty() {
                    by_agent.insert(aid.clone(), rel);
                }
            }
        }

        let observations = w
            .agents
            .keys()
            .cloned()
            .filter_map(|id| w.observe(&id).map(|obs| (id, obs)))
            .collect();

        let spectator = SpectatorView {
            tick: w.clock.tick,
            clock: w.clock,
            agents: w
                .agents
                .values()
                .map(|a| SpectatorAgent {
                    id: a.id.clone(),
                    name: a.name.clone(),
                    pos: a.pos,
                    hp: a.status.hp,
                    hunger: a.status.hunger,
                    stamina: a.status.stamina,
                    state: match a.state {
                        world::AgentState::Alive => "alive".into(),
                        world::AgentState::Dying { .. } => "dying".into(),
                        world::AgentState::Meditating { .. } => "meditating".into(),
                    },
                    inventory: a.inventory.slots.clone(),
                })
                .collect(),
            entities: collect_spectator_entities(&w),
            events: events.clone(),
        };

        let frame = TickFrame {
            tick: w.clock.tick,
            clock: w.clock,
            events,
            spectator_view: spectator,
            observations,
        };

        if state.db_tx.try_send(DbWrite::Frame(frame.clone())).is_err() {
            warn!("db writer queue full, dropping frame {}", frame.tick);
        }
        if w.clock.tick > 0 && w.clock.tick % state.config.snapshot_every == 0 {
            let snap = w.clone();
            let _ = state.db_tx.try_send(DbWrite::Snapshot(snap));
            debug!(tick = w.clock.tick, "queued snapshot");
        }

        let _ = state.frames_tx.send(frame);

        if w.clock.tick % 30 == 0 {
            info!(tick = w.clock.tick, agents = w.agent_count(), "tick");
        }
    }
}
