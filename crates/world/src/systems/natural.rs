use crate::{Agent, AgentId, AgentState};
use std::collections::BTreeMap;

/// 每 tick 对 alive agent：
/// - hunger 每 4 tick -1
/// - stamina 每 8 tick +2（上限 100）
/// - hunger==0 时每 2 tick hp -1
pub fn step_status(tick: u64, agents: &mut BTreeMap<AgentId, Agent>) {
    for a in agents.values_mut() {
        if !matches!(a.state, AgentState::Alive) {
            continue;
        }
        if tick > 0 && tick % 4 == 0 {
            a.status.hunger = (a.status.hunger - 1).max(0);
        }
        if tick > 0 && tick % 8 == 0 {
            a.status.stamina = (a.status.stamina + 2).min(100);
        }
        if a.status.hunger == 0 && tick > 0 && tick % 2 == 0 {
            a.status.hp = (a.status.hp - 1).max(0);
        }
    }
}
