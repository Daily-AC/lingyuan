use crate::persistence::{deserialize_world, serialize_world};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tokio::sync::mpsc;
use tracing::{error, info};
use world::World;

#[derive(Clone)]
pub struct Db {
    pub pool: SqlitePool,
}

#[derive(Debug)]
pub enum DbWrite {
    Frame(crate::state::TickFrame),
    Snapshot(World),
    UpsertAgentMeta {
        agent_id: String,
        name: String,
        token_hash: String,
        joined_at: i64,
    },
}

const MIGRATION_SQL: &str = include_str!("../migrations/0001_initial.sql");

impl Db {
    pub async fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(opts)
            .await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        for stmt in MIGRATION_SQL.split(';') {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            sqlx::query(s).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn load_or_bootstrap(&self, seed: u64) -> anyhow::Result<World> {
        let row: Option<(i64, Vec<u8>)> =
            sqlx::query_as("SELECT tick, bin FROM snapshots ORDER BY tick DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        let Some((snap_tick, bin)) = row else {
            info!(seed, "no snapshot, bootstrapping world");
            return Ok(World::bootstrap(seed));
        };
        let mut world = deserialize_world(&bin)?;
        info!(snap_tick, "loaded snapshot");

        let evt_rows: Vec<(i64, i64, String)> = sqlx::query_as(
            "SELECT tick, seq, event_json FROM events WHERE tick > ? ORDER BY tick ASC, seq ASC",
        )
        .bind(snap_tick)
        .fetch_all(&self.pool)
        .await?;

        if let Some((max_tick, _, _)) = evt_rows.last() {
            let needed = (*max_tick as u64).saturating_sub(world.clock.tick);
            for _ in 0..needed {
                world.step(vec![]);
            }
            info!(target_tick = max_tick, "replayed clock to match event log tail");
        }
        Ok(world)
    }
}

pub async fn writer_task(db: Db, mut rx: mpsc::Receiver<DbWrite>) {
    while let Some(w) = rx.recv().await {
        if let Err(e) = handle(&db, w).await {
            error!(error = %e, "db write failed");
        }
    }
}

async fn handle(db: &Db, w: DbWrite) -> anyhow::Result<()> {
    match w {
        DbWrite::Frame(frame) => {
            let mut tx = db.pool.begin().await?;
            for (seq, evt) in frame.events.iter().enumerate() {
                let json = serde_json::to_string(evt)?;
                sqlx::query("INSERT INTO events(tick, seq, event_json) VALUES(?, ?, ?)")
                    .bind(frame.tick as i64)
                    .bind(seq as i64)
                    .bind(json)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
        }
        DbWrite::Snapshot(world) => {
            let bin = serialize_world(&world)?;
            let now = chrono::Utc::now().timestamp();
            sqlx::query(
                "INSERT OR REPLACE INTO snapshots(tick, bin, created_at) VALUES(?, ?, ?)",
            )
            .bind(world.clock.tick as i64)
            .bind(bin)
            .bind(now)
            .execute(&db.pool)
            .await?;
        }
        DbWrite::UpsertAgentMeta {
            agent_id,
            name,
            token_hash,
            joined_at,
        } => {
            sqlx::query(
                "INSERT INTO agents_meta(agent_id, name, token_hash, joined_at, total_lives) VALUES(?, ?, ?, ?, 0)
                 ON CONFLICT(agent_id) DO UPDATE SET name=excluded.name, token_hash=excluded.token_hash"
            )
            .bind(agent_id).bind(name).bind(token_hash).bind(joined_at)
            .execute(&db.pool).await?;
        }
    }
    Ok(())
}
