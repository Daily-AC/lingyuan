use crate::token_store::TokenFile;
use anyhow::anyhow;
use serde::{de::DeserializeOwned, Serialize};

pub struct Client {
    http: reqwest::Client,
    base: String,
    token: TokenFile,
}

fn new_http() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    // 默认绕开系统代理：本地 server 走 proxy 会失败
    if std::env::var("LINGYUAN_USE_PROXY").is_err() {
        builder = builder.no_proxy();
    }
    builder.build().unwrap_or_else(|_| reqwest::Client::new())
}

impl Client {
    pub fn from_token(t: TokenFile) -> Self {
        Self {
            http: new_http(),
            base: t.server.clone(),
            token: t,
        }
    }

    pub async fn observe<T: DeserializeOwned>(&self) -> anyhow::Result<T> {
        let r = self
            .http
            .get(format!("{}/api/v1/observe", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .send()
            .await?;
        if !r.status().is_success() {
            let s = r.status();
            let t = r.text().await.unwrap_or_default();
            return Err(anyhow!("{}: {}", s, t));
        }
        Ok(r.json().await?)
    }

    pub async fn act<A: Serialize, R: DeserializeOwned>(&self, action: &A) -> anyhow::Result<R> {
        let r = self
            .http
            .post(format!("{}/api/v1/act", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .json(action)
            .send()
            .await?;
        if !r.status().is_success() {
            return Err(anyhow!(
                "{}: {}",
                r.status(),
                r.text().await.unwrap_or_default()
            ));
        }
        Ok(r.json().await?)
    }

    pub async fn leave(&self) -> anyhow::Result<()> {
        self.http
            .post(format!("{}/api/v1/leave", self.base))
            .header("Authorization", format!("Bearer {}", self.token.token))
            .header("X-Agent-Id", &self.token.agent_id)
            .send()
            .await?;
        Ok(())
    }
}

pub async fn join_remote(server: &str, name: &str) -> anyhow::Result<TokenFile> {
    let r = new_http()
        .post(format!("{}/api/v1/join", server))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await?;
    if !r.status().is_success() {
        return Err(anyhow!(
            "{}: {}",
            r.status(),
            r.text().await.unwrap_or_default()
        ));
    }
    let v: serde_json::Value = r.json().await?;
    Ok(TokenFile {
        agent_id: v["agent_id"].as_str().unwrap().to_string(),
        token: v["token"].as_str().unwrap().to_string(),
        server: server.to_string(),
        name: name.to_string(),
    })
}
