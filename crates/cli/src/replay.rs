//! 读 SQLite events 表回放某 tick 区间的事件流。
//! 用 shell out 到 sqlite3 binary，避免引入 rusqlite 与 server 的 sqlx 冲突。

use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn stats(db_path: &Path) -> Result<()> {
    if !db_path.exists() {
        return Err(anyhow!("db file not found: {}", db_path.display()));
    }
    // 一口气拉所有事件
    let out = Command::new("sqlite3")
        .arg("-noheader")
        .arg(db_path)
        .arg("SELECT event_json FROM events")
        .output()
        .context("sqlite3")?;
    if !out.status.success() {
        return Err(anyhow!("sqlite3 failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let mut gathers: BTreeMap<String, u32> = BTreeMap::new();
    let mut kills: BTreeMap<String, u32> = BTreeMap::new();
    let mut deaths: BTreeMap<String, u32> = BTreeMap::new();
    let mut crafted: BTreeMap<String, u32> = BTreeMap::new();
    let mut signs: BTreeMap<String, u32> = BTreeMap::new();
    let mut mails: BTreeMap<String, u32> = BTreeMap::new();
    let mut names: BTreeMap<String, String> = BTreeMap::new();
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.is_empty() { continue; }
        let v: serde_json::Value = match serde_json::from_str(line) { Ok(x) => x, Err(_) => continue };
        let kind = v["kind"].as_str().unwrap_or("");
        let d = &v["data"];
        match kind {
            "agent_joined" => {
                if let (Some(id), Some(n)) = (d["agent"].as_str(), d["name"].as_str()) {
                    names.insert(id.to_string(), n.to_string());
                }
            }
            "agent_gathered" => {
                if let Some(a) = d["agent"].as_str() { *gathers.entry(a.into()).or_default() += 1; }
            }
            "agent_attacked_creature" => {
                // 算击杀（粗略：每次攻击都记 1；精确应 join creature_killed event slayer，但 server 当前只在 boss_killed 暴露 slayer）
                if let Some(a) = d["attacker"].as_str() { *kills.entry(a.into()).or_default() += 1; }
            }
            "agent_died" => {
                if let Some(a) = d["agent"].as_str() { *deaths.entry(a.into()).or_default() += 1; }
            }
            "agent_crafted" => {
                if let Some(a) = d["agent"].as_str() { *crafted.entry(a.into()).or_default() += 1; }
            }
            "agent_wrote_sign" => {
                if let Some(a) = d["agent"].as_str() { *signs.entry(a.into()).or_default() += 1; }
            }
            "agent_sent_mail" => {
                if let Some(a) = d["from"].as_str() { *mails.entry(a.into()).or_default() += 1; }
            }
            _ => {}
        }
    }
    let all_ids: std::collections::BTreeSet<String> = gathers.keys()
        .chain(kills.keys()).chain(deaths.keys())
        .chain(crafted.keys()).chain(signs.keys()).chain(mails.keys())
        .cloned().collect();
    println!("{:<18} {:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "agent_id", "name", "gather", "atk", "craft", "sign", "mail", "die");
    println!("{}", "-".repeat(90));
    let mut rows: Vec<_> = all_ids.iter().map(|id| {
        let g = *gathers.get(id).unwrap_or(&0);
        let k = *kills.get(id).unwrap_or(&0);
        let dn = *deaths.get(id).unwrap_or(&0);
        let cr = *crafted.get(id).unwrap_or(&0);
        let sg = *signs.get(id).unwrap_or(&0);
        let ml = *mails.get(id).unwrap_or(&0);
        let n = names.get(id).cloned().unwrap_or_else(|| "?".into());
        (id.clone(), n, g, k, cr, sg, ml, dn)
    }).collect();
    rows.sort_by_key(|r| std::cmp::Reverse((r.2 + r.3) as i64));
    for (id, n, g, k, cr, sg, ml, dn) in rows {
        println!("{:<18} {:<14} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}", id, n, g, k, cr, sg, ml, dn);
    }
    Ok(())
}

pub fn watch(db_path: &Path, kinds_filter: Option<Vec<String>>, only_new: bool) -> Result<()> {
    if !db_path.exists() {
        return Err(anyhow!("db file not found: {}", db_path.display()));
    }
    let filter: Option<HashSet<String>> =
        kinds_filter.map(|v| v.into_iter().collect());
    let mut last_tick: i64 = if only_new {
        query_max_tick(db_path)?.unwrap_or(0)
    } else {
        -1
    };
    eprintln!("[watch] tail from tick {} (Ctrl-C to exit)", last_tick.max(0));
    loop {
        let rows = read_since(db_path, last_tick)?;
        for (tick, seq, json) in rows {
            let parsed: serde_json::Value = match serde_json::from_str(&json) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let kind = parsed["kind"].as_str().unwrap_or("?").to_string();
            if let Some(f) = &filter {
                if !f.contains(&kind) {
                    last_tick = last_tick.max(tick);
                    continue;
                }
            }
            let data = &parsed["data"];
            let data_str = if data.is_null() {
                String::new()
            } else {
                serde_json::to_string(data).unwrap_or_default()
            };
            println!("[t{tick:>6}#{seq:>2}] {kind:<28} {data_str}");
            last_tick = last_tick.max(tick);
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn query_max_tick(db_path: &Path) -> Result<Option<i64>> {
    let out = Command::new("sqlite3")
        .arg("-noheader")
        .arg(db_path)
        .arg("SELECT COALESCE(MAX(tick), -1) FROM events")
        .output()
        .context("sqlite3")?;
    if !out.status.success() {
        return Ok(None);
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    s.parse::<i64>().ok().map(Some).ok_or_else(|| anyhow!("bad max tick: {}", s))
}

fn read_since(db_path: &Path, since: i64) -> Result<Vec<(i64, i64, String)>> {
    let sql = format!(
        "SELECT tick || char(31) || seq || char(31) || event_json FROM events WHERE tick > {} ORDER BY tick ASC, seq ASC",
        since,
    );
    let out = Command::new("sqlite3")
        .arg("-noheader")
        .arg("-separator").arg("\n")
        .arg(db_path)
        .arg(sql)
        .output()
        .context("sqlite3")?;
    if !out.status.success() {
        return Err(anyhow!("sqlite3 failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut rows = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, '\x1f');
        let t: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let s: i64 = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let j = parts.next().unwrap_or("{}").to_string();
        rows.push((t, s, j));
    }
    Ok(rows)
}

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
