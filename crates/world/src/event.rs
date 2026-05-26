use crate::{agent::AgentId, clock::Season, coord::TileCoord, item::ItemKind};
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
    AgentPickedUp {
        agent: AgentId,
        item: ItemKind,
        n: u16,
    },
    AgentDropped {
        agent: AgentId,
        item: ItemKind,
        n: u16,
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
