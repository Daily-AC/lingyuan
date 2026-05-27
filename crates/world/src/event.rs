use crate::{
    agent::AgentId, clock::Season, coord::TileCoord, creature::CreatureKind, item::ItemKind,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum TickEvent {
    AgentJoined {
        agent: AgentId,
        name: String,
        at: TileCoord,
    },
    AgentLeft {
        agent: AgentId,
        name: String,
    },
    AgentMoved {
        agent: AgentId,
        from: TileCoord,
        to: TileCoord,
    },
    AgentMoveFailed {
        agent: AgentId,
        reason: String,
    },
    AgentGathered {
        agent: AgentId,
        item: ItemKind,
        n: u16,
        from: TileCoord,
    },
    AgentGatherFailed {
        agent: AgentId,
        reason: String,
    },
    AgentAte {
        agent: AgentId,
        item: ItemKind,
        hp_gain: i16,
        hunger_gain: i16,
    },
    AgentEatFailed {
        agent: AgentId,
        reason: String,
    },
    AgentCrafted {
        agent: AgentId,
        recipe: String,
    },
    AgentCraftFailed {
        agent: AgentId,
        reason: String,
    },
    AgentPlaced {
        agent: AgentId,
        building: String,
        at: TileCoord,
    },
    AgentPlaceFailed {
        agent: AgentId,
        reason: String,
    },
    AgentPickedUp {
        agent: AgentId,
        item: ItemKind,
        n: u16,
    },
    AgentPickUpFailed {
        agent: AgentId,
        reason: String,
    },
    AgentDropped {
        agent: AgentId,
        item: ItemKind,
        n: u16,
    },
    AgentDropFailed {
        agent: AgentId,
        reason: String,
    },
    AgentDied {
        agent: AgentId,
        at: TileCoord,
        cause: String,
    },
    AgentRespawned {
        agent: AgentId,
        at: TileCoord,
    },
    AgentAttackedAgent {
        attacker: AgentId,
        target: AgentId,
        damage: i16,
        weapon: Option<String>,
    },
    AgentAttackedCreature {
        attacker: AgentId,
        creature_id: u64,
        damage: i16,
    },
    AgentAttackFailed {
        agent: AgentId,
        reason: String,
    },
    CreatureSpawned {
        id: u64,
        kind: CreatureKind,
        at: TileCoord,
    },
    BossSpawned {
        id: u64,
        kind: CreatureKind,
        at: TileCoord,
        announcement: String,
    },
    BossKilled {
        id: u64,
        kind: CreatureKind,
        slayer: Option<AgentId>,
        at: TileCoord,
    },
    CreatureKilled {
        id: u64,
        kind: CreatureKind,
        at: TileCoord,
    },
    CreatureAttackedAgent {
        creature_id: u64,
        creature_kind: CreatureKind,
        target: AgentId,
        damage: i16,
    },
    AgentWroteSign {
        agent: AgentId,
        pos: TileCoord,
        text_excerpt: String,
    },
    AgentSentMail {
        from: AgentId,
        to: String,
        text_excerpt: String,
    },
    SeasonChanged {
        to: Season,
    },
    DayStarted {
        day: u32,
    },
    NightStarted {
        day: u32,
    },
}
