use crate::{
    agent::{Agent, AgentId, AgentState},
    coord::TileCoord,
    creature::{Creature, CreatureId, CreatureKind},
    event::TickEvent,
    grid::Grid,
    tile::Tile,
};
use std::collections::BTreeMap;

/// boss 渡劫者每 BOSS_PERIOD tick 刷一次，若世界没有同类 boss
pub const BOSS_PERIOD: u64 = 1500;

fn maybe_spawn_boss(
    tick: u64,
    seed: u64,
    grid: &Grid<Tile>,
    agents: &BTreeMap<AgentId, Agent>,
    creatures: &mut BTreeMap<CreatureId, Creature>,
    next_creature_id: &mut CreatureId,
    out: &mut Vec<TickEvent>,
) {
    use rand::{Rng, SeedableRng};
    if tick == 0 || tick % BOSS_PERIOD != 0 {
        return;
    }
    if creatures.values().any(|c| c.kind.is_boss()) {
        return;
    }
    let alive: Vec<&Agent> = agents
        .values()
        .filter(|a| matches!(a.state, AgentState::Alive))
        .collect();
    if alive.is_empty() {
        return;
    }
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ tick ^ 0xB055_FACE);
    let center = alive[rng.gen_range(0..alive.len())].pos;
    for _ in 0..40 {
        let dx: i16 = rng.gen_range(-15..=15);
        let dy: i16 = rng.gen_range(-15..=15);
        if dx.abs() + dy.abs() < 10 {
            continue;
        }
        let p = TileCoord::new(center.x + dx, center.y + dy);
        if !grid.get(p).map(|t| t.is_walkable()).unwrap_or(false) {
            continue;
        }
        if agents.values().any(|a| a.pos == p) {
            continue;
        }
        if creatures.values().any(|c| c.pos == p) {
            continue;
        }
        *next_creature_id += 1;
        let id = *next_creature_id;
        creatures.insert(
            id,
            Creature::new(id, CreatureKind::BossDujie, p, tick),
        );
        out.push(TickEvent::BossSpawned {
            id,
            kind: CreatureKind::BossDujie,
            at: p,
            announcement: "渡劫者降世".into(),
        });
        return;
    }
}

pub fn step_spawn(
    tick: u64,
    seed: u64,
    is_night: bool,
    grid: &Grid<Tile>,
    agents: &BTreeMap<AgentId, Agent>,
    creatures: &mut BTreeMap<CreatureId, Creature>,
    next_creature_id: &mut CreatureId,
) -> Vec<TickEvent> {
    use rand::{Rng, SeedableRng};
    let mut out = Vec::new();
    // boss tick 总是检查
    maybe_spawn_boss(tick, seed, grid, agents, creatures, next_creature_id, &mut out);
    if !is_night || tick % 8 != 0 {
        return out;
    }
    if creatures.values().filter(|c| c.kind.is_hostile()).count() >= 32 {
        return out;
    }
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(seed ^ tick);
    let alive: Vec<&Agent> = agents
        .values()
        .filter(|a| matches!(a.state, AgentState::Alive))
        .collect();
    if alive.is_empty() {
        return out;
    }
    let center = alive[rng.gen_range(0..alive.len())].pos;
    for _ in 0..20 {
        let dx: i16 = rng.gen_range(-12..=12);
        let dy: i16 = rng.gen_range(-12..=12);
        if dx.abs() + dy.abs() < 6 {
            continue;
        }
        let p = TileCoord::new(center.x + dx, center.y + dy);
        let walkable = grid.get(p).map(|t| t.is_walkable()).unwrap_or(false);
        if !walkable {
            continue;
        }
        if agents.values().any(|a| a.pos == p) {
            continue;
        }
        if creatures.values().any(|c| c.pos == p) {
            continue;
        }
        let kind = if rng.gen_bool(0.65) {
            CreatureKind::Wolf
        } else {
            CreatureKind::NightDemon
        };
        *next_creature_id += 1;
        let id = *next_creature_id;
        creatures.insert(id, Creature::new(id, kind, p, tick));
        out.push(TickEvent::CreatureSpawned { id, kind, at: p });
        break;
    }
    out
}
