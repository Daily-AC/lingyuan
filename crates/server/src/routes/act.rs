use crate::{
    auth::AuthAgent,
    state::{AppState, PendingAction},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ActResp {
    pub accepted: bool,
    pub accepted_at_tick: u64,
    pub will_resolve_at_tick: u64,
    pub queue_depth: usize,
}

#[derive(Serialize)]
pub struct ActConflict {
    pub accepted: bool,
    pub reason: &'static str,
    pub existing_action: world::Action,
    pub will_resolve_at_tick: u64,
}

pub async fn act(
    State(s): State<AppState>,
    AuthAgent { agent_id }: AuthAgent,
    Json(action): Json<world::Action>,
) -> Response {
    let cur_tick = { s.world.lock().await.clock.tick };
    let will_resolve = cur_tick + 1;

    let mut pending = s.pending.lock().await;
    if let Some(existing) = pending.get(&agent_id) {
        // 重复入队：拒绝并告诉调用方既存动作和落地 tick。
        let body = ActConflict {
            accepted: false,
            reason: "already_queued",
            existing_action: existing.action.clone(),
            will_resolve_at_tick: existing.will_resolve_at_tick,
        };
        return (StatusCode::CONFLICT, Json(body)).into_response();
    }
    pending.insert(
        agent_id,
        PendingAction {
            action,
            will_resolve_at_tick: will_resolve,
        },
    );
    let queue_depth = pending.len();
    drop(pending);

    Json(ActResp {
        accepted: true,
        accepted_at_tick: cur_tick,
        will_resolve_at_tick: will_resolve,
        queue_depth,
    })
    .into_response()
}
