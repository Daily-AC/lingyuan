use crate::coord::TileCoord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub hp: i16,
    pub hunger: i16,
    pub stamina: i16,
    pub warmth: i16,
    pub sanity: i16,
}

impl AgentStatus {
    pub fn fresh() -> Self {
        Self {
            hp: 100,
            hunger: 100,
            stamina: 100,
            warmth: 0,
            sanity: 100,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum AgentState {
    Alive,
    Dying { revives_at_tick: u64 },
    Meditating { since_tick: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub pos: TileCoord,
    pub status: AgentStatus,
    pub state: AgentState,
    pub last_action_tick: u64,
    pub joined_tick: u64,
}

impl Agent {
    pub fn new_at(id: AgentId, name: String, pos: TileCoord, tick: u64) -> Self {
        Self {
            id,
            name,
            pos,
            status: AgentStatus::fresh(),
            state: AgentState::Alive,
            last_action_tick: tick,
            joined_tick: tick,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_status_full() {
        let s = AgentStatus::fresh();
        assert_eq!(s.hp, 100);
        assert_eq!(s.hunger, 100);
    }
}
