use crate::{
    agent::{Agent, AgentId, AgentState, AgentStatus},
    coord::TileCoord,
    entity::Entity,
    event::TickEvent,
    gen,
    grid::Grid,
    item::ItemStack,
    tile::Tile,
};
use std::collections::BTreeMap;

pub const RESPAWN_DELAY: u64 = 30;
pub const ITEM_DROP_TTL: u64 = 1800;

pub fn handle_deaths(
    tick: u64,
    seed: u64,
    grid: &Grid<Tile>,
    agents: &mut BTreeMap<AgentId, Agent>,
    entities: &mut BTreeMap<TileCoord, Entity>,
) -> Vec<TickEvent> {
    let mut events = Vec::new();
    for (id, a) in agents.iter_mut() {
        if matches!(a.state, AgentState::Alive) && a.status.hp <= 0 {
            let pos = a.pos;
            let stacks: Vec<ItemStack> = std::mem::take(&mut a.inventory.slots);
            for (i, stack) in stacks.into_iter().enumerate() {
                let drop_pos =
                    nearby_slot(grid, entities, pos, seed.wrapping_add(tick).wrapping_add(i as u64))
                        .unwrap_or(pos);
                entities.insert(
                    drop_pos,
                    Entity::ItemDrop {
                        stack,
                        expires_at: tick + ITEM_DROP_TTL,
                    },
                );
            }
            a.state = AgentState::Dying {
                revives_at_tick: tick + RESPAWN_DELAY,
            };
            a.status = AgentStatus {
                hp: 0,
                hunger: 0,
                stamina: 0,
                warmth: 0,
                sanity: 0,
            };
            events.push(TickEvent::AgentDied {
                agent: id.clone(),
                at: pos,
                cause: "starvation".into(),
            });
        }
    }
    events
}

pub fn handle_respawns(
    tick: u64,
    seed: u64,
    grid: &Grid<Tile>,
    agents: &mut BTreeMap<AgentId, Agent>,
) -> Vec<TickEvent> {
    let mut events = Vec::new();
    for (id, a) in agents.iter_mut() {
        if let AgentState::Dying { revives_at_tick } = a.state {
            if tick >= revives_at_tick {
                let pos = gen::find_safe_spawn(grid, seed ^ tick.wrapping_mul(0xABCD_1234));
                a.pos = pos;
                a.status = AgentStatus::fresh();
                a.state = AgentState::Alive;
                a.last_action_tick = tick;
                events.push(TickEvent::AgentRespawned {
                    agent: id.clone(),
                    at: pos,
                });
            }
        }
    }
    events
}

fn nearby_slot(
    grid: &Grid<Tile>,
    entities: &BTreeMap<TileCoord, Entity>,
    center: TileCoord,
    salt: u64,
) -> Option<TileCoord> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(salt);
    let mut candidates: Vec<TileCoord> = (-2..=2)
        .flat_map(|dy| (-2..=2).map(move |dx| TileCoord::new(center.x + dx, center.y + dy)))
        .filter(|c| grid.get(*c).map(|t| t.is_walkable()).unwrap_or(false))
        .filter(|c| !entities.contains_key(c))
        .collect();
    candidates.shuffle(&mut rng);
    candidates.into_iter().next()
}
