//! 简单规则 NPC：observe → decide → act 循环。
//! 决策优先级：
//!   1. 受击 / 看到 hostile 在相邻 → 反击（用最佳武器）
//!   2. hunger < 35 且 inventory 有食物 → eat
//!   3. stamina < 12 → wait
//!   4. 看到可采 plant 在相邻 → gather
//!   5. 看到可采 plant 在视野内 → 向它走一步
//!   6. else 随机方向走

use anyhow::Result;
use rand::seq::SliceRandom;
use rand::Rng;
use std::time::Duration;

use crate::client::Client;

const DIRS: [&str; 4] = ["north", "south", "east", "west"];

pub async fn run(client: Client, period_ms: u64, verbose: bool) -> Result<()> {
    let mut tick_seen: u64 = 0;
    let mut consecutive_fails: u32 = 0;
    loop {
        let obs: serde_json::Value = match client.observe().await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[demo] observe err: {e}");
                consecutive_fails += 1;
                if consecutive_fails > 10 {
                    return Err(e);
                }
                tokio::time::sleep(Duration::from_millis(period_ms)).await;
                continue;
            }
        };
        consecutive_fails = 0;
        let t = obs["tick"].as_u64().unwrap_or(0);
        if t == tick_seen {
            tokio::time::sleep(Duration::from_millis(period_ms / 4)).await;
            continue;
        }
        tick_seen = t;

        let action = decide(&obs);
        if verbose {
            let kind = action["kind"].as_str().unwrap_or("?");
            let hunger = obs["self"]["status"]["hunger"].as_i64().unwrap_or(0);
            let hp = obs["self"]["status"]["hp"].as_i64().unwrap_or(0);
            eprintln!("[t{t}] hp={hp} 饿={hunger} -> {kind}");
        }
        match client.act::<_, serde_json::Value>(&action).await {
            Ok(_) => {}
            Err(e) => eprintln!("[demo] act err: {e}"),
        }
        tokio::time::sleep(Duration::from_millis(period_ms)).await;
    }
}

fn decide(obs: &serde_json::Value) -> serde_json::Value {
    let state = obs["self"]["state"].as_str().unwrap_or("alive");
    if state != "alive" {
        return wait();
    }
    let hp = obs["self"]["status"]["hp"].as_i64().unwrap_or(100);
    let hunger = obs["self"]["status"]["hunger"].as_i64().unwrap_or(100);
    let stamina = obs["self"]["status"]["stamina"].as_i64().unwrap_or(100);
    let my_pos = parse_pos(&obs["self"]["pos"]);
    let inventory: Vec<(String, i64)> = obs["self"]["inventory"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|it| {
                    (
                        it["item"].as_str().unwrap_or("").to_string(),
                        it["n"].as_i64().unwrap_or(0),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    let entities = obs["visible_entities"].as_array();

    // 1. 受击/相邻 hostile？反击
    if let Some(arr) = entities {
        let nearest_hostile = arr
            .iter()
            .filter(|e| e["kind"] == "creature" && e["hostile"].as_bool().unwrap_or(false))
            .map(|e| (e, dist(my_pos, parse_pos(&e["pos"]))))
            .filter(|(_, d)| *d <= 1)
            .min_by_key(|(_, d)| *d);
        if let Some((e, _)) = nearest_hostile {
            let id = e["id"].as_u64().unwrap_or(0);
            return attack_creature(id);
        }
    }

    // 2. hunger < 35 → eat
    if hunger < 35 {
        for food in ["rice_cake", "cooked_mushroom", "cooked_berry", "lingzhi", "mushroom", "red_berry"] {
            if inventory.iter().any(|(k, n)| k == food && *n > 0) {
                return eat(food);
            }
        }
    }

    // 3. stamina 太低 → wait
    if stamina < 12 {
        return wait();
    }

    // hp 危急 → 也尽量吃东西保命
    if hp < 25 {
        for food in ["lingzhi", "rice_cake", "cooked_mushroom", "cooked_berry", "mushroom", "red_berry"] {
            if inventory.iter().any(|(k, n)| k == food && *n > 0) {
                return eat(food);
            }
        }
    }

    // 4 + 5. 找最近可采 plant
    if let Some(arr) = entities {
        let nearest_plant = arr
            .iter()
            .filter(|e| e["kind"] == "plant" && e["available"].as_bool().unwrap_or(false))
            .map(|e| (e, dist(my_pos, parse_pos(&e["pos"]))))
            .min_by_key(|(_, d)| *d);
        if let Some((e, d)) = nearest_plant {
            let pos = parse_pos(&e["pos"]);
            if d <= 1 {
                return gather(pos);
            }
            return move_toward(my_pos, pos);
        }
    }

    // 6. 在视野里找可走方向，挑一个；都不行就 wait
    random_walk_smart(my_pos, obs)
}

fn tile_walkable(kind: &str) -> bool {
    !matches!(kind, "mountain" | "deep_water")
}

fn random_walk_smart(my_pos: (i32, i32), obs: &serde_json::Value) -> serde_json::Value {
    let tiles = obs["vision"]["tiles"].as_array();
    let entities = obs["visible_entities"].as_array();
    let blocked_by_entity: std::collections::HashSet<(i32, i32)> = entities
        .map(|arr| {
            arr.iter()
                .filter(|e| {
                    e["kind"] == "building"
                        || e["kind"] == "agent"
                        || e["kind"] == "creature"
                })
                .map(|e| parse_pos(&e["pos"]))
                .collect()
        })
        .unwrap_or_default();
    let candidates: Vec<&str> = DIRS
        .iter()
        .copied()
        .filter(|d| {
            let target = step(my_pos, d);
            if blocked_by_entity.contains(&target) {
                return false;
            }
            match tiles {
                Some(arr) => arr
                    .iter()
                    .find(|t| parse_pos(&t["pos"]) == target)
                    .map(|t| {
                        let k = t["tile"]["kind"].as_str().unwrap_or("grass");
                        tile_walkable(k)
                    })
                    .unwrap_or(true),
                None => true,
            }
        })
        .collect();
    if candidates.is_empty() {
        return wait();
    }
    let mut rng = rand::thread_rng();
    let dir = candidates.choose(&mut rng).copied().unwrap_or("north");
    serde_json::json!({"kind":"move","data":{"dir":dir}})
}

fn step(p: (i32, i32), dir: &str) -> (i32, i32) {
    match dir {
        "north" => (p.0, p.1 - 1),
        "south" => (p.0, p.1 + 1),
        "east" => (p.0 + 1, p.1),
        "west" => (p.0 - 1, p.1),
        _ => p,
    }
}

fn parse_pos(v: &serde_json::Value) -> (i32, i32) {
    (
        v["x"].as_i64().unwrap_or(0) as i32,
        v["y"].as_i64().unwrap_or(0) as i32,
    )
}

fn dist(a: (i32, i32), b: (i32, i32)) -> i32 {
    (a.0 - b.0).abs() + (a.1 - b.1).abs()
}

fn move_toward(from: (i32, i32), to: (i32, i32)) -> serde_json::Value {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let dir = if dx.abs() >= dy.abs() {
        if dx > 0 {
            "east"
        } else {
            "west"
        }
    } else if dy > 0 {
        "south"
    } else {
        "north"
    };
    serde_json::json!({"kind":"move","data":{"dir":dir}})
}

fn random_walk() -> serde_json::Value {
    let mut rng = rand::thread_rng();
    let dir = DIRS.choose(&mut rng).copied().unwrap_or("north");
    serde_json::json!({"kind":"move","data":{"dir":dir}})
}

fn gather(pos: (i32, i32)) -> serde_json::Value {
    serde_json::json!({"kind":"gather","data":{"target":{"x":pos.0,"y":pos.1}}})
}

fn eat(item: &str) -> serde_json::Value {
    serde_json::json!({"kind":"eat","data":{"item":item}})
}

fn wait() -> serde_json::Value {
    serde_json::json!({"kind":"wait"})
}

fn attack_creature(id: u64) -> serde_json::Value {
    serde_json::json!({
        "kind":"attack",
        "data": {"target": { "target_kind": "creature", "target_id": id }}
    })
}

// 让 rand 编译进 cli 时被用到（防止 unused 警告）
#[allow(dead_code)]
fn _ensure_rng_used() {
    let _: u32 = rand::thread_rng().gen();
}
