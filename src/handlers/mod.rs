// handlers/mod.rs - Main router and health endpoints
use crate::openapi;
use crate::AppState;
use axum::{response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;

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

pub fn create_router(state: Arc<AppState>) -> Router {
    let vouch_public = vouch::public_routes();
    let commit_boost_public = commit_boost::public_routes();

    let admin_routes = Router::new()
        .nest("/vouch", vouch::admin_routes())
        .nest("/commit-boost", commit_boost::admin_routes());

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
}
