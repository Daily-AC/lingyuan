use crate::state::AppState;
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;

#[derive(Deserialize)]
pub struct TickMsReq {
    pub ms: u64,
}

#[derive(Serialize)]
pub struct TickMsResp {
    pub ok: bool,
    pub tick_ms: u64,
}

pub async fn set_tick_ms(
    State(s): State<AppState>,
    Json(req): Json<TickMsReq>,
) -> Result<Json<TickMsResp>, (StatusCode, String)> {
    if req.ms < 50 || req.ms > 5000 {
        return Err((StatusCode::BAD_REQUEST, "ms 必须在 50..=5000".into()));
    }
    s.tick_ms.store(req.ms, Ordering::Relaxed);
    Ok(Json(TickMsResp {
        ok: true,
        tick_ms: req.ms,
    }))
}

pub async fn get_tick_ms(State(s): State<AppState>) -> Json<TickMsResp> {
    Json(TickMsResp {
        ok: true,
        tick_ms: s.tick_ms.load(Ordering::Relaxed),
    })
}
