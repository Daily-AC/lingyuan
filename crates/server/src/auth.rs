use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use world::AgentId;

pub fn new_token() -> String {
    Uuid::new_v4().to_string().replace('-', "")
}

pub fn hash_token(tok: &str) -> String {
    let mut h = Sha256::new();
    h.update(tok.as_bytes());
    hex_lower(&h.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[derive(Debug, Clone)]
pub struct AuthAgent {
    pub agent_id: AgentId,
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthAgent {
    type Rejection = (StatusCode, &'static str);
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let _bearer = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or((StatusCode::UNAUTHORIZED, "missing bearer token"))?;
        let agent_id = parts
            .headers
            .get("x-agent-id")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "missing x-agent-id"))?;
        Ok(AuthAgent {
            agent_id: AgentId::new(agent_id),
        })
    }
}
