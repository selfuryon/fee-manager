// handlers.rs
use crate::errors::{ApiError, ErrorResponse};
use crate::models::{DbDefaultConfig, DbRelayConfig};
use crate::schema::{ApiDefaultConfig, SuccessResponse};
use crate::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::{collections::HashMap, sync::Arc};
use tracing::{error, info};

#[utoipa::path(
    get,
    path = "/api/v1/configs/default/{config_id}",
    params(("config_id" = String, Path, description = "Default config identifier")),
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = ApiDefaultConfig, example = json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "0.1",
            "grace": 30,
            "relays": {
                "https://relay1.com/": {
                    "public_key": "0xabcdef1234567890abcdef1234567890abcdef12",
                    "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
                    "gas_limit": "30000000",
                    "min_value": "0.2"
                }
            }
        })),
        (status = 404, description = "Configuration not found", body = ErrorResponse, example = json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "Default config test123 not found"
            }
        })),
        (status = 500, description = "Internal server error", body = ErrorResponse, example = json!({
            "error": {
                "code": "INTERNAL_ERROR",
                "message": "Failed to fetch default config test123"
            }
        }))
    ),
    tag = "Default config"
)]
#[tracing::instrument(skip(state))]
pub async fn get_default_config(
    State(state): State<Arc<AppState>>,
    Path(config_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Fetching default config with id: {}", config_id);

    let config = sqlx::query!(
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
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::NotFound(format!("Default config {} not found", config_id))
        }
        _ => ApiError::DatabaseError(e),
    })?;

    let relays: Option<HashMap<String, DbRelayConfig>> = config
        .relays
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or_else(|| ApiError::InvalidData("Failed to parse relays configuration".to_string()))?;

    let default_config = DbDefaultConfig {
        config_id: config.config_id,
        fee_recipient: config.fee_recipient,
        gas_limit: config.gas_limit,
        min_value: config.min_value,
        grace: config.grace,
        relays,
    };

    info!("Default config {} retrieved successfully", config_id);
    let response: ApiDefaultConfig = default_config.into();
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/v1/configs/default/{config_id}",
    params(("config_id" = String, Path, description = "Default config identifier")),
    request_body = ApiDefaultConfig,
    responses(
        (status = 201, description = "Config created successfully", body = ApiDefaultConfig, example = json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "0.1",
            "grace": 30,
            "relays": {
                "https://relay1.com/": {
                    "public_key": "0xabcdef1234567890abcdef1234567890abcdef12",
                    "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
                    "gas_limit": "30000000",
                    "min_value": "0.2"
                }
            }
        })),
        (status = 409, description = "Config already exists", body = ErrorResponse),
        (status = 500, description = "Internal Server Error", body = ErrorResponse)
    ),
    tag = "Default config"
)]
#[tracing::instrument(skip(state))]
pub async fn create_or_update_default_config(
    State(state): State<Arc<AppState>>,
    Path(config_id): Path<String>,
    Json(default_config): Json<ApiDefaultConfig>,
) -> impl IntoResponse {
    info!(
        "Attempting to create default config with id: {}",
        &config_id
    );

    let config_json = match serde_json::to_value(&default_config) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize config: {:?}", e);
            return (StatusCode::BAD_REQUEST, "Invalid config data").into_response();
        }
    };

    let result = sqlx::query!(
        r#"
        INSERT INTO execution_configs (config_id, config_type, config)
        VALUES ($1, 'default', $2)
        ON CONFLICT (config_id, config_type)
        DO UPDATE SET config = EXCLUDED.config
        "#,
        config_id,
        config_json
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(pg_result) => {
            if pg_result.rows_affected() == 0 {
                info!("Default config with id {} updated", config_id);
                (
                    StatusCode::OK,
                    Json(SuccessResponse {
                        message: format!("Default config {} updated ", config_id),
                    }),
                )
                    .into_response()
            } else {
                info!("Default config {} created successfully", config_id);
                (
                    StatusCode::CREATED,
                    Json(SuccessResponse {
                        message: format!("Default config {} created", config_id),
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            error!("Failed to create default config {}: {:?}", config_id, e);
            ErrorResponse::internal_error(format!("Failed to create default config {}", config_id))
                .into_response()
        }
    }
}
