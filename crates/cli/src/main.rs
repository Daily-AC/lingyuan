mod client;
mod commands;
mod demo_bot;
mod render;
mod token_store;

use anyhow::Context;
use clap::Parser;
use commands::{Cli, Cmd};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Join { name, server } => {
            let tok = client::join_remote(&server, &name).await?;
            token_store::save(&tok)?;
            println!(
                "joined as {} (id {}) on {}",
                tok.name, tok.agent_id, tok.server
            );
        }
        Cmd::Leave => {
            let t = token_store::load().context("not joined yet")?;
            client::Client::from_token(t).leave().await?;
            token_store::clear()?;
            println!("left");
        }
        Cmd::Observe { format } => {
            let t = token_store::load().context("not joined yet")?;
            let c = client::Client::from_token(t);
            let obs: serde_json::Value = c.observe().await?;
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&obs)?),
                _ => print!("{}", render::render_markdown(&obs)),
            }
        }
        Cmd::Act {
            verb,
            dir,
            pos,
            item,
            recipe,
            n,
            target_kind,
            target,
            text,
            to,
        } => {
            let t = token_store::load().context("not joined yet")?;
            let c = client::Client::from_token(t);
            let parse_pos = |s: &str| -> anyhow::Result<serde_json::Value> {
                let (x, y) = s.split_once(',').context("--pos must be x,y")?;
                Ok(serde_json::json!({
                    "x": x.trim().parse::<i16>()?,
                    "y": y.trim().parse::<i16>()?,
                }))
            };
            let action = match verb.as_str() {
                "move" => {
                    let d = dir.context("--dir required for move")?;
                    serde_json::json!({"kind":"move","data":{"dir":d}})
                }
                "wait" => serde_json::json!({"kind":"wait"}),
                "observe" => serde_json::json!({"kind":"observe"}),
                "gather" => {
                    let p = parse_pos(&pos.context("--pos required for gather")?)?;
                    serde_json::json!({"kind":"gather","data":{"target": p}})
                }
                "eat" => {
                    let i = item.context("--item required for eat")?;
                    serde_json::json!({"kind":"eat","data":{"item": i}})
                }
                "craft" => {
                    let r = recipe.context("--recipe required for craft")?;
                    serde_json::json!({"kind":"craft","data":{"recipe": r}})
                }
                "place" => {
                    let i = item.context("--item required for place")?;
                    let p = parse_pos(&pos.context("--pos required for place")?)?;
                    serde_json::json!({"kind":"place","data":{"item": i, "pos": p}})
                }
                "pickup" => {
                    let p = parse_pos(&pos.context("--pos required for pickup")?)?;
                    serde_json::json!({"kind":"pick_up","data":{"pos": p}})
                }
                "drop" => {
                    let i = item.context("--item required for drop")?;
                    serde_json::json!({"kind":"drop","data":{"item": i, "n": n}})
                }
                "attack" => {
                    let tk = target_kind
                        .context("--target-kind=agent|creature required for attack")?;
                    let tid = target.context("--target=<id> required for attack")?;
                    let target_value = if tk == "creature" {
                        serde_json::Value::Number(tid.parse::<u64>()?.into())
                    } else {
                        serde_json::Value::String(tid)
                    };
                    serde_json::json!({
                        "kind":"attack",
                        "data": {"target": { "target_kind": tk, "target_id": target_value }}
                    })
                }
                "write_sign" | "sign" => {
                    let p = parse_pos(&pos.context("--pos required for sign")?)?;
                    let t = text.context("--text required for sign")?;
                    serde_json::json!({"kind":"write_sign","data":{"pos": p, "text": t}})
                }
                "send_mail" | "mail" => {
                    let target_name = to.context("--to=<name> required for mail")?;
                    let t = text.context("--text required for mail")?;
                    serde_json::json!({"kind":"send_mail","data":{"to": target_name, "text": t}})
                }
                v => anyhow::bail!("unknown verb {v}"),
            };
            let r: serde_json::Value = c.act(&action).await?;
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        Cmd::Clear => {
            token_store::clear()?;
            println!("cleared");
        }
        Cmd::Demo {
            name,
            server,
            period_ms,
            verbose,
        } => {
            // 用临时 token，不污染全局 token store
            let tok = client::join_remote(&server, &name).await?;
            println!(
                "[demo] joined as {} (id {}) — Ctrl-C 退出",
                tok.name, tok.agent_id
            );
            let c = client::Client::from_token(tok);
            demo_bot::run(c, period_ms, verbose).await?;
        }
    }
    Ok(())
}
