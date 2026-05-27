use crate::{
    db::DbWrite,
    state::{ActionEnvelope, AppState, SpectatorAgent, SpectatorEntity, SpectatorView, TickFrame},
};
use std::time::Duration;
use tokio::sync::mpsc;
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
                });
            }
            world::Entity::ItemDrop { stack, .. } => {
                out.push(SpectatorEntity {
                    pos: *pos,
                    kind: format!("drop:{}", serde_plain(&stack.item)),
                    label: Some(format!("×{}", stack.n)),
                });
            }
        }
    }
    for (pos, b) in &w.buildings {
        out.push(SpectatorEntity {
            pos: *pos,
            kind: format!("building:{}", serde_plain(&b.kind)),
            label: None,
        });
    }
    for c in w.creatures.values() {
        out.push(SpectatorEntity {
            pos: c.pos,
            kind: format!("creature:{}", serde_plain(&c.kind)),
            label: Some(format!("hp{}", c.hp)),
        });
    }
    out
}

fn serde_plain<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|j| j.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into())
}

pub async fn run(state: AppState, mut rx: mpsc::Receiver<ActionEnvelope>) {
    let mut ticker = tokio::time::interval(Duration::from_millis(state.config.tick_ms));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        ticker.tick().await;

        let mut actions = Vec::new();
        while let Ok(env) = rx.try_recv() {
            actions.push((env.agent, env.action));
        }

        let mut w = state.world.lock().await;
        let events = w.step(actions);

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
