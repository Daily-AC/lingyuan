use crate::state::AppState;
use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;

mod act;
mod clock;
mod health;
mod join;
mod leave;
mod observe;
pub mod ws;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health::health))
        .route("/api/v1/world/clock", get(clock::clock))
        .route("/api/v1/join", post(join::join))
        .route("/api/v1/observe", get(observe::observe))
        .route("/api/v1/act", post(act::act))
        .route("/api/v1/leave", post(leave::leave))
        .route("/ws/spectator", get(ws::spectator_ws))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
