// handlers.rs
use axum::{response::IntoResponse, routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

pub mod default_config;

use crate::AppState;
use default_config::get_default_config;

#[tracing::instrument]
pub async fn get_live() -> impl IntoResponse {
    Json(json!({ "service": "ok" }))
}

#[tracing::instrument]
pub async fn get_ready() -> impl IntoResponse {
    Json(json!({ "service": "ok" }))
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/ready", get(get_ready))
        .route("/live", get(get_live))
        .route(
            "/api/v1/configs/default/{config_id}",
            get(get_default_config),
        )
        .with_state(state)
}
