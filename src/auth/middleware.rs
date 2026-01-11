// Authentication middleware for admin routes

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request},
    middleware::Next,
    response::Response,
};

use super::service::validate_token;
use crate::{errors::ApiError, AppState};

/// Middleware that requires authentication via Bearer token
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
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

    // Validate the token
    if !validate_token(&state.pool, token).await? {
        return Err(ApiError::Unauthorized);
    }

    Ok(next.run(request).await)
}
