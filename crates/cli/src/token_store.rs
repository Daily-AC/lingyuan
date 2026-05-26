use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenFile {
    pub agent_id: String,
    pub token: String,
    pub server: String,
    pub name: String,
}

pub fn store_path() -> PathBuf {
    if let Ok(p) = std::env::var("LINGYUAN_TOKEN_PATH") {
        return PathBuf::from(p);
    }
    let mut p = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push(".lingyuan");
    p.push("token.json");
    p
}

pub fn load() -> anyhow::Result<TokenFile> {
    let bytes = std::fs::read(store_path())?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn save(t: &TokenFile) -> anyhow::Result<()> {
    let p = store_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&p, serde_json::to_vec_pretty(t)?)?;
    Ok(())
}

pub fn clear() -> anyhow::Result<()> {
    let p = store_path();
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    Ok(())
}
