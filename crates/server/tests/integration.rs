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
