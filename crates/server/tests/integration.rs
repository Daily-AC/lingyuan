use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::time::sleep;

async fn wait_for_clock(base: &str, timeout: Duration) -> Option<u64> {
    let url = format!("{}/api/v1/world/clock", base);
    let start = std::time::Instant::now();
    let cli = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .no_proxy()
        .build()
        .unwrap();
    let mut attempts = 0u32;
    while start.elapsed() < timeout {
        attempts += 1;
        match cli.get(&url).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<serde_json::Value>().await {
                    Ok(v) => {
                        if let Some(t) = v["tick"].as_u64() {
                            return Some(t);
                        } else {
                            eprintln!("[wait_for_clock] status {} body {:?}", status, v);
                        }
                    }
                    Err(e) => eprintln!("[wait_for_clock] json err: {}", e),
                }
            }
            Err(e) => {
                if attempts <= 3 || attempts % 10 == 0 {
                    eprintln!("[wait_for_clock] attempt {} send err: {}", attempts, e);
                }
            }
        }
        sleep(Duration::from_millis(200)).await;
    }
    None
}

async fn dump_child_stderr(child: &mut tokio::process::Child) -> String {
    let mut out = String::new();
    if let Some(mut s) = child.stdout.take() {
        let mut buf = String::new();
        let _ = tokio::time::timeout(Duration::from_millis(500), s.read_to_string(&mut buf)).await;
        out.push_str("--- stdout ---\n");
        out.push_str(&buf);
    }
    if let Some(mut s) = child.stderr.take() {
        let mut buf = String::new();
        let _ = tokio::time::timeout(Duration::from_millis(500), s.read_to_string(&mut buf)).await;
        out.push_str("\n--- stderr ---\n");
        out.push_str(&buf);
    }
    out
}

#[tokio::test]
async fn server_starts_and_advances_clock() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17777")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "100")
        .env("RUST_LOG", "info,server=debug")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let base = "http://127.0.0.1:17777";
    let t1 = match wait_for_clock(base, Duration::from_secs(10)).await {
        Some(t) => t,
        None => {
            child.kill().await.ok();
            let stderr = dump_child_stderr(&mut child).await;
            panic!("server never became ready. stderr:\n{}", stderr);
        }
    };
    sleep(Duration::from_millis(500)).await;
    let t2 = wait_for_clock(base, Duration::from_secs(5))
        .await
        .expect("clock endpoint stopped responding");

    assert!(t2 > t1, "clock should advance ({} -> {})", t1, t2);
    child.kill().await.ok();
}

#[tokio::test]
async fn hunger_decays_over_long_run() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17779")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "30")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let base = "http://127.0.0.1:17779";
    assert!(wait_for_clock(base, Duration::from_secs(10)).await.is_some());

    let cli = reqwest::Client::builder().no_proxy().build().unwrap();
    let join: serde_json::Value = cli
        .post(format!("{}/api/v1/join", base))
        .json(&serde_json::json!({"name":"eve"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = join["agent_id"].as_str().unwrap().to_string();
    let tok = join["token"].as_str().unwrap().to_string();

    // 跑 ~6s @30ms = 200 tick；hunger 应当从 100 降到 ~50
    sleep(Duration::from_millis(6000)).await;
    let obs: serde_json::Value = cli
        .get(format!("{}/api/v1/observe", base))
        .header("Authorization", format!("Bearer {}", tok))
        .header("X-Agent-Id", &id)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let hunger = obs["self"]["status"]["hunger"].as_i64().unwrap();
    assert!(
        (30..=80).contains(&hunger),
        "hunger after 6s = {}",
        hunger
    );
    let inv = obs["self"]["inventory"].as_array().unwrap();
    // 现在 spawn 自带 red_berry × 3 (starter food)
    assert_eq!(inv.len(), 1, "spawn inventory should have starter food");
    assert_eq!(inv[0]["item"], "red_berry");
    assert_eq!(inv[0]["n"], 3);

    child.kill().await.ok();
}

#[tokio::test]
async fn world_info_exposes_recipes_and_constants() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17781")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "200")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let base = "http://127.0.0.1:17781";
    assert!(wait_for_clock(base, Duration::from_secs(10)).await.is_some());

    let cli = reqwest::Client::builder().no_proxy().build().unwrap();
    let info: serde_json::Value = cli
        .get(format!("{}/api/v1/world/info", base))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // 时钟
    assert!(info["clock"]["tick"].as_u64().is_some());
    assert_eq!(info["clock"]["season"], serde_json::json!("chun"));

    // 常量
    assert_eq!(info["constants"]["vision_radius"], 6);
    assert_eq!(info["constants"]["interaction_range"], 1);
    assert_eq!(info["constants"]["inventory_slots"], 20);
    assert_eq!(info["constants"]["max_hp"], 100);
    assert_eq!(info["constants"]["hunger_decay_period_ticks"], 4);
    let weapons = info["constants"]["weapon_damage"].as_array().unwrap();
    assert!(weapons.iter().any(|w| w["item"].is_null() && w["damage"] == 3));
    assert!(weapons
        .iter()
        .any(|w| w["item"] == "bamboo_spear" && w["damage"] == 8));

    // 配方 — 至少应该看到 campfire_kit
    let recipes = info["recipes"].as_array().unwrap();
    assert_eq!(recipes.len(), 9, "expect 9 recipes hardcoded");
    let campfire = recipes
        .iter()
        .find(|r| r["id"] == "campfire_kit")
        .expect("campfire_kit recipe missing");
    assert_eq!(campfire["station"], "hand");
    let inputs = campfire["inputs"].as_array().unwrap();
    assert!(inputs
        .iter()
        .any(|x| x["item"] == "pinewood" && x["n"] == 3));
    assert!(inputs.iter().any(|x| x["item"] == "flint" && x["n"] == 1));
    assert_eq!(campfire["output"]["item"], "campfire_kit");

    // 物品 — 19 个 item，至少 mushroom 有 nutrition
    let items = info["items"].as_array().unwrap();
    assert_eq!(items.len(), 19);
    let mushroom = items.iter().find(|i| i["id"] == "mushroom").unwrap();
    assert_eq!(mushroom["is_food"], true);
    assert_eq!(mushroom["nutrition"]["hunger"], 8);
    let stone = items.iter().find(|i| i["id"] == "stone").unwrap();
    assert_eq!(stone["is_food"], false);
    assert!(stone["nutrition"].is_null());

    child.kill().await.ok();
}

#[tokio::test]
async fn act_rejects_duplicate_in_same_tick() {
    // 验证：同一 agent 在 tick 落地前连发两个 action，第二个被 409 拒绝，
    // 并附带既存 action 和 will_resolve_at_tick。
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");
    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17780")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        // tick 慢一些，留充足窗口连发两次
        .env("LINGYUAN_TICK_MS", "1500")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let base = "http://127.0.0.1:17780";
    assert!(wait_for_clock(base, Duration::from_secs(10)).await.is_some());

    let cli = reqwest::Client::builder().no_proxy().build().unwrap();
    let join: serde_json::Value = cli
        .post(format!("{}/api/v1/join", base))
        .json(&serde_json::json!({"name":"dup"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = join["agent_id"].as_str().unwrap().to_string();
    let tok = join["token"].as_str().unwrap().to_string();

    let post_act = |body: serde_json::Value| {
        let cli = cli.clone();
        let id = id.clone();
        let tok = tok.clone();
        async move {
            cli.post(format!("{}/api/v1/act", base))
                .header("Authorization", format!("Bearer {}", tok))
                .header("X-Agent-Id", &id)
                .json(&body)
                .send()
                .await
                .unwrap()
        }
    };

    // 第一发：accepted=true，吐回 will_resolve_at_tick
    let r1 = post_act(serde_json::json!({"kind":"move","data":{"dir":"north"}})).await;
    assert!(r1.status().is_success(), "first act should accept");
    let v1: serde_json::Value = r1.json().await.unwrap();
    assert_eq!(v1["accepted"], serde_json::json!(true));
    let will1 = v1["will_resolve_at_tick"].as_u64().unwrap();
    assert!(v1["accepted_at_tick"].as_u64().unwrap() + 1 == will1);
    assert!(v1["queue_depth"].as_u64().unwrap() >= 1);

    // 第二发：同 tick 内重复 — 应当 409，且回吐既存 action
    let r2 = post_act(serde_json::json!({"kind":"wait"})).await;
    assert_eq!(r2.status(), 409, "duplicate act must be rejected");
    let v2: serde_json::Value = r2.json().await.unwrap();
    assert_eq!(v2["accepted"], serde_json::json!(false));
    assert_eq!(v2["reason"], serde_json::json!("already_queued"));
    assert_eq!(v2["existing_action"]["kind"], serde_json::json!("move"));
    assert_eq!(v2["will_resolve_at_tick"].as_u64().unwrap(), will1);

    // 等 tick 落地，再发一发应该 OK
    sleep(Duration::from_millis(2000)).await;
    let r3 = post_act(serde_json::json!({"kind":"wait"})).await;
    assert!(r3.status().is_success(), "after tick advances, new act accepted");

    child.kill().await.ok();
}

#[tokio::test]
async fn agent_can_join_and_observe_and_move() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("test.db");

    let mut child = Command::new(env!("CARGO_BIN_EXE_server"))
        .kill_on_drop(true)
        .env("LINGYUAN_BIND", "127.0.0.1:17778")
        .env("LINGYUAN_DB", db_path.to_str().unwrap())
        .env("LINGYUAN_TICK_MS", "100")
        .env("RUST_LOG", "info,server=debug")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let base = "http://127.0.0.1:17778";
    if wait_for_clock(base, Duration::from_secs(10)).await.is_none() {
        child.kill().await.ok();
        let stderr = dump_child_stderr(&mut child).await;
        panic!("server never became ready. stderr:\n{}", stderr);
    }

    let cli = reqwest::Client::builder().no_proxy().build().unwrap();
    let join_resp: serde_json::Value = cli
        .post(format!("{}/api/v1/join", base))
        .json(&serde_json::json!({ "name": "alice" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let agent_id = join_resp["agent_id"].as_str().unwrap().to_string();
    let token = join_resp["token"].as_str().unwrap().to_string();

    let obs: serde_json::Value = cli
        .get(format!("{}/api/v1/observe", base))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Agent-Id", &agent_id)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(obs["self"]["name"], "alice");
    assert!(obs["vision"]["tiles"].as_array().unwrap().len() > 1);

    let act_resp = cli
        .post(format!("{}/api/v1/act", base))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Agent-Id", &agent_id)
        .json(&serde_json::json!({"kind":"wait"}))
        .send()
        .await
        .unwrap();
    assert!(act_resp.status().is_success());

    sleep(Duration::from_millis(400)).await;
    let obs2: serde_json::Value = cli
        .get(format!("{}/api/v1/observe", base))
        .header("Authorization", format!("Bearer {}", token))
        .header("X-Agent-Id", &agent_id)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(obs2["tick"].as_u64().unwrap() > obs["tick"].as_u64().unwrap());

    child.kill().await.ok();
}
