use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::{broadcast, mpsc, Mutex};
use world::{Action, AgentId, Observation, World};

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<Mutex<World>>,
    pub actions_tx: mpsc::Sender<ActionEnvelope>,
    pub frames_tx: broadcast::Sender<TickFrame>,
    pub db_tx: mpsc::Sender<crate::db::DbWrite>,
    pub config: crate::config::ServerConfig,
    pub tick_ms: Arc<AtomicU64>,
}

#[derive(Debug, Clone)]
pub struct ActionEnvelope {
    pub agent: AgentId,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct TickFrame {
    pub tick: u64,
    pub clock: world::WorldClock,
    pub events: Vec<world::TickEvent>,
    pub spectator_view: SpectatorView,
    pub observations: std::collections::HashMap<AgentId, Observation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorView {
    pub tick: u64,
    pub clock: world::WorldClock,
    pub agents: Vec<SpectatorAgent>,
    pub entities: Vec<SpectatorEntity>,
    pub events: Vec<world::TickEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorAgent {
    pub id: AgentId,
    pub name: String,
    pub pos: world::TileCoord,
    pub hp: i16,
    pub hunger: i16,
    pub stamina: i16,
    pub state: String,
    pub inventory: Vec<world::ItemStack>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorEntity {
    pub pos: world::TileCoord,
    /// 格式: "plant:mushroom" / "drop:stone" / "building:campfire" / "creature:wolf"
    pub kind: String,
    pub label: Option<String>,
    /// 实体 ID（创造物）— 用于伤害浮字定位
    pub id: Option<u64>,
}
