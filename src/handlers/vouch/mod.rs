// handlers/vouch/mod.rs - Vouch routes
use crate::AppState;
use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use std::sync::Arc;

/// Body limit of the execution-config endpoint
pub const EXECUTION_CONFIG_BODY_LIMIT: usize = 10 * 1024 * 1024;

pub mod default_configs;
pub mod execution_config;
pub mod proposer_patterns;
pub mod proposers;

/// Public routes for Vouch (no authentication)
pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/v2/execution-config/{config}",
            post(execution_config::get_execution_config),
        )
        // Vouch sends all of its keys in one request: ~101 bytes per key, so
        // 10 MB is about 100k keys. Admin routes keep the 2 MB default.
        .layer(DefaultBodyLimit::max(EXECUTION_CONFIG_BODY_LIMIT))
}

/// Admin routes for Vouch (authentication required)
pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        // Proposers
        .route("/proposers", get(proposers::list_proposers))
        .route(
            "/proposers/{public_key}",
            get(proposers::get_proposer)
                .put(proposers::create_or_update_proposer)
                .delete(proposers::delete_proposer),
        )
        // Default Configs
        .route(
            "/configs/default",
            get(default_configs::list_default_configs).post(default_configs::create_default_config),
        )
        .route(
            "/configs/default/{name}",
            get(default_configs::get_default_config)
                .put(default_configs::update_default_config)
                .delete(default_configs::delete_default_config),
        )
        // Proposer Patterns
        .route(
            "/proposer-patterns",
            get(proposer_patterns::list_proposer_patterns)
                .post(proposer_patterns::create_proposer_pattern),
        )
        .route(
            "/proposer-patterns/{name}",
            get(proposer_patterns::get_proposer_pattern)
                .put(proposer_patterns::update_proposer_pattern)
                .delete(proposer_patterns::delete_proposer_pattern),
        )
}
