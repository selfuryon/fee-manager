// handlers/mod.rs - Main router and health endpoints
use crate::auth;
use crate::openapi;
use crate::AppState;
use axum::{
    body::Body, http::Request, middleware, response::IntoResponse, routing::get, Json, Router,
};
use serde::Serialize;
use std::sync::Arc;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tracing::instrument;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

pub mod commit_boost;
pub mod vouch;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
}

#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Service ready", body = HealthResponse)
    ),
    tag = "Health"
)]
#[instrument]
pub async fn get_ready() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ready".to_string(),
    })
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service healthy", body = HealthResponse)
    ),
    tag = "Health"
)]
#[instrument]
pub async fn get_health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
    })
}

/// Middleware to inject request ID into extensions for handlers
async fn inject_request_id(
    mut request: Request<Body>,
    next: axum::middleware::Next,
) -> axum::response::Response {
    // Try to get request ID from header, or generate a new one
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or_else(Uuid::new_v4);

    request.extensions_mut().insert(request_id);
    next.run(request).await
}

pub fn create_router(state: Arc<AppState>) -> Router {
    let vouch_public = vouch::public_routes();
    let commit_boost_public = commit_boost::public_routes();

    // Admin routes protected by authentication middleware
    let admin_routes = Router::new()
        .nest("/vouch", vouch::admin_routes())
        .nest("/commit-boost", commit_boost::admin_routes())
        .nest("/tokens", auth::handlers::token_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::middleware::require_auth,
        ));

    Router::new()
        .route("/ready", get(get_ready))
        .route("/health", get(get_health))
        .nest("/vouch", vouch_public)
        .nest("/commit-boost", commit_boost_public)
        .nest("/api/admin", admin_routes)
        .with_state(state)
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", openapi::ApiDoc::openapi()),
        )
        // Add request ID middleware
        .layer(middleware::from_fn(inject_request_id))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
}
