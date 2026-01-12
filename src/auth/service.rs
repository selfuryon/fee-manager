// Token service: generation, validation, and CRUD operations

use rand::Rng;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use super::AuthToken;
use crate::errors::ApiError;

const TOKEN_LENGTH: usize = 32;

/// Generate a new random token and its hash
pub fn generate_token() -> (String, String) {
    let mut rng = rand::rng();
    let token_bytes: [u8; TOKEN_LENGTH] = rng.random();
    let token = hex::encode(token_bytes);
    let hash = hash_token(&token);
    (token, hash)
}

/// Hash a token using SHA-256
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

/// Validate a token against the database
pub async fn validate_token(pool: &PgPool, token: &str) -> Result<bool, ApiError> {
    let hash = hash_token(token);

    let result = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM auth_tokens
            WHERE token_hash = $1 AND active = true
        ) as "exists!"
        "#,
        hash
    )
    .fetch_one(pool)
    .await?;

    if result {
        // Update last_used_at
        sqlx::query!(
            "UPDATE auth_tokens SET last_used_at = NOW() WHERE token_hash = $1",
            hash
        )
        .execute(pool)
        .await?;
    }

    Ok(result)
}

/// Get token info by hash (for audit logging)
/// Returns the token with updated last_used_at
pub async fn get_token_by_hash(pool: &PgPool, token: &str) -> Result<Option<AuthToken>, ApiError> {
    let hash = hash_token(token);

    let token = sqlx::query_as!(
        AuthToken,
        r#"
        SELECT id, name, description, token_hash, created_at, last_used_at, active
        FROM auth_tokens
        WHERE token_hash = $1
        "#,
        hash
    )
    .fetch_optional(pool)
    .await?;

    Ok(token)
}

/// Update last_used_at for a token
pub async fn update_last_used(pool: &PgPool, id: Uuid) -> Result<(), ApiError> {
    sqlx::query!("UPDATE auth_tokens SET last_used_at = NOW() WHERE id = $1", id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Create a new token in the database
pub async fn create_token(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
) -> Result<(AuthToken, String), ApiError> {
    let (plaintext, hash) = generate_token();

    let token = sqlx::query_as!(
        AuthToken,
        r#"
        INSERT INTO auth_tokens (name, description, token_hash)
        VALUES ($1, $2, $3)
        RETURNING id, name, description, token_hash, created_at, last_used_at, active
        "#,
        name,
        description,
        hash
    )
    .fetch_one(pool)
    .await?;

    Ok((token, plaintext))
}

/// List all tokens (without hashes)
pub async fn list_tokens(pool: &PgPool) -> Result<Vec<AuthToken>, ApiError> {
    let tokens = sqlx::query_as!(
        AuthToken,
        r#"
        SELECT id, name, description, token_hash, created_at, last_used_at, active
        FROM auth_tokens
        ORDER BY created_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(tokens)
}

/// Get a single token by ID
pub async fn get_token(pool: &PgPool, id: Uuid) -> Result<Option<AuthToken>, ApiError> {
    let token = sqlx::query_as!(
        AuthToken,
        r#"
        SELECT id, name, description, token_hash, created_at, last_used_at, active
        FROM auth_tokens
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(pool)
    .await?;

    Ok(token)
}

/// Delete a token by ID
pub async fn delete_token(pool: &PgPool, id: Uuid) -> Result<bool, ApiError> {
    let result = sqlx::query!("DELETE FROM auth_tokens WHERE id = $1", id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected() > 0)
}

/// Ensure a default token exists (creates one if table is empty)
/// Returns the plaintext token if a new one was created
pub async fn ensure_default_token(pool: &PgPool) -> Result<Option<String>, ApiError> {
    // Check if any tokens exist
    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) as \"count!\" FROM auth_tokens")
        .fetch_one(pool)
        .await?;

    if count > 0 {
        return Ok(None);
    }

    // Create default token
    let (_, plaintext) = create_token(pool, "default", Some("Auto-generated initial token")).await?;

    Ok(Some(plaintext))
}
