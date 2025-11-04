// handlers.rs
use crate::openapi;
use crate::AppState;
use axum::{response::IntoResponse, routing::get, Json, Router};
use default_config::{create_or_update_default_config, get_default_config};
use serde::Serialize;
use std::sync::Arc;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;

pub mod default_config;
pub mod proposer_config;

#[derive(Serialize, ToSchema)]
pub struct HealthResponse {
    pub service: String,
}

#[utoipa::path(
    get, path = "/ready",
    responses(
        (status = 200, description = "Service ready", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn get_ready() -> impl IntoResponse {
    Json(HealthResponse {
        service: "ready".to_string(),
    })
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ready", get(get_ready))
        .route(
            "/api/v1/configs/default/{config_id}",
            get(get_default_config).put(create_or_update_default_config),
        )
        .with_state(state)
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", openapi::ApiDoc::openapi()),
        )
}
