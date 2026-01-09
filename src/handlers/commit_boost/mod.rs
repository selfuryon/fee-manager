// handlers/commit_boost/mod.rs - Commit-Boost routes
use crate::AppState;
use axum::{routing::{get, post}, Router};
use std::sync::Arc;

pub mod mux;

/// Public routes for Commit-Boost (no authentication)
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new().route("/v1/mux/{name}", get(mux::get_mux_keys_public))
}

/// Admin routes for Commit-Boost (authentication required)
pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/mux", get(mux::list_mux_configs).post(mux::create_mux_config))
        .route(
            "/mux/{name}",
            get(mux::get_mux_config)
                .put(mux::update_mux_config)
                .delete(mux::delete_mux_config),
        )
        .route(
            "/mux/{name}/keys",
            post(mux::add_mux_keys).delete(mux::remove_mux_keys),
        )
}
