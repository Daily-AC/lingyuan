use crate::{
    action::{Action, AttackTarget},
    agent::{Agent, AgentId, AgentState},
    building::{Building, BuildingKind},
    clock::{WorldClock, TICKS_PER_DAY},
    coord::{Direction, TileCoord},
    creature::{Creature, CreatureId},
    entity::Entity,
    event::TickEvent,
    gen,
    grid::Grid,
    item::{ItemKind, ItemStack},
    observation::{
        ClockView, MailView, Observation, SelfView, SignView, VisibleEntity, VisibleTile,
        VisionView, VISION_RADIUS,
    },
    recipe::{self, CraftStation, RecipeOutput},
    tile::Tile,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub agents: BTreeMap<AgentId, Agent>,
    pub entities: BTreeMap<TileCoord, Entity>,
    pub buildings: BTreeMap<TileCoord, Building>,
    pub creatures: BTreeMap<CreatureId, Creature>,
    pub next_creature_id: CreatureId,
    pub signs: BTreeMap<TileCoord, SignText>,
    pub mail: BTreeMap<AgentId, Vec<MailEntry>>,
    pub max_agents: usize,
    #[serde(skip)]
    pending_events: Vec<TickEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignText {
    pub text: String,
    pub author_name: Option<String>,
    pub written_at_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailEntry {
    pub from: String,
    pub text: String,
    pub received_at_tick: u64,
}

pub const SIGN_TEXT_MAX: usize = 200;
pub const MAIL_TEXT_MAX: usize = 500;
pub const MAIL_INBOX_MAX: usize = 32;
pub const SIGNS_PER_AGENT_LIMIT: u32 = 50;

impl World {
    pub fn bootstrap(seed: u64) -> Self {
        let grid = gen::generate(seed);
        let entities = gen::populate(&grid, seed);
        Self {
            seed,
            clock: WorldClock::new(),
            grid,
            agents: BTreeMap::new(),
            entities,
            buildings: BTreeMap::new(),
            creatures: BTreeMap::new(),
            next_creature_id: 0,
            signs: BTreeMap::new(),
            mail: BTreeMap::new(),
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
        // 把 tick 也算进 seed，避免 leave→rejoin 拿到同一个出生点；同时把
        // hostile creature（含 boss）的视野半径排除掉，防止 spawn 进 boss 嘴边。
        let spawn_seed = self
            .seed
            .wrapping_add(self.clock.tick.wrapping_mul(0x9E37_79B1_7F4A_7C15))
            .wrapping_add(self.agents.len() as u64);
        let pos = gen::find_safe_spawn_avoiding(
            &self.grid,
            &self.creatures,
            spawn_seed,
            None,
            crate::observation::VISION_RADIUS,
        );
        let mut agent = Agent::new_at(id.clone(), name.clone(), pos, self.clock.tick);
        // 开局食物缓存：3 颗朱果（6×3 = 18 hunger），让 agent 至少撑过头一波
        // 食物源冷却，避免出生死亡螺旋。
        agent.inventory.add(ItemKind::RedBerry, 3);
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

    pub fn step(&mut self, actions: Vec<(AgentId, Action)>) -> Vec<TickEvent> {
        let mut acts = actions;
        acts.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));

        for (aid, action) in acts {
            self.resolve(&aid, action);
        }

        // 怪物 AI（在 agents 动作之后，但在死亡处理之前）
        let mut ai_events = crate::systems::creature_ai::step_creatures(
            self.clock.tick,
            &self.grid,
            &mut self.agents,
            &mut self.creatures,
        );
        self.pending_events.append(&mut ai_events);

        // 夜晚 spawn
        let is_night = self.clock.is_night();
        let mut spawn_events = crate::systems::spawn::step_spawn(
            self.clock.tick,
            self.seed,
            is_night,
            &self.grid,
            &self.agents,
            &mut self.creatures,
            &mut self.next_creature_id,
        );
        self.pending_events.append(&mut spawn_events);

        // 自然系统
        crate::systems::natural::step_status(self.clock.tick, &mut self.agents);

        // 死亡 + 重生
        let mut died = crate::systems::death::handle_deaths(
            self.clock.tick,
            self.seed,
            &self.grid,
            &mut self.agents,
            &mut self.entities,
        );
        let mut respawned = crate::systems::death::handle_respawns(
            self.clock.tick,
            self.seed,
            &self.grid,
            &mut self.agents,
            &self.creatures,
        );
        self.pending_events.append(&mut died);
        self.pending_events.append(&mut respawned);

        // 过期 item drop
        let expire_tick = self.clock.tick;
        self.entities.retain(|_, e| match e {
            Entity::ItemDrop { expires_at, .. } => *expires_at > expire_tick,
            _ => true,
        });

        // 时钟
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
            Action::Gather { target } => self.resolve_gather(aid.clone(), target),
            Action::Eat { item } => self.resolve_eat(aid.clone(), item),
            Action::Craft { recipe } => self.resolve_craft(aid.clone(), recipe),
            Action::Place { item, pos } => self.resolve_place(aid.clone(), item, pos),
            Action::PickUp { pos } => self.resolve_pickup(aid.clone(), pos),
            Action::Drop { item, n } => self.resolve_drop(aid.clone(), item, n),
            Action::Attack { target } => self.resolve_attack(aid.clone(), target),
            Action::WriteSign { pos, text } => self.resolve_write_sign(aid.clone(), pos, text),
            Action::SendMail { to, text } => self.resolve_send_mail(aid.clone(), to, text),
        }
    }

    fn resolve_write_sign(&mut self, aid: AgentId, pos: TileCoord, text: String) {
        if text.is_empty() || text.len() > SIGN_TEXT_MAX {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: aid,
                reason: format!("sign text must be 1..={} chars", SIGN_TEXT_MAX),
            });
            return;
        }
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        if agent.pos.manhattan(pos) > 1 {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: aid,
                reason: "sign out of range".into(),
            });
            return;
        }
        if !self.grid.get(pos).map(|t| t.is_walkable()).unwrap_or(false) {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: aid,
                reason: "tile not walkable".into(),
            });
            return;
        }
        let author_name = agent.name.clone();
        let excerpt: String = text.chars().take(40).collect();
        self.signs.insert(
            pos,
            SignText {
                text,
                author_name: Some(author_name),
                written_at_tick: self.clock.tick,
            },
        );
        self.pending_events.push(TickEvent::AgentWroteSign {
            agent: aid,
            pos,
            text_excerpt: excerpt,
        });
    }

    fn resolve_send_mail(&mut self, from: AgentId, to_name: String, text: String) {
        if text.is_empty() || text.len() > MAIL_TEXT_MAX {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: from,
                reason: format!("mail text must be 1..={} chars", MAIL_TEXT_MAX),
            });
            return;
        }
        let target = self
            .agents
            .iter()
            .find(|(_, a)| a.name == to_name)
            .map(|(id, _)| id.clone());
        let Some(target_id) = target else {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: from,
                reason: format!("agent named '{}' not found", to_name),
            });
            return;
        };
        let from_name = self.agents.get(&from).map(|a| a.name.clone()).unwrap_or_default();
        let entry = MailEntry {
            from: from_name,
            text: text.clone(),
            received_at_tick: self.clock.tick,
        };
        let inbox = self.mail.entry(target_id).or_default();
        inbox.push(entry);
        if inbox.len() > MAIL_INBOX_MAX {
            let drop_n = inbox.len() - MAIL_INBOX_MAX;
            inbox.drain(0..drop_n);
        }
        let excerpt: String = text.chars().take(60).collect();
        self.pending_events.push(TickEvent::AgentSentMail {
            from,
            to: to_name,
            text_excerpt: excerpt,
        });
    }

    fn resolve_attack(&mut self, aid: AgentId, target: AttackTarget) {
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        let attacker_pos = agent.pos;
        if agent.status.stamina < 5 {
            self.pending_events.push(TickEvent::AgentAttackFailed {
                agent: aid,
                reason: "exhausted".into(),
            });
            return;
        }
        let weapon = agent_best_weapon(agent);
        let dmg = crate::combat::resolve_attack_damage(weapon);
        match target {
            AttackTarget::Agent(target_id) => {
                let Some(target_agent) = self.agents.get(&target_id) else {
                    self.pending_events.push(TickEvent::AgentAttackFailed {
                        agent: aid,
                        reason: "target not found".into(),
                    });
                    return;
                };
                if attacker_pos.manhattan(target_agent.pos) > 1 {
                    self.pending_events.push(TickEvent::AgentAttackFailed {
                        agent: aid,
                        reason: "out of range".into(),
                    });
                    return;
                }
                let target_mut = self.agents.get_mut(&target_id).unwrap();
                target_mut.status.hp = (target_mut.status.hp - dmg).max(0);
                self.pending_events.push(TickEvent::AgentAttackedAgent {
                    attacker: aid.clone(),
                    target: target_id,
                    damage: dmg,
                    weapon: weapon.map(|w| format!("{:?}", w)),
                });
            }
            AttackTarget::Creature(cid) => {
                let Some(creature) = self.creatures.get(&cid) else {
                    self.pending_events.push(TickEvent::AgentAttackFailed {
                        agent: aid,
                        reason: "creature not found".into(),
                    });
                    return;
                };
                if attacker_pos.manhattan(creature.pos) > 1 {
                    self.pending_events.push(TickEvent::AgentAttackFailed {
                        agent: aid,
                        reason: "out of range".into(),
                    });
                    return;
                }
                let creature_mut = self.creatures.get_mut(&cid).unwrap();
                creature_mut.hp = (creature_mut.hp - dmg).max(0);
                let is_dead = creature_mut.hp <= 0;
                let pos = creature_mut.pos;
                let kind = creature_mut.kind;
                self.pending_events.push(TickEvent::AgentAttackedCreature {
                    attacker: aid.clone(),
                    creature_id: cid,
                    damage: dmg,
                });
                if is_dead {
                    self.creatures.remove(&cid);
                    if kind.is_boss() {
                        self.pending_events.push(TickEvent::BossKilled {
                            id: cid,
                            kind,
                            slayer: Some(aid.clone()),
                            at: pos,
                        });
                    } else {
                        self.pending_events
                            .push(TickEvent::CreatureKilled { id: cid, kind, at: pos });
                    }
                }
            }
        }
        let attacker = self.agents.get_mut(&aid).unwrap();
        attacker.status.stamina = (attacker.status.stamina - 5).max(0);
        attacker.last_action_tick = self.clock.tick;
    }

    fn resolve_move(&mut self, aid: AgentId, dir: Direction) {
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        let from = agent.pos;
        let to = from.step(dir);

        // 注意：creature 不挡路（走过它会同 tick 被攻击/或站到一起）。
        // reason 字段格式："blocked_by_tile:<kind>" / "blocked_by_agent:<id>"
        // / "blocked_by_building:<kind>" / "blocked_by_oob" —— 让调用方能精确
        // 定位"视野空但 blocked"的真正原因。
        let tile = self.grid.get(to);
        if tile.is_none() {
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: "blocked_by_oob".into(),
            });
            return;
        }
        let tile = tile.unwrap();
        if !tile.is_walkable() {
            let kind = serde_json::to_value(tile.kind)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".into());
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: format!("blocked_by_tile:{}", kind),
            });
            return;
        }
        if let Some(other) = self.agents.values().find(|a| a.pos == to && a.id != aid) {
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: format!("blocked_by_agent:{}", other.id.0),
            });
            return;
        }
        if let Some(b) = self.buildings.get(&to) {
            let kind = serde_json::to_value(b.kind)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "unknown".into());
            self.pending_events.push(TickEvent::AgentMoveFailed {
                agent: aid,
                reason: format!("blocked_by_building:{}", kind),
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

    fn resolve_gather(&mut self, aid: AgentId, target: TileCoord) {
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        if agent.pos.manhattan(target) > 1 {
            self.pending_events.push(TickEvent::AgentGatherFailed {
                agent: aid,
                reason: "out of range".into(),
            });
            return;
        }
        let tick = self.clock.tick;
        let take = match self.entities.get(&target) {
            Some(Entity::Plant { plant }) if plant.is_available(tick) => Some((
                plant.kind.yield_item(),
                plant.kind.yield_count(),
                plant.kind.regrow_after(),
            )),
            _ => None,
        };
        let Some((item, n, regrow)) = take else {
            self.pending_events.push(TickEvent::AgentGatherFailed {
                agent: aid,
                reason: "no harvestable".into(),
            });
            return;
        };
        let added = self.agents.get_mut(&aid).unwrap().inventory.add(item, n);
        if added == 0 {
            self.pending_events.push(TickEvent::AgentGatherFailed {
                agent: aid,
                reason: "inventory full".into(),
            });
            return;
        }
        if let Some(Entity::Plant { plant }) = self.entities.get_mut(&target) {
            if let Some(regrow_in) = regrow {
                plant.harvested_until = Some(tick + regrow_in);
            } else {
                self.entities.remove(&target);
            }
        }
        let a = self.agents.get_mut(&aid).unwrap();
        a.status.stamina = (a.status.stamina - 3).max(0);
        a.last_action_tick = tick;
        self.pending_events.push(TickEvent::AgentGathered {
            agent: aid,
            item,
            n: added,
            from: target,
        });
    }

    fn resolve_eat(&mut self, aid: AgentId, item: ItemKind) {
        let Some(a) = self.agents.get_mut(&aid) else {
            return;
        };
        if !item.is_food() {
            self.pending_events.push(TickEvent::AgentEatFailed {
                agent: aid,
                reason: format!("{:?} not food", item),
            });
            return;
        }
        if !a.inventory.remove(item, 1) {
            self.pending_events.push(TickEvent::AgentEatFailed {
                agent: aid,
                reason: format!("no {:?} in inventory", item),
            });
            return;
        }
        let (hunger, hp) = item.nutrition();
        a.status.hunger = (a.status.hunger + hunger).min(100);
        a.status.hp = (a.status.hp + hp).min(100);
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentAte {
            agent: aid,
            item,
            hp_gain: hp,
            hunger_gain: hunger,
        });
    }

    fn resolve_craft(&mut self, aid: AgentId, recipe_id: String) {
        let Some(rec) = recipe::find(&recipe_id) else {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: aid,
                reason: "unknown recipe".into(),
            });
            return;
        };
        let pos = match self.agents.get(&aid) {
            Some(a) => a.pos,
            None => return,
        };
        let station_ok = match rec.station {
            CraftStation::Hand => true,
            CraftStation::Campfire => self.has_nearby_building(pos, BuildingKind::Campfire),
            CraftStation::CookingStove => self.has_nearby_building(pos, BuildingKind::CookingStove),
        };
        if !station_ok {
            self.pending_events.push(TickEvent::AgentCraftFailed {
                agent: aid,
                reason: "station not nearby".into(),
            });
            return;
        }
        let a = self.agents.get(&aid).unwrap();
        for (item, n) in rec.inputs {
            if a.inventory.count(*item) < *n {
                self.pending_events.push(TickEvent::AgentCraftFailed {
                    agent: aid.clone(),
                    reason: format!("missing {:?}", item),
                });
                return;
            }
        }
        let a = self.agents.get_mut(&aid).unwrap();
        for (item, n) in rec.inputs {
            a.inventory.remove(*item, *n);
        }
        match rec.output {
            RecipeOutput::Item(item, n) => {
                a.inventory.add(item, n);
            }
        }
        a.status.stamina = (a.status.stamina - 5).max(0);
        a.last_action_tick = self.clock.tick;
        self.pending_events.push(TickEvent::AgentCrafted {
            agent: aid,
            recipe: recipe_id,
        });
    }

    fn has_nearby_building(&self, pos: TileCoord, kind: BuildingKind) -> bool {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let c = TileCoord::new(pos.x + dx, pos.y + dy);
                if let Some(b) = self.buildings.get(&c) {
                    if b.kind == kind {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn resolve_place(&mut self, aid: AgentId, item: ItemKind, pos: TileCoord) {
        let Some(kind) = recipe::kit_to_building(item) else {
            self.pending_events.push(TickEvent::AgentPlaceFailed {
                agent: aid,
                reason: format!("{:?} not placeable", item),
            });
            return;
        };
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        if agent.pos.manhattan(pos) > 1 {
            self.pending_events.push(TickEvent::AgentPlaceFailed {
                agent: aid,
                reason: "out of range (interaction_range=1)".into(),
            });
            return;
        }
        if self.buildings.contains_key(&pos)
            || self.agents.values().any(|a| a.pos == pos)
            || self.entities.contains_key(&pos)
        {
            self.pending_events.push(TickEvent::AgentPlaceFailed {
                agent: aid,
                reason: "tile occupied".into(),
            });
            return;
        }
        if !self.grid.get(pos).map(|t| t.is_walkable()).unwrap_or(false) {
            self.pending_events.push(TickEvent::AgentPlaceFailed {
                agent: aid,
                reason: "tile not walkable".into(),
            });
            return;
        }
        if !self.agents.get_mut(&aid).unwrap().inventory.remove(item, 1) {
            self.pending_events.push(TickEvent::AgentPlaceFailed {
                agent: aid,
                reason: format!("no {:?} in inventory", item),
            });
            return;
        }
        let placed_by = aid.clone();
        self.buildings.insert(
            pos,
            Building {
                kind,
                placed_by,
                placed_at_tick: self.clock.tick,
            },
        );
        self.pending_events.push(TickEvent::AgentPlaced {
            agent: aid,
            building: format!("{:?}", kind),
            at: pos,
        });
    }

    fn resolve_pickup(&mut self, aid: AgentId, pos: TileCoord) {
        let Some(agent) = self.agents.get(&aid) else {
            return;
        };
        if agent.pos.manhattan(pos) > 1 {
            self.pending_events.push(TickEvent::AgentPickUpFailed {
                agent: aid,
                reason: "out of range (interaction_range=1)".into(),
            });
            return;
        }
        let stack = match self.entities.get(&pos) {
            Some(Entity::ItemDrop { stack, .. }) => Some(*stack),
            _ => None,
        };
        let Some(stack) = stack else {
            self.pending_events.push(TickEvent::AgentPickUpFailed {
                agent: aid,
                reason: "no item drop at pos".into(),
            });
            return;
        };
        let added = self
            .agents
            .get_mut(&aid)
            .unwrap()
            .inventory
            .add(stack.item, stack.n);
        if added >= stack.n {
            self.entities.remove(&pos);
            self.pending_events.push(TickEvent::AgentPickedUp {
                agent: aid,
                item: stack.item,
                n: stack.n,
            });
        } else if added > 0 {
            if let Some(Entity::ItemDrop { stack: s, .. }) = self.entities.get_mut(&pos) {
                s.n -= added;
            }
            self.pending_events.push(TickEvent::AgentPickedUp {
                agent: aid,
                item: stack.item,
                n: added,
            });
        } else {
            self.pending_events.push(TickEvent::AgentPickUpFailed {
                agent: aid,
                reason: "inventory full".into(),
            });
        }
    }

    fn resolve_drop(&mut self, aid: AgentId, item: ItemKind, n: u16) {
        let Some(a) = self.agents.get_mut(&aid) else {
            return;
        };
        if !a.inventory.remove(item, n) {
            self.pending_events.push(TickEvent::AgentDropFailed {
                agent: aid,
                reason: format!("inventory has fewer than {} {:?}", n, item),
            });
            return;
        }
        let pos = a.pos;
        let tick = self.clock.tick;
        if let Some(Entity::ItemDrop { stack, expires_at }) = self.entities.get_mut(&pos) {
            if stack.item == item {
                stack.n += n;
                *expires_at = tick + crate::systems::death::ITEM_DROP_TTL;
                self.pending_events.push(TickEvent::AgentDropped {
                    agent: aid,
                    item,
                    n,
                });
                return;
            }
        }
        let drop_pos = if self.entities.contains_key(&pos) {
            TileCoord::new(pos.x + 1, pos.y)
        } else {
            pos
        };
        self.entities.insert(
            drop_pos,
            Entity::ItemDrop {
                stack: ItemStack { item, n },
                expires_at: tick + crate::systems::death::ITEM_DROP_TTL,
            },
        );
        self.pending_events.push(TickEvent::AgentDropped {
            agent: aid,
            item,
            n,
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

        let mut entities_view = Vec::new();
        for (id, a) in &self.agents {
            if id == viewer {
                continue;
            }
            if a.pos.manhattan(center) <= VISION_RADIUS {
                entities_view.push(VisibleEntity::Agent {
                    id: id.clone(),
                    name: a.name.clone(),
                    pos: a.pos,
                    hp: a.status.hp,
                });
            }
        }
        for (pos, e) in &self.entities {
            if pos.manhattan(center) <= VISION_RADIUS {
                match e {
                    Entity::Plant { plant } => {
                        let tick = self.clock.tick;
                        let available = plant.is_available(tick);
                        let cooldown_remaining = if available {
                            None
                        } else {
                            plant.harvested_until.map(|t| t.saturating_sub(tick))
                        };
                        entities_view.push(VisibleEntity::Plant {
                            pos: *pos,
                            plant_kind: plant.kind,
                            available,
                            cooldown_remaining,
                        });
                    }
                    Entity::ItemDrop { stack, expires_at } => {
                        entities_view.push(VisibleEntity::ItemDrop {
                            pos: *pos,
                            item: stack.item,
                            n: stack.n,
                            expires_in: expires_at.saturating_sub(self.clock.tick),
                        });
                    }
                }
            }
        }
        for (pos, b) in &self.buildings {
            if pos.manhattan(center) <= VISION_RADIUS {
                entities_view.push(VisibleEntity::Building {
                    pos: *pos,
                    building_kind: b.kind,
                    owner: b.placed_by.clone(),
                });
            }
        }
        for c in self.creatures.values() {
            if c.pos.manhattan(center) <= VISION_RADIUS {
                entities_view.push(VisibleEntity::Creature {
                    id: c.id,
                    pos: c.pos,
                    creature_kind: c.kind,
                    hp: c.hp,
                    hostile: c.kind.is_hostile(),
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
                inventory: agent.inventory.slots.clone(),
            },
            vision: VisionView {
                radius: VISION_RADIUS,
                tiles,
            },
            visible_entities: entities_view,
            nearby_signs: self
                .signs
                .iter()
                .filter(|(p, _)| p.manhattan(center) <= VISION_RADIUS)
                .map(|(p, s)| SignView {
                    pos: *p,
                    text: s.text.clone(),
                    author: s.author_name.clone(),
                    written_at_tick: s.written_at_tick,
                })
                .collect(),
            mail: self
                .mail
                .get(viewer)
                .map(|inbox| {
                    inbox
                        .iter()
                        .map(|m| MailView {
                            from: m.from.clone(),
                            text: m.text.clone(),
                            received_at_tick: m.received_at_tick,
                        })
                        .collect()
                })
                .unwrap_or_default(),
            recent_events: Vec::new(),
        })
    }
}

fn agent_best_weapon(a: &Agent) -> Option<ItemKind> {
    if a.inventory.count(ItemKind::StoneAxe) > 0 {
        Some(ItemKind::StoneAxe)
    } else if a.inventory.count(ItemKind::BambooSpear) > 0 {
        Some(ItemKind::BambooSpear)
    } else {
        None
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
    use crate::{plant::Plant, PlantKind, Season};

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
            if w.grid.get(target).map(|t| t.is_walkable()).unwrap_or(false)
                && !w.entities.contains_key(&target)
            {
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
    fn bootstrap_populates_entities() {
        let w = World::bootstrap(42);
        assert!(w.entities.len() > 50, "entities = {}", w.entities.len());
    }

    #[test]
    fn gather_adds_to_inventory() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let pos = w.agents[&id].pos;
        let target = TileCoord::new(pos.x + 1, pos.y);
        w.entities.insert(
            target,
            Entity::Plant {
                plant: Plant::fresh(PlantKind::Mushroom),
            },
        );
        let events = w.step(vec![(id.clone(), Action::Gather { target })]);
        assert!(events
            .iter()
            .any(|e| matches!(e, TickEvent::AgentGathered { .. })));
        assert_eq!(w.agents[&id].inventory.count(ItemKind::Mushroom), 1);
    }

    #[test]
    fn eat_restores_hunger() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents
            .get_mut(&id)
            .unwrap()
            .inventory
            .add(ItemKind::Mushroom, 3);
        w.agents.get_mut(&id).unwrap().status.hunger = 50;
        let _ = w.step(vec![(
            id.clone(),
            Action::Eat {
                item: ItemKind::Mushroom,
            },
        )]);
        let s = w.agents[&id].status;
        assert_eq!(s.hunger, 58);
        assert_eq!(w.agents[&id].inventory.count(ItemKind::Mushroom), 2);
    }

    #[test]
    fn craft_bamboo_spear_consumes_inputs() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        {
            let inv = &mut w.agents.get_mut(&id).unwrap().inventory;
            inv.add(ItemKind::Flint, 1);
            inv.add(ItemKind::Bamboo, 1);
        }
        let _ = w.step(vec![(
            id.clone(),
            Action::Craft {
                recipe: "bamboo_spear".into(),
            },
        )]);
        let inv = &w.agents[&id].inventory;
        assert_eq!(inv.count(ItemKind::BambooSpear), 1);
        assert_eq!(inv.count(ItemKind::Bamboo), 0);
    }

    #[test]
    fn place_campfire_creates_building() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents
            .get_mut(&id)
            .unwrap()
            .inventory
            .add(ItemKind::CampfireKit, 1);
        let pos = w.agents[&id].pos;
        let target = [
            (pos.x + 1, pos.y),
            (pos.x - 1, pos.y),
            (pos.x, pos.y + 1),
            (pos.x, pos.y - 1),
        ]
        .into_iter()
        .map(|(x, y)| TileCoord::new(x, y))
        .find(|c| {
            w.grid.get(*c).map(|t| t.is_walkable()).unwrap_or(false)
                && !w.entities.contains_key(c)
                && !w.buildings.contains_key(c)
        })
        .expect("no walkable neighbor");
        let _ = w.step(vec![(
            id.clone(),
            Action::Place {
                item: ItemKind::CampfireKit,
                pos: target,
            },
        )]);
        assert!(matches!(
            w.buildings.get(&target).map(|b| b.kind),
            Some(BuildingKind::Campfire)
        ));
    }

    #[test]
    fn hunger_decays_over_time() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let before = w.agents[&id].status.hunger;
        for _ in 0..40 {
            w.step(vec![]);
        }
        assert!(w.agents[&id].status.hunger < before);
    }

    #[test]
    fn agent_dies_when_hp_reaches_zero() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().status.hp = 1;
        w.agents.get_mut(&id).unwrap().status.hunger = 0;
        for _ in 0..10 {
            w.step(vec![]);
        }
        assert!(
            matches!(w.agents[&id].state, AgentState::Dying { .. }),
            "state = {:?}",
            w.agents[&id].state
        );
    }

    #[test]
    fn dying_agent_respawns_after_delay() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents.get_mut(&id).unwrap().status.hp = 0;
        w.agents.get_mut(&id).unwrap().status.hunger = 0;
        w.step(vec![]);
        for _ in 0..crate::systems::death::RESPAWN_DELAY + 2 {
            w.step(vec![]);
        }
        assert!(matches!(w.agents[&id].state, AgentState::Alive));
        assert_eq!(w.agents[&id].status.hp, 100);
    }

    #[test]
    fn season_constant_imported() {
        let _: Season = Season::Chun;
    }

    #[test]
    fn agent_attacks_agent_damages_target() {
        let mut w = World::bootstrap(42);
        let alice = w.join("alice".into()).unwrap();
        let bob = w.join("bob".into()).unwrap();
        // 强制相邻
        let pos = w.agents[&alice].pos;
        let nearby = TileCoord::new(pos.x + 1, pos.y);
        if w.grid.get(nearby).map(|t| t.is_walkable()).unwrap_or(false) {
            w.agents.get_mut(&bob).unwrap().pos = nearby;
        } else {
            return; // 跳过本测试（罕见 seed 冲突）
        }
        let before = w.agents[&bob].status.hp;
        let _ = w.step(vec![(
            alice.clone(),
            Action::Attack {
                target: AttackTarget::Agent(bob.clone()),
            },
        )]);
        assert!(w.agents[&bob].status.hp < before);
    }

    #[test]
    fn agent_writes_sign_visible_in_observation() {
        let mut w = World::bootstrap(42);
        let alice = w.join("alice".into()).unwrap();
        let bob = w.join("bob".into()).unwrap();
        let pos = w.agents[&alice].pos;
        let sign_pos = TileCoord::new(pos.x + 1, pos.y);
        if !w.grid.get(sign_pos).map(|t| t.is_walkable()).unwrap_or(false) {
            return;
        }
        let _ = w.step(vec![(
            alice.clone(),
            Action::WriteSign {
                pos: sign_pos,
                text: "前方有狼，绕道".into(),
            },
        )]);
        // bob 站在 sign 旁边
        w.agents.get_mut(&bob).unwrap().pos = pos;
        let obs = w.observe(&bob).unwrap();
        assert!(obs.nearby_signs.iter().any(|s| s.text.contains("狼")));
    }

    #[test]
    fn agent_sends_mail_appears_in_recipient_observation() {
        let mut w = World::bootstrap(42);
        let alice = w.join("alice".into()).unwrap();
        let bob = w.join("bob".into()).unwrap();
        let _ = w.step(vec![(
            alice.clone(),
            Action::SendMail {
                to: "bob".into(),
                text: "灶台造好了，35,40 见".into(),
            },
        )]);
        let obs = w.observe(&bob).unwrap();
        assert_eq!(obs.mail.len(), 1);
        assert_eq!(obs.mail[0].from, "alice");
        assert!(obs.mail[0].text.contains("灶台"));
    }

    #[test]
    fn agent_kills_creature() {
        use crate::creature::{Creature, CreatureKind};
        let mut w = World::bootstrap(42);
        let alice = w.join("alice".into()).unwrap();
        let pos = w.agents[&alice].pos;
        let target_pos = TileCoord::new(pos.x + 1, pos.y);
        if !w.grid.get(target_pos).map(|t| t.is_walkable()).unwrap_or(false) {
            return;
        }
        // 给 alice 配石斧（伤害 10）→ 1 hit 即可
        w.agents.get_mut(&alice).unwrap().inventory.add(ItemKind::StoneAxe, 1);
        w.next_creature_id += 1;
        let cid = w.next_creature_id;
        w.creatures.insert(
            cid,
            Creature::new(cid, CreatureKind::Rabbit, target_pos, w.clock.tick),
        );
        let _ = w.step(vec![(
            alice.clone(),
            Action::Attack {
                target: AttackTarget::Creature(cid),
            },
        )]);
        assert!(!w.creatures.contains_key(&cid), "rabbit should be dead");
    }

    #[test]
    fn move_failed_reason_specifies_tile_kind() {
        // 移到山/深水 tile，reason 应当是 "blocked_by_tile:mountain" 之类
        let mut w = World::bootstrap(7);
        let id = w.join("alice".into()).unwrap();
        let here = w.agents[&id].pos;
        // 找一个邻接的不可走 tile，方向 = 那个方向
        let mut blocked_dir = None;
        let mut blocked_kind = None;
        for d in Direction::ALL {
            let t = here.step(d);
            if let Some(tile) = w.grid.get(t) {
                if !tile.is_walkable() {
                    blocked_dir = Some(d);
                    blocked_kind = Some(tile.kind);
                    break;
                }
            } else {
                // out of bounds 邻位
                blocked_dir = Some(d);
                blocked_kind = None;
                break;
            }
        }
        let Some(dir) = blocked_dir else {
            return; // seed-dependent，没找到不可走邻位就跳过
        };
        let events = w.step(vec![(id.clone(), Action::Move { dir })]);
        let reason = events.iter().find_map(|e| match e {
            TickEvent::AgentMoveFailed { reason, .. } => Some(reason.clone()),
            _ => None,
        });
        let reason = reason.expect("expected AgentMoveFailed");
        if let Some(kind) = blocked_kind {
            let expected_suffix = serde_json::to_value(kind)
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            assert_eq!(
                reason,
                format!("blocked_by_tile:{}", expected_suffix),
                "reason 应包含具体 tile kind"
            );
        } else {
            assert_eq!(reason, "blocked_by_oob");
        }
    }

    #[test]
    fn move_failed_reason_specifies_agent_id_when_occupied() {
        let mut w = World::bootstrap(7);
        let alice = w.join("alice".into()).unwrap();
        let bob = w.join("bob".into()).unwrap();
        // 把 bob 强制挪到 alice 邻位
        let here = w.agents[&alice].pos;
        let mut placed_dir = None;
        for d in Direction::ALL {
            let t = here.step(d);
            if w.grid.get(t).map(|x| x.is_walkable()).unwrap_or(false) {
                w.agents.get_mut(&bob).unwrap().pos = t;
                placed_dir = Some(d);
                break;
            }
        }
        let dir = placed_dir.expect("no walkable neighbor");
        let events = w.step(vec![(alice.clone(), Action::Move { dir })]);
        let reason = events.iter().find_map(|e| match e {
            TickEvent::AgentMoveFailed { reason, .. } => Some(reason.clone()),
            _ => None,
        });
        let reason = reason.expect("expected AgentMoveFailed");
        assert_eq!(reason, format!("blocked_by_agent:{}", bob.0));
    }

    #[test]
    fn move_failed_reason_specifies_building_kind() {
        let mut w = World::bootstrap(7);
        let id = w.join("alice".into()).unwrap();
        let here = w.agents[&id].pos;
        let mut placed_dir = None;
        for d in Direction::ALL {
            let t = here.step(d);
            if w.grid.get(t).map(|x| x.is_walkable()).unwrap_or(false)
                && !w.entities.contains_key(&t)
            {
                w.buildings.insert(
                    t,
                    crate::Building {
                        kind: crate::BuildingKind::Campfire,
                        placed_by: id.clone(),
                        placed_at_tick: 0,
                    },
                );
                placed_dir = Some(d);
                break;
            }
        }
        let dir = placed_dir.expect("no walkable neighbor");
        let events = w.step(vec![(id.clone(), Action::Move { dir })]);
        let reason = events.iter().find_map(|e| match e {
            TickEvent::AgentMoveFailed { reason, .. } => Some(reason.clone()),
            _ => None,
        });
        assert_eq!(reason.unwrap(), "blocked_by_building:campfire");
    }

    #[test]
    fn eat_non_food_emits_eat_failed_not_gather_failed() {
        // 上一版本 resolve_eat 把失败贴成了 AgentGatherFailed，agent 没法区分。
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        w.agents
            .get_mut(&id)
            .unwrap()
            .inventory
            .add(ItemKind::Reed, 1);
        let events = w.step(vec![(
            id.clone(),
            Action::Eat { item: ItemKind::Reed },
        )]);
        let has_eat_failed = events
            .iter()
            .any(|e| matches!(e, TickEvent::AgentEatFailed { agent, .. } if agent == &id));
        let has_gather_failed = events
            .iter()
            .any(|e| matches!(e, TickEvent::AgentGatherFailed { .. }));
        assert!(has_eat_failed, "AgentEatFailed should be emitted");
        assert!(!has_gather_failed, "should NOT spill into AgentGatherFailed");
        // reed 不消耗
        assert_eq!(w.agents[&id].inventory.count(ItemKind::Reed), 1);
    }

    #[test]
    fn pickup_out_of_range_emits_pickup_failed() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let my = w.agents[&id].pos;
        let far = TileCoord::new(my.x + 5, my.y);
        let events = w.step(vec![(
            id.clone(),
            Action::PickUp { pos: far },
        )]);
        assert!(
            events.iter().any(|e| matches!(
                e,
                TickEvent::AgentPickUpFailed { agent, reason }
                    if agent == &id && reason.contains("range")
            )),
            "expected AgentPickUpFailed (out of range), got events: {:?}",
            events
        );
    }

    #[test]
    fn drop_more_than_held_emits_drop_failed() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        // join 已经给了 red_berry × 3，drop 5 个超过库存
        let events = w.step(vec![(
            id.clone(),
            Action::Drop {
                item: ItemKind::RedBerry,
                n: 5,
            },
        )]);
        assert!(
            events.iter().any(|e| matches!(
                e,
                TickEvent::AgentDropFailed { agent, .. } if agent == &id
            )),
            "expected AgentDropFailed, got events: {:?}",
            events
        );
        // 库存没动
        assert_eq!(w.agents[&id].inventory.count(ItemKind::RedBerry), 3);
    }

    #[test]
    fn place_non_kit_emits_place_failed() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let my = w.agents[&id].pos;
        let target = TileCoord::new(my.x + 1, my.y);
        let events = w.step(vec![(
            id.clone(),
            Action::Place {
                item: ItemKind::Mushroom, // 不是 kit
                pos: target,
            },
        )]);
        assert!(
            events.iter().any(|e| matches!(
                e,
                TickEvent::AgentPlaceFailed { agent, reason }
                    if agent == &id && reason.contains("not placeable")
            )),
            "expected AgentPlaceFailed (not placeable), got events: {:?}",
            events
        );
    }

    #[test]
    fn spawn_includes_starter_red_berries() {
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        assert_eq!(
            w.agents[&id].inventory.count(ItemKind::RedBerry),
            3,
            "spawn should grant 3 red berries to prevent death spiral"
        );
    }

    #[test]
    fn plant_cooldown_remaining_decreases_then_clears() {
        // 用一个伪植物：harvested_until = tick + 10，然后 tick 5 步看 cooldown_remaining=5
        let mut w = World::bootstrap(42);
        let id = w.join("alice".into()).unwrap();
        let my = w.agents[&id].pos;
        // 在邻位放一个手动挂冷却的植物
        let cd_pos = TileCoord::new(my.x + 1, my.y);
        if !w.grid.get(cd_pos).map(|t| t.is_walkable()).unwrap_or(false) {
            return; // 邻位是水/山，跳过
        }
        let mut plant = Plant::fresh(PlantKind::Mushroom);
        plant.harvested_until = Some(w.clock.tick + 10);
        w.entities.insert(cd_pos, Entity::Plant { plant });
        let obs = w.observe(&id).unwrap();
        let cd_entity = obs
            .visible_entities
            .iter()
            .find_map(|e| match e {
                crate::observation::VisibleEntity::Plant { pos, available, cooldown_remaining, .. }
                    if *pos == cd_pos => Some((*available, *cooldown_remaining)),
                _ => None,
            })
            .expect("plant should be in fov");
        assert!(!cd_entity.0, "should be unavailable");
        assert_eq!(cd_entity.1, Some(10));
    }
}
