use crate::{
    auth::AuthAgent,
    state::{ActionEnvelope, AppState},
};
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ActResp {
    pub accepted: bool,
    pub queued_for_tick: u64,
}

pub async fn act(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
    Json(action): Json<world::Action>,
) -> Result<Json<ActResp>, (StatusCode, String)> {
    let tick = { s.world.lock().await.clock.tick + 1 };
    s.actions_tx
        .send(ActionEnvelope {
            agent: agent_id,
            action,
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(ActResp {
        accepted: true,
        queued_for_tick: tick,
    }))
}
