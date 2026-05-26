use crate::{
    agent::{Agent, AgentId, AgentState},
    coord::TileCoord,
    creature::{Creature, CreatureId},
    event::TickEvent,
    grid::Grid,
    tile::Tile,
};
use std::collections::BTreeMap;

pub fn step_creatures(
    _tick: u64,
    grid: &Grid<Tile>,
    agents: &mut BTreeMap<AgentId, Agent>,
    creatures: &mut BTreeMap<CreatureId, Creature>,
) -> Vec<TickEvent> {
    let mut events = Vec::new();
    let ids: Vec<CreatureId> = creatures.keys().copied().collect();
    for cid in ids {
        let Some(c) = creatures.get(&cid).cloned() else {
            continue;
        };
        let vision = c.kind.vision();
        let nearest = agents
            .iter()
            .filter(|(_, a)| matches!(a.state, AgentState::Alive))
            .filter(|(_, a)| a.pos.manhattan(c.pos) <= vision)
            .min_by_key(|(_, a)| a.pos.manhattan(c.pos));
        let Some((aid, agent)) = nearest else {
            continue;
        };
        let dx = (agent.pos.x - c.pos.x).signum();
        let dy = (agent.pos.y - c.pos.y).signum();
        if c.kind.is_hostile() {
            if c.pos.manhattan(agent.pos) <= 1 {
                let aid_owned = aid.clone();
                let creature_attack = c.kind.attack();
                if let Some(a_mut) = agents.get_mut(&aid_owned) {
                    a_mut.status.hp = (a_mut.status.hp - creature_attack).max(0);
                }
                events.push(TickEvent::CreatureAttackedAgent {
                    creature_id: cid,
                    creature_kind: c.kind,
                    target: aid_owned,
                    damage: creature_attack,
                });
                continue;
            }
            if let Some(p) = pick_step_towards(grid, agents, creatures, c.pos, dx, dy) {
                creatures.get_mut(&cid).unwrap().pos = p;
            }
        } else {
            if let Some(p) = pick_step_towards(grid, agents, creatures, c.pos, -dx, -dy) {
                creatures.get_mut(&cid).unwrap().pos = p;
            }
        }
    }
    events
}

fn pick_step_towards(
    grid: &Grid<Tile>,
    agents: &BTreeMap<AgentId, Agent>,
    creatures: &BTreeMap<CreatureId, Creature>,
    from: TileCoord,
    dx: i16,
    dy: i16,
) -> Option<TileCoord> {
    let dirs: [(i16, i16); 4] = [(dx, 0), (0, dy), (dx, dy), (-dx, dy)];
    for (mx, my) in dirs {
        if mx == 0 && my == 0 {
            continue;
        }
        let target = TileCoord::new(from.x + mx.signum(), from.y + my.signum());
        if !grid.get(target).map(|t| t.is_walkable()).unwrap_or(false) {
            continue;
        }
        if agents.values().any(|a| a.pos == target) {
            continue;
        }
        if creatures.values().any(|c| c.pos == target) {
            continue;
        }
        return Some(target);
    }
    None
}
