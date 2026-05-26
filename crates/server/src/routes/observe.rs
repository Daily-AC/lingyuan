use crate::{auth::AuthAgent, state::AppState};
use axum::{extract::State, http::StatusCode, Json};

pub async fn observe(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
) -> Result<Json<world::Observation>, (StatusCode, &'static str)> {
    let w = s.world.lock().await;
    w.observe(&agent_id)
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "agent not found"))
}
