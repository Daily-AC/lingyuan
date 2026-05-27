//! 读 SQLite events 表回放某 tick 区间的事件流。
//! 用 shell out 到 sqlite3 binary，避免引入 rusqlite 与 server 的 sqlx 冲突。

use anyhow::{anyhow, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub fn run(
    db_path: &Path,
    from: u64,
    to: u64,
    kinds_filter: Option<Vec<String>>,
    summary: bool,
) -> Result<()> {
    if !db_path.exists() {
        return Err(anyhow!("db file not found: {}", db_path.display()));
    }
    let sql = format!(
        "SELECT tick || '\t' || seq || '\t' || event_json FROM events WHERE tick BETWEEN {} AND {} ORDER BY tick ASC, seq ASC",
        from, to.min(i64::MAX as u64),
    );
    let out = Command::new("sqlite3")
        .arg("-noheader")
        .arg("-separator")
        .arg("\n") // 一行一条记录
        .arg(db_path)
        .arg(sql)
        .output()
        .context("spawn sqlite3 (确保 sqlite3 binary 在 PATH)")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("sqlite3 失败: {}", err));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);

    let filter: Option<std::collections::HashSet<String>> =
        kinds_filter.map(|v| v.into_iter().collect());
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    let mut total = 0u64;
    let mut min_tick: Option<u64> = None;
    let mut max_tick: Option<u64> = None;
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\t');
        let tick: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let seq: u64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let json = parts.next().unwrap_or("{}");
        let parsed: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let kind = parsed["kind"].as_str().unwrap_or("?").to_string();
        if let Some(f) = &filter {
            if !f.contains(&kind) {
                continue;
            }
        }
        total += 1;
        *counts.entry(kind.clone()).or_default() += 1;
        min_tick = Some(min_tick.map(|m| m.min(tick)).unwrap_or(tick));
        max_tick = Some(max_tick.map(|m| m.max(tick)).unwrap_or(tick));
        let data = &parsed["data"];
        let data_str = if data.is_null() {
            String::new()
        } else {
            serde_json::to_string(data).unwrap_or_default()
        };
        println!("[t{tick:>6}#{seq:>2}] {kind:<28} {data_str}");
    }
    if summary {
        println!("\n=== summary ===");
        println!(
            "total {total} events ({}..={})",
            min_tick.unwrap_or(0),
            max_tick.unwrap_or(0)
        );
        for (k, n) in counts.iter() {
            println!("  {k:<28} {n}");
        }
    }
    Ok(())
}
