use crate::AgentId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildingKind {
    Campfire,
    CookingStove,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub kind: BuildingKind,
    pub placed_by: AgentId,
    pub placed_at_tick: u64,
}
