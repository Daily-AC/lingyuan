mod client;
mod commands;
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
        Cmd::Act { verb, dir } => {
            let t = token_store::load().context("not joined yet")?;
            let c = client::Client::from_token(t);
            let action = match verb.as_str() {
                "move" => {
                    let d = dir.context("--dir required for move")?;
                    serde_json::json!({"kind":"move","data":{"dir":d}})
                }
                "wait" => serde_json::json!({"kind":"wait"}),
                v => anyhow::bail!("unknown verb {v}"),
            };
            let r: serde_json::Value = c.act(&action).await?;
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        Cmd::Clear => {
            token_store::clear()?;
            println!("cleared");
        }
    }
    Ok(())
}
