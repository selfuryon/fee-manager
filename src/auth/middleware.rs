// Authentication middleware for admin routes

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};

use super::service::{get_token_by_hash, update_last_used};
use crate::{audit::ActorInfo, errors::ApiError, AppState};

/// Middleware that requires authentication via Bearer token
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // Skip authentication if disabled in config
    if !state.config.auth.enabled {
        return Ok(next.run(request).await);
    }

    // Extract Bearer token from Authorization header
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .ok_or(ApiError::Unauthorized)?;

    // Validate and get token info
    let token_info = get_token_by_hash(&state.pool, token)
        .await?
        .ok_or(ApiError::Unauthorized)?;

    if !token_info.active {
        return Err(ApiError::Unauthorized);
    }

    // Update last_used_at
    update_last_used(&state.pool, token_info.id).await?;

    // Insert actor info into request extensions for audit logging
    request.extensions_mut().insert(ActorInfo {
        token_id: token_info.id,
        token_name: token_info.name,
    });

    Ok(next.run(request).await)
}
