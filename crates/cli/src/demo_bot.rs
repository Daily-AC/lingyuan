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

struct BotState {
    last_sign_tick: u64,
    own_name: String,
}

pub async fn run(client: Client, period_ms: u64, verbose: bool) -> Result<()> {
    let mut tick_seen: u64 = 0;
    let mut consecutive_fails: u32 = 0;
    let mut bot = BotState {
        last_sign_tick: 0,
        own_name: String::new(),
    };
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
        if bot.own_name.is_empty() {
            bot.own_name = obs["self"]["name"]
                .as_str()
                .unwrap_or("")
                .to_string();
        }
        let t = obs["tick"].as_u64().unwrap_or(0);
        if t == tick_seen {
            tokio::time::sleep(Duration::from_millis(period_ms / 4)).await;
            continue;
        }
        tick_seen = t;

        let action = decide_with_state(&obs, &mut bot, t);
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

fn decide_with_state(obs: &serde_json::Value, bot: &mut BotState, tick: u64) -> serde_json::Value {
    // 优先：偶尔写路牌（200 tick 一次 + 视野有 3+ 富资源）
    if tick.saturating_sub(bot.last_sign_tick) > 200 {
        if let Some(sign) = maybe_write_sign(obs, bot, tick) {
            bot.last_sign_tick = tick;
            return sign;
        }
    }
    decide(obs)
}

fn maybe_write_sign(
    obs: &serde_json::Value,
    bot: &BotState,
    tick: u64,
) -> Option<serde_json::Value> {
    let _ = tick;
    let entities = obs["visible_entities"].as_array()?;
    let plants: Vec<&serde_json::Value> = entities
        .iter()
        .filter(|e| e["kind"] == "plant" && e["available"].as_bool().unwrap_or(false))
        .collect();
    if plants.len() < 3 {
        return None;
    }
    // 统计 species
    let mut counts: std::collections::HashMap<String, u32> = Default::default();
    for p in &plants {
        let s = p["species"].as_str().unwrap_or("").to_string();
        *counts.entry(s).or_default() += 1;
    }
    let (top_species, top_count) = counts
        .into_iter()
        .max_by_key(|(_, c)| *c)?;
    if top_count < 2 {
        return None;
    }
    // 找一个相邻可写 tile
    let my_pos = parse_pos(&obs["self"]["pos"]);
    let pos = find_walkable_neighbor(my_pos, obs)?;
    let text = format!(
        "{} 处 {} 丰盛 — {}",
        top_species_label(&top_species),
        top_count,
        bot.own_name
    );
    Some(serde_json::json!({
        "kind":"write_sign",
        "data":{"pos":{"x":pos.0,"y":pos.1},"text":text}
    }))
}

fn top_species_label(s: &str) -> &'static str {
    match s {
        "lingzhi" => "灵芝",
        "mushroom" => "菇",
        "red_berry" => "朱果",
        "bamboo_stalk" => "竹",
        "pine_log" => "松木",
        "stone_chunk" => "石",
        "flint_chunk" => "燧石",
        "clay_lump" => "陶土",
        "vine" => "藤",
        "reed" => "苇",
        _ => "此",
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

    // 1. hp 低 + 视野内有 hostile → 逃
    if let Some(arr) = entities {
        let hostiles: Vec<((i32, i32), i32)> = arr
            .iter()
            .filter(|e| e["kind"] == "creature" && e["hostile"].as_bool().unwrap_or(false))
            .map(|e| (parse_pos(&e["pos"]), dist(my_pos, parse_pos(&e["pos"]))))
            .collect();
        if hp < 40 {
            if let Some((p, _)) = hostiles.iter().min_by_key(|(_, d)| *d) {
                // 朝相反方向走一步
                let opp = (my_pos.0 - p.0, my_pos.1 - p.1);
                return move_toward(my_pos, (my_pos.0 + opp.0, my_pos.1 + opp.1));
            }
        }
        // 相邻有 hostile 且 hp 够 → 反击
        if hp >= 40 {
            if let Some((e, _)) = arr
                .iter()
                .filter(|e| e["kind"] == "creature" && e["hostile"].as_bool().unwrap_or(false))
                .map(|e| (e, dist(my_pos, parse_pos(&e["pos"]))))
                .filter(|(_, d)| *d <= 1)
                .min_by_key(|(_, d)| *d)
            {
                let id = e["id"].as_u64().unwrap_or(0);
                return attack_creature(id);
            }
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

    // 3.5 有 campfire_kit 在身 → 找空地放下（夜间或视野无 hostile）
    let has_campfire_kit = inventory.iter().any(|(k, n)| k == "campfire_kit" && *n > 0);
    if has_campfire_kit {
        if let Some(pos) = find_walkable_neighbor(my_pos, obs) {
            return place_item("campfire_kit", pos);
        }
    }

    // 3.6 材料齐 + 没火堆 kit → 合成 campfire_kit（pinewood ×3 + flint ×1）
    let pinewood = inventory
        .iter()
        .find(|(k, _)| k == "pinewood")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    let flint = inventory
        .iter()
        .find(|(k, _)| k == "flint")
        .map(|(_, n)| *n)
        .unwrap_or(0);
    let bag_total: i64 = inventory.iter().map(|(_, n)| *n).sum();
    if pinewood >= 3 && flint >= 1 && !has_campfire_kit && bag_total < 18 {
        return craft("campfire_kit");
    }

    // hp 危急 → 也尽量吃东西保命
    if hp < 25 {
        for food in ["lingzhi", "rice_cake", "cooked_mushroom", "cooked_berry", "mushroom", "red_berry"] {
            if inventory.iter().any(|(k, n)| k == food && *n > 0) {
                return eat(food);
            }
        }
    }

    // 4 + 5. 找最近可采 plant，hunger 低时只挑食物类（mushroom/red_berry/lingzhi）
    let prefer_food = hunger < 60;
    let food_species = ["mushroom", "red_berry", "lingzhi"];
    if let Some(arr) = entities {
        let pick_plant = |only_food: bool| -> Option<(serde_json::Value, i32)> {
            arr.iter()
                .filter(|e| e["kind"] == "plant" && e["available"].as_bool().unwrap_or(false))
                .filter(|e| {
                    if !only_food {
                        return true;
                    }
                    let s = e["species"].as_str().unwrap_or("");
                    food_species.contains(&s)
                })
                .map(|e| (e.clone(), dist(my_pos, parse_pos(&e["pos"]))))
                .min_by_key(|(_, d)| *d)
        };
        let target = if prefer_food {
            pick_plant(true).or_else(|| pick_plant(false))
        } else {
            pick_plant(false)
        };
        if let Some((e, d)) = target {
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
                    // 不在视野 = 出界 or 被遮挡，保守跳过
                    .unwrap_or(false),
                None => false,
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

fn craft(recipe: &str) -> serde_json::Value {
    serde_json::json!({"kind":"craft","data":{"recipe":recipe}})
}

fn place_item(item: &str, pos: (i32, i32)) -> serde_json::Value {
    serde_json::json!({
        "kind":"place",
        "data":{"item":item,"pos":{"x":pos.0,"y":pos.1}}
    })
}

fn find_walkable_neighbor(my_pos: (i32, i32), obs: &serde_json::Value) -> Option<(i32, i32)> {
    let tiles = obs["vision"]["tiles"].as_array()?;
    let entities = obs["visible_entities"].as_array();
    let occupied: std::collections::HashSet<(i32, i32)> = entities
        .map(|arr| arr.iter().map(|e| parse_pos(&e["pos"])).collect())
        .unwrap_or_default();
    for d in DIRS.iter() {
        let p = step(my_pos, d);
        if occupied.contains(&p) {
            continue;
        }
        let walkable = tiles
            .iter()
            .find(|t| parse_pos(&t["pos"]) == p)
            .map(|t| tile_walkable(t["tile"]["kind"].as_str().unwrap_or("grass")))
            .unwrap_or(false);
        if walkable {
            return Some(p);
        }
    }
    None
}

// 让 rand 编译进 cli 时被用到（防止 unused 警告）
#[allow(dead_code)]
fn _ensure_rng_used() {
    let _: u32 = rand::thread_rng().gen();
}
