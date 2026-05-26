use crate::{auth::AuthAgent, state::AppState};
use axum::{extract::State, http::StatusCode};

pub async fn leave(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
) -> Result<(), (StatusCode, String)> {
    let mut w = s.world.lock().await;
    w.leave(&agent_id)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(())
}
