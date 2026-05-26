use crate::state::AppState;
use axum::{extract::State, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct ClockResp {
    pub tick: u64,
    pub day: u32,
    pub season: world::Season,
    pub phase: world::DayPhase,
    pub tick_in_day: u32,
}

pub async fn clock(State(s): State<AppState>) -> Json<ClockResp> {
    let w = s.world.lock().await;
    Json(ClockResp {
        tick: w.clock.tick,
        day: w.clock.tick as u32 / world::clock::TICKS_PER_DAY,
        season: w.clock.season(),
        phase: w.clock.phase(),
        tick_in_day: w.clock.tick_in_day(),
    })
}
