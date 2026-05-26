use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use world::{Action, AgentId, Observation, World};

#[derive(Clone)]
pub struct AppState {
    pub world: Arc<Mutex<World>>,
    pub actions_tx: mpsc::Sender<ActionEnvelope>,
    pub frames_tx: broadcast::Sender<TickFrame>,
    pub db_tx: mpsc::Sender<crate::db::DbWrite>,
    pub config: crate::config::ServerConfig,
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
    pub events: Vec<world::TickEvent>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectatorAgent {
    pub id: AgentId,
    pub name: String,
    pub pos: world::TileCoord,
    pub hp: i16,
}
