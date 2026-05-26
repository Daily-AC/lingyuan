use crate::{
    action::Action,
    agent::{Agent, AgentId, AgentState},
    clock::{Season, WorldClock, TICKS_PER_DAY},
    coord::{Direction, TileCoord},
    event::TickEvent,
    gen,
    grid::Grid,
    observation::{ClockView, Observation, SelfView, VisibleEntity, VisibleTile, VisionView, VISION_RADIUS},
    tile::Tile,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum WorldError {
    #[error("name '{0}' already taken")]
    NameTaken(String),
    #[error("agent {0} not found")]
    AgentNotFound(String),
    #[error("world full (max {0})")]
    WorldFull(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub seed: u64,
    pub clock: WorldClock,
    pub grid: Grid<Tile>,
    pub agents: HashMap<AgentId, Agent>,
    pub max_agents: usize,
    #[serde(skip)]
    pending_events: Vec<TickEvent>,
}

impl World {
    pub fn bootstrap(seed: u64) -> Self {
        Self {
            seed,
            clock: WorldClock::new(),
            grid: gen::generate(seed),
            agents: HashMap::new(),
            max_agents: 64,
            pending_events: Vec::new(),
        }
    }

    pub fn join(&mut self, name: String) -> Result<AgentId, WorldError> {
        if self.agents.values().any(|a| a.name == name) {
            return Err(WorldError::NameTaken(name));
        }
        if self.agents.len() >= self.max_agents {
            return Err(WorldError::WorldFull(self.max_agents));
        }
        let id = AgentId::new(format!(
            "ag_{:08x}",
            rand_for_id(self.seed, self.clock.tick, self.agents.len() as u64)
        ));
        let pos = gen::find_safe_spawn(&self.grid, self.seed.wrapping_add(self.agents.len() as u64));
        let agent = Agent::new_at(id.clone(), name.clone(), pos, self.clock.tick);
        self.agents.insert(id.clone(), agent);
        self.pending_events.push(TickEvent::AgentJoined {
            agent: id.clone(),
            name,
            at: pos,
        });
        Ok(id)
    }

    pub fn leave(&mut self, id: &AgentId) -> Result<(), WorldError> {
        let a = self
            .agents
            .remove(id)
            .ok_or_else(|| WorldError::AgentNotFound(id.0.clone()))?;
        self.pending_events.push(TickEvent::AgentLeft {
            agent: id.clone(),
            name: a.name,
        });
        Ok(())
    }

    /// 推进一个 tick。actions 是本 tick 收到的 agent 动作。
    pub fn step(&mut self, actions: Vec<(AgentId, Action)>) -> Vec<TickEvent> {
        let mut acts = actions;
        acts.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));

        for (aid, action) in acts {
            self.resolve(&aid, action);
        }

        // 时钟与季节/昼夜事件
        let was_phase_day = self.clock.tick_in_day() < 30;
        let was_season = self.clock.season();
        self.clock.advance();
        let is_phase_day = self.clock.tick_in_day() < 30;
        let day = self.clock.tick as u32 / TICKS_PER_DAY;
        if was_phase_day && !is_phase_day && self.clock.is_night() {
            self.pending_events.push(TickEvent::NightStarted { day });
        } else if !was_phase_day && is_phase_day {
            self.pending_events.push(TickEvent::DayStarted { day });
        }
        let new_season = self.clock.season();
        if new_season != was_season {
            self.pending_events
                .push(TickEvent::SeasonChanged { to: new_season });
        }

        std::mem::take(&mut self.pending_events)
    }

    fn resolve(&mut self, aid: &AgentId, action: Action) {
        let Some(agent) = self.agents.get(aid) else {
            return;
        };
        if !matches!(agent.state, AgentState::Alive) {
            return;
        }
        match action {
            Action::Move { dir } => self.resolve_move(aid.clone(), dir),
            Action::Wait => {}
            Action::Observe => {}
        }
    }

    fn resolve_move(&mut self, aid: AgentId, dir: Direction) {
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        let from = agent.pos;
        let to = from.step(dir);

        let walkable = self.grid.get(to).map(|t| t.is_walkable()).unwrap_or(false);
        let occupied = self.agents.values().any(|a| a.pos == to && a.id != aid);

        if !walkable {
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: "blocked".into(),
            });
            return;
        }
        if occupied {
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: "occupied".into(),
            });
            return;
        }
        let a = self.agents.get_mut(&aid).unwrap();
        a.pos = to;
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentMoved {
            agent: aid,
            from,
            to,
        });
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }

    pub fn observe(&self, viewer: &AgentId) -> Option<Observation> {
        let agent = self.agents.get(viewer)?;
        let center = agent.pos;
        let r = VISION_RADIUS as i16;

        let mut tiles = Vec::new();
        for dy in -r..=r {
            for dx in -r..=r {
                let c = TileCoord::new(center.x + dx, center.y + dy);
                if c.manhattan(center) > VISION_RADIUS {
                    continue;
                }
                if let Some(t) = self.grid.get(c) {
                    tiles.push(VisibleTile { pos: c, tile: *t });
                }
            }
        }

        let mut entities = Vec::new();
        for (id, a) in &self.agents {
            if id == viewer {
                continue;
            }
            if a.pos.manhattan(center) <= VISION_RADIUS {
                entities.push(VisibleEntity::Agent {
                    id: id.clone(),
                    name: a.name.clone(),
                    pos: a.pos,
                    hp: a.status.hp,
                });
            }
        }

        Some(Observation {
            tick: self.clock.tick,
            clock: ClockView {
                day: self.clock.tick as u32 / TICKS_PER_DAY,
                season: self.clock.season(),
                phase: self.clock.phase(),
                tick_in_day: self.clock.tick_in_day(),
            },
            self_: SelfView {
                id: agent.id.clone(),
                name: agent.name.clone(),
                pos: agent.pos,
                status: agent.status,
                state: agent.state,
            },
            vision: VisionView {
                radius: VISION_RADIUS,
                tiles,
            },
            visible_entities: entities,
            recent_events: Vec::new(),
        })
    }
}

fn rand_for_id(seed: u64, tick: u64, n: u64) -> u32 {
    use rand::{Rng, SeedableRng};
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(
        seed.wrapping_mul(0x100000001b3)
            .wrapping_add(tick)
            .wrapping_add(n),
    );
    r.gen()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_assigns_unique_id_and_walkable_pos() {
        let mut w = World::bootstrap(7);
        let id1 = w.join("alice".into()).unwrap();
        let id2 = w.join("bob".into()).unwrap();
        assert_ne!(id1, id2);
        let pos1 = w.agents[&id1].pos;
        assert!(w.grid.get(pos1).unwrap().is_walkable());
    }

    #[test]
    fn duplicate_name_rejected() {
        let mut w = World::bootstrap(7);
        w.join("alice".into()).unwrap();
        let err = w.join("alice".into()).unwrap_err();
        assert!(matches!(err, WorldError::NameTaken(_)));
    }

    #[test]
    fn move_event_emitted_on_walkable() {
        let mut w = World::bootstrap(7);
        let id = w.join("alice".into()).unwrap();
        let orig = w.agents[&id].pos;
        for dir in Direction::ALL {
            let target = orig.step(dir);
            if w.grid.get(target).map(|t| t.is_walkable()).unwrap_or(false) {
                let events = w.step(vec![(id.clone(), Action::Move { dir })]);
                assert!(events
                    .iter()
                    .any(|e| matches!(e, TickEvent::AgentMoved { .. })));
                return;
            }
        }
        panic!("no walkable direction (seed-dependent flake; retry with different seed)");
    }

    #[test]
    fn step_advances_clock() {
        let mut w = World::bootstrap(7);
        let before = w.clock.tick;
        w.step(vec![]);
        assert_eq!(w.clock.tick, before + 1);
    }

    #[test]
    fn determinism_of_step() {
        let mut a = World::bootstrap(42);
        let mut b = World::bootstrap(42);
        a.step(vec![]);
        b.step(vec![]);
        assert_eq!(a.clock, b.clock);
        // grid 已确定，agents 都为空，对比序列化字节
        assert_eq!(
            bincode::serialize(&a).unwrap(),
            bincode::serialize(&b).unwrap()
        );
    }

    #[test]
    fn observe_returns_tiles_within_radius() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let obs = w.observe(&id).unwrap();
        let center = w.agents[&id].pos;
        assert!(obs.vision.tiles.len() > 1);
        for t in &obs.vision.tiles {
            assert!(t.pos.manhattan(center) <= VISION_RADIUS);
        }
    }

    #[test]
    fn season_constant_imported() {
        // 确保 use 没被优化掉，对编译失败给个明确信号
        let _: Season = Season::Chun;
    }
}
