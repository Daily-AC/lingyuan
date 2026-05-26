use crate::{
    agent::{AgentId, AgentState, AgentStatus},
    clock::{DayPhase, Season},
    coord::TileCoord,
    event::TickEvent,
    tile::Tile,
};
use serde::{Deserialize, Serialize};

pub const VISION_RADIUS: u16 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub tick: u64,
    pub clock: ClockView,
    #[serde(rename = "self")]
    pub self_: SelfView,
    pub vision: VisionView,
    pub visible_entities: Vec<VisibleEntity>,
    pub recent_events: Vec<TickEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockView {
    pub day: u32,
    pub season: Season,
    pub phase: DayPhase,
    pub tick_in_day: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfView {
    pub id: AgentId,
    pub name: String,
    pub pos: TileCoord,
    pub status: AgentStatus,
    pub state: AgentState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionView {
    pub radius: u16,
    pub tiles: Vec<VisibleTile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisibleTile {
    pub pos: TileCoord,
    pub tile: Tile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisibleEntity {
    Agent {
        id: AgentId,
        name: String,
        pos: TileCoord,
        hp: i16,
    },
}
