// Auth module for API token authentication

pub mod handlers;
pub mod middleware;
pub mod service;

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

/// Database model for auth tokens
#[derive(Debug, Clone, FromRow)]
pub struct AuthToken {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub token_hash: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub active: bool,
}

/// API response for token info (excludes hash)
#[derive(Debug, Serialize, ToSchema)]
pub struct TokenInfo {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used_at: Option<DateTime<Utc>>,
    pub active: bool,
}

impl From<AuthToken> for TokenInfo {
    fn from(token: AuthToken) -> Self {
        Self {
            id: token.id,
            name: token.name,
            description: token.description,
            created_at: token.created_at,
            last_used_at: token.last_used_at,
            active: token.active,
        }
    }
}
