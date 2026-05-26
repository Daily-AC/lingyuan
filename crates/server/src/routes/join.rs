use crate::{
    auth::{hash_token, new_token},
    db::DbWrite,
    state::AppState,
};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct JoinReq {
    pub name: String,
}

#[derive(Serialize)]
pub struct JoinResp {
    pub agent_id: String,
    pub token: String,
    pub spawn_at: world::TileCoord,
    pub tick: u64,
}

pub async fn join(
    State(s): State<AppState>,
    Json(req): Json<JoinReq>,
) -> Result<Json<JoinResp>, (StatusCode, String)> {
    if req.name.is_empty() || req.name.len() > 32 {
        return Err((StatusCode::BAD_REQUEST, "name must be 1..=32 chars".into()));
    }
    let (agent_id, pos, tick) = {
        let mut w = s.world.lock().await;
        let id = w
            .join(req.name.clone())
            .map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
        let pos = w.agents[&id].pos;
        let tick = w.clock.tick;
        (id, pos, tick)
    };

    let token = new_token();
    let _ = s
        .db_tx
        .send(DbWrite::UpsertAgentMeta {
            agent_id: agent_id.0.clone(),
            name: req.name,
            token_hash: hash_token(&token),
            joined_at: chrono::Utc::now().timestamp(),
        })
        .await;

    Ok(Json(JoinResp {
        agent_id: agent_id.0,
        token,
        spawn_at: pos,
        tick,
    }))
}
