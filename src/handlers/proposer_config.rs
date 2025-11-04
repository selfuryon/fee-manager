// handlers_proposer.rs
use crate::errors::{ApiError, ErrorResponse};
use crate::models::{DbProposerConfig, DbRelayConfig};
use crate::schema::{ApiProposerConfig, SuccessResponse};
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
    path = "/api/v1/configs/proposer/{proposer_id}",
    params(("proposer_id" = String, Path, description = "Proposer identifier (validator public key or regex pattern)")),
    responses(
        (status = 200, description = "Configuration retrieved successfully", body = ApiProposerConfig, example = json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "0.1",
            "grace": 30,
            "reset_relays": false,
            "default_configs": null,
            "relays": {
                "https://relay1.com/": {
                    "public_key": "0xabcdef1234567890abcdef1234567890abcdef12",
                    "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
                    "gas_limit": "30000000",
                    "min_value": "0.2",
                    "disabled": false
                }
            }
        })),
        (status = 404, description = "Configuration not found", body = ErrorResponse, example = json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "Proposer config 0x1234567890abcdef not found"
            }
        })),
        (status = 500, description = "Internal server error", body = ErrorResponse, example = json!({
            "error": {
                "code": "INTERNAL_ERROR",
                "message": "Failed to fetch proposer config 0x1234567890abcdef"
            }
        }))
    ),
    tag = "Proposer config"
)]
#[tracing::instrument(skip(state))]
pub async fn get_proposer_config(
    State(state): State<Arc<AppState>>,
    Path(proposer_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Fetching proposer config with id: {}", proposer_id);

    let config = sqlx::query!(
        r#"
        SELECT
            proposer as "proposer!: String",
            default_configs,
            fee_recipient,
            gas_limit,
            min_value,
            grace,
            reset_relays,
            relays
        FROM proposer_configs
        WHERE proposer = $1 LIMIT 1
        "#,
        proposer_id
    )
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => {
            ApiError::NotFound(format!("Proposer config {} not found", proposer_id))
        }
        _ => ApiError::DatabaseError(e),
    })?;

    let relays: Option<HashMap<String, DbRelayConfig>> = config
        .relays
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or_else(|| ApiError::InvalidData("Failed to parse relays configuration".to_string()))?;

    let proposer_config = DbProposerConfig {
        proposer: config.proposer,
        default_configs: config.default_configs,
        fee_recipient: config.fee_recipient,
        gas_limit: config.gas_limit,
        min_value: config.min_value,
        grace: config.grace,
        reset_relays: config.reset_relays,
        relays,
    };

    info!("Proposer config {} retrieved successfully", proposer_id);
    let response: ApiProposerConfig = proposer_config.into();
    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/v1/configs/proposer/{proposer_id}",
    params(("proposer_id" = String, Path, description = "Proposer identifier (validator public key or regex pattern)")),
    request_body = ApiProposerConfig,
    responses(
        (status = 201, description = "Config created successfully", body = ApiProposerConfig, example = json!({
            "fee_recipient": "0x1234567890abcdef1234567890abcdef12345678",
            "gas_limit": "30000000",
            "min_value": "0.1",
            "grace": 30,
            "reset_relays": false,
            "default_configs": ["default-standard"],
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
    tag = "Proposer config"
)]
#[tracing::instrument(skip(state))]
pub async fn create_or_update_proposer_config(
    State(state): State<Arc<AppState>>,
    Path(proposer_id): Path<String>,
    Json(proposer_config): Json<ApiProposerConfig>,
) -> impl IntoResponse {
    info!(
        "Attempting to create proposer config with id: {}",
        &proposer_id
    );

    // Convert to JSON for storage
    let config_json = match serde_json::to_value(&proposer_config) {
        Ok(json) => json,
        Err(e) => {
            error!("Failed to serialize config: {:?}", e);
            return (StatusCode::BAD_REQUEST, "Invalid config data").into_response();
        }
    };

    // Check if default_configs exist if specified
    if let Some(default_configs) = &proposer_config.default_configs {
        if !default_configs.is_empty() {
            let count = sqlx::query!(
                r#"
                SELECT COUNT(*) as "count!: i64"
                FROM execution_configs
                WHERE config_type = 'default' AND config_id = ANY($1)
                "#,
                default_configs
            )
            .fetch_one(&state.db)
            .await;

            if let Ok(result) = count {
                if result.count != default_configs.len() as i64 {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(ErrorResponse::bad_request(
                            "One or more default configs do not exist".to_string(),
                        )),
                    )
                        .into_response();
                }
            } else {
                return ErrorResponse::internal_error(
                    "Failed to validate default config references".to_string(),
                )
                .into_response();
            }
        }
    }

    let result = sqlx::query!(
        r#"
        INSERT INTO execution_configs (config_id, config_type, default_configs, config)
        VALUES ($1, 'proposer', $2, $3)
        ON CONFLICT (config_id, config_type)
        DO UPDATE SET default_configs = EXCLUDED.default_configs, config = EXCLUDED.config
        "#,
        proposer_id,
        proposer_config
            .default_configs
            .unwrap_or_default()
            .as_slice(),
        config_json
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(pg_result) => {
            if pg_result.rows_affected() == 0 {
                info!("Proposer config with id {} updated", proposer_id);
                (
                    StatusCode::OK,
                    Json(SuccessResponse {
                        message: format!("Proposer config {} updated", proposer_id),
                    }),
                )
                    .into_response()
            } else {
                info!("Proposer config {} created successfully", proposer_id);
                (
                    StatusCode::CREATED,
                    Json(SuccessResponse {
                        message: format!("Proposer config {} created", proposer_id),
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            error!("Failed to create proposer config {}: {:?}", proposer_id, e);
            ErrorResponse::internal_error(format!(
                "Failed to create proposer config {}",
                proposer_id
            ))
            .into_response()
        }
    }
}

#[utoipa::path(
    delete,
    path = "/api/v1/configs/proposer/{proposer_id}",
    params(("proposer_id" = String, Path, description = "Proposer identifier to delete")),
    responses(
        (status = 200, description = "Config deleted successfully", body = SuccessResponse, example = json!({
            "message": "Proposer config 0x1234567890abcdef deleted successfully"
        })),
        (status = 404, description = "Configuration not found", body = ErrorResponse, example = json!({
            "error": {
                "code": "NOT_FOUND",
                "message": "Proposer config 0x1234567890abcdef not found"
            }
        })),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Proposer config"
)]
#[tracing::instrument(skip(state))]
pub async fn delete_proposer_config(
    State(state): State<Arc<AppState>>,
    Path(proposer_id): Path<String>,
) -> impl IntoResponse {
    info!(
        "Attempting to delete proposer config with id: {}",
        proposer_id
    );

    let result = sqlx::query!(
        r#"
        DELETE FROM execution_configs
        WHERE config_id = $1 AND config_type = 'proposer'
        "#,
        proposer_id
    )
    .execute(&state.db)
    .await;

    match result {
        Ok(pg_result) => {
            if pg_result.rows_affected() == 0 {
                info!("Proposer config {} not found for deletion", proposer_id);
                (
                    StatusCode::NOT_FOUND,
                    Json(ErrorResponse::not_found(format!(
                        "Proposer config {} not found",
                        proposer_id
                    ))),
                )
                    .into_response()
            } else {
                info!("Proposer config {} deleted successfully", proposer_id);
                (
                    StatusCode::OK,
                    Json(SuccessResponse {
                        message: format!("Proposer config {} deleted successfully", proposer_id),
                    }),
                )
                    .into_response()
            }
        }
        Err(e) => {
            error!("Failed to delete proposer config {}: {:?}", proposer_id, e);
            ErrorResponse::internal_error(format!(
                "Failed to delete proposer config {}",
                proposer_id
            ))
            .into_response()
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/configs/proposers",
    responses(
        (status = 200, description = "Proposer configs retrieved successfully", body = Vec<String>),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    ),
    tag = "Proposer config"
)]
#[tracing::instrument(skip(state))]
pub async fn list_proposer_configs(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Listing all proposer configs");

    let configs = sqlx::query!(
        r#"
        SELECT config_id as "config_id!: String"
        FROM execution_configs
        WHERE config_type = 'proposer'
        ORDER BY config_id
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::DatabaseError)?;

    let config_ids: Vec<String> = configs.into_iter().map(|row| row.config_id).collect();

    info!("Retrieved {} proposer configs", config_ids.len());
    Ok((StatusCode::OK, Json(config_ids)))
}
