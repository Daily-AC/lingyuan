mod auth;
mod config;
mod db;
mod persistence;
mod routes;
mod state;
mod tick_loop;

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,server=debug".into()),
        )
        .init();

    let cfg = config::ServerConfig::from_env();
    if let Some(parent) = cfg.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = db::Db::open(&cfg.db_path).await?;
    db.migrate().await?;

    let world = db.load_or_bootstrap(cfg.world_seed).await?;
    info!(tick = world.clock.tick, agents = world.agent_count(), "world loaded");
    let world = Arc::new(Mutex::new(world));

    let (actions_tx, actions_rx) = mpsc::channel(1024);
    let (frames_tx, _) = broadcast::channel(64);
    let (db_tx, db_rx) = mpsc::channel(256);

    let state = state::AppState {
        world: world.clone(),
        actions_tx,
        frames_tx: frames_tx.clone(),
        db_tx,
        config: cfg.clone(),
        tick_ms: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(cfg.tick_ms)),
    };

    tokio::spawn(db::writer_task(db.clone(), db_rx));
    tokio::spawn(tick_loop::run(state.clone(), actions_rx));

    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(&cfg.bind_addr).await?;
    info!(addr = %cfg.bind_addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
