use crate::{auth::AuthAgent, state::AppState};
use axum::{extract::State, http::StatusCode, Json};

pub async fn observe(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
) -> Result<Json<world::Observation>, (StatusCode, &'static str)> {
    let w = s.world.lock().await;
    let mut obs = w
        .observe(&agent_id)
        .ok_or((StatusCode::NOT_FOUND, "agent not found"))?;
    drop(w);

    let by_agent = s.recent_events_by_agent.lock().await;
    if let Some(evts) = by_agent.get(&agent_id) {
        obs.recent_events = evts.clone();
    }
    Ok(Json(obs))
}
