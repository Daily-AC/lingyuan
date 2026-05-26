use crate::{agent::AgentId, clock::Season, coord::TileCoord};
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
