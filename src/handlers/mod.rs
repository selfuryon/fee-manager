// handlers/mod.rs - Main router and health endpoints
use crate::AppState;
use crate::auth;
use crate::errors::ApiError;
use crate::openapi;
use axum::{
    Json, Router, body::Body, extract::State, http::Request, middleware, response::IntoResponse,
    routing::get,
};
use serde::Serialize;
use std::sync::Arc;
use std::time::Duration;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tracing::{instrument, warn};
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
        (status = 200, description = "Service ready", body = HealthResponse),
        (status = 503, description = "Database unavailable", body = crate::errors::ErrorResponse)
    ),
    tag = "Health"
)]
#[instrument(skip(state))]
pub async fn get_ready(
    State(state): State<Arc<AppState>>,
) -> Result<Json<HealthResponse>, ApiError> {
    // Goes through the shared pool on purpose: an exhausted pool cannot serve
    // requests either. The timeout keeps the probe well below the pool's 30s
    // acquire timeout.
    match tokio::time::timeout(READY_TIMEOUT, sqlx::query("SELECT 1").execute(&state.pool)).await {
        Ok(Ok(_)) => Ok(Json(HealthResponse {
            status: "ready".to_string(),
        })),
        Ok(Err(e)) => {
            warn!("Readiness check failed: {e}");
            Err(ApiError::ServiceUnavailable)
        }
        Err(_) => {
            warn!("Readiness check timed out after {READY_TIMEOUT:?}");
            Err(ApiError::ServiceUnavailable)
        }
    }
}

/// How long `/ready` waits for the database
const READY_TIMEOUT: Duration = Duration::from_secs(2);

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
        // JSON error bodies for unknown routes and methods too
        .fallback(|| async { ApiError::NotFound("Route not found".to_string()) })
        .method_not_allowed_fallback(|| async { ApiError::MethodNotAllowed })
        // Add request ID middleware
        .layer(middleware::from_fn(inject_request_id))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(PropagateRequestIdLayer::x_request_id())
}
