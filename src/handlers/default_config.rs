// handlers.rs
use crate::errors::ErrorResponse;
use crate::models::{DbDefaultConfig, DbRelayConfig, DefaultConfigResponse};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};

#[derive(Deserialize, Serialize, Debug)]
pub struct DefaultConfig {
    config_id: String,
    config: serde_json::Value, // JSONB field for the config
}

pub async fn get_default_config(
    State(state): State<Arc<AppState>>,
    Path(config_id): Path<String>,
) -> impl IntoResponse {
    info!("Fetching default config with id: {}", config_id);
    let result = sqlx::query!(
        r#"
        SELECT
            config_id as "config_id!: String",
            fee_recipient,
            gas_limit,
            min_value,
            grace,
            relays
        FROM default_configs
        WHERE config_id = $1 LIMIT 1
        "#,
        config_id
    )
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(config) => {
            info!("Default config {} retrieved successfully", config_id);
            let relays: Option<HashMap<String, DbRelayConfig>> =
                config.relays.and_then(|v| serde_json::from_value(v).ok());
            let default_config = DbDefaultConfig {
                config_id: config.config_id,
                fee_recipient: config.fee_recipient,
                gas_limit: config.gas_limit,
                min_value: config.min_value,
                grace: config.grace,
                relays,
            };
            let response: DefaultConfigResponse = default_config.into();
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => match e {
            sqlx::Error::RowNotFound => {
                error!("Default config not found: {}", config_id);
                ErrorResponse::not_found(format!("Default config {} not found", config_id))
                    .into_response()
            }
            _ => {
                error!("Failed to fetch default config {}: {:?}", config_id, e);
                ErrorResponse::internal_error(format!(
                    "Failed to fetch default config {}",
                    config_id
                ))
                .into_response()
            }
        },
    }
}
