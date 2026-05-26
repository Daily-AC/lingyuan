use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub db_path: PathBuf,
    pub tick_ms: u64,
    pub world_seed: u64,
    pub snapshot_every: u64,
}

impl ServerConfig {
    pub fn from_env() -> Self {
        Self {
            bind_addr: std::env::var("LINGYUAN_BIND").unwrap_or_else(|_| "127.0.0.1:7777".into()),
            db_path: std::env::var("LINGYUAN_DB")
                .map(PathBuf::from)
                .unwrap_or_else(|_| "data/world.db".into()),
            tick_ms: std::env::var("LINGYUAN_TICK_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(2000),
            world_seed: std::env::var("LINGYUAN_SEED")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(42),
            snapshot_every: std::env::var("LINGYUAN_SNAPSHOT_EVERY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
        }
    }
}
