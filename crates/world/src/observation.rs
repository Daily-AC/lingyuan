use crate::{
    agent::{AgentId, AgentState, AgentStatus},
    building::BuildingKind,
    clock::{DayPhase, Season},
    coord::TileCoord,
    creature::{CreatureId, CreatureKind},
    event::TickEvent,
    item::{ItemKind, ItemStack},
    plant::PlantKind,
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
    pub nearby_signs: Vec<SignView>,
    pub mail: Vec<MailView>,
    pub recent_events: Vec<TickEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignView {
    pub pos: TileCoord,
    pub text: String,
    pub author: Option<String>,
    pub written_at_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailView {
    pub from: String,
    pub text: String,
    pub received_at_tick: u64,
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
    pub inventory: Vec<ItemStack>,
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
    Plant {
        pos: TileCoord,
        #[serde(rename = "species")]
        plant_kind: PlantKind,
        available: bool,
        /// 不可采时距离重新可采的剩余 tick；可采时为 None。
        /// agent 用它来判断"该等还是该走"。
        cooldown_remaining: Option<u64>,
    },
    ItemDrop {
        pos: TileCoord,
        item: ItemKind,
        n: u16,
        expires_in: u64,
    },
    Building {
        pos: TileCoord,
        #[serde(rename = "subkind")]
        building_kind: BuildingKind,
        owner: AgentId,
    },
    Creature {
        id: CreatureId,
        pos: TileCoord,
        #[serde(rename = "species")]
        creature_kind: CreatureKind,
        hp: i16,
        hostile: bool,
    },
}
