// handlers/vouch/execution_config.rs - Public execution config endpoint
use crate::errors::ApiError;
use crate::schema::{ExecutionConfigRequest, ExecutionConfigResponse, ProposerEntry, RelayConfig};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Deserialize)]
pub struct ExecutionConfigQuery {
    pub tags: Option<String>,
}

#[utoipa::path(
    post,
    path = "/vouch/v2/execution-config/{config}",
    params(
        ("config" = String, Path, description = "Default config name"),
        ("tags" = Option<String>, Query, description = "Comma-separated list of tags")
    ),
    request_body = ExecutionConfigRequest,
    responses(
        (status = 200, description = "Execution configuration", body = ExecutionConfigResponse),
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Public"
)]
#[instrument(skip(state))]
pub async fn get_execution_config(
    State(state): State<Arc<AppState>>,
    Path(config_name): Path<String>,
    Query(query): Query<ExecutionConfigQuery>,
    Json(req): Json<ExecutionConfigRequest>,
) -> Result<Json<ExecutionConfigResponse>, ApiError> {
    info!(
        "Getting execution config: {} with tags: {:?}, keys: {}",
        config_name,
        query.tags,
        req.keys.len()
    );

    // 1. Load default config
    let default_config = sqlx::query_as::<_, crate::models::VouchDefaultConfig>(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs WHERE name = $1 AND active = true",
    )
    .bind(&config_name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Default config '{}' not found", config_name)))?;

    // 2. Load default relays
    let default_relays = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(
        "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_default_relays WHERE config_name = $1",
    )
    .bind(&config_name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = default_relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    // 3. Load proposer-specific configs for requested keys
    let mut proposers: Vec<ProposerEntry> = Vec::new();

    if !req.keys.is_empty() {
        let proposer_configs = sqlx::query_as::<_, crate::models::VouchProposer>(
            "SELECT public_key, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
             FROM vouch_proposers WHERE public_key = ANY($1)",
        )
        .bind(&req.keys)
        .fetch_all(&state.pool)
        .await?;

        for proposer in proposer_configs {
            // Load proposer's relays
            let proposer_relays = sqlx::query_as::<_, crate::models::VouchProposerRelay>(
                "SELECT id, proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled
                 FROM vouch_proposer_relays WHERE proposer_public_key = $1 AND disabled = false",
            )
            .bind(&proposer.public_key)
            .fetch_all(&state.pool)
            .await?;

            let proposer_relays_map: HashMap<String, RelayConfig> = proposer_relays
                .into_iter()
                .map(|r| {
                    (
                        r.url.clone(),
                        RelayConfig {
                            public_key: r.public_key,
                            fee_recipient: r.fee_recipient,
                            gas_limit: r.gas_limit,
                            min_value: r.min_value,
                        },
                    )
                })
                .collect();

            proposers.push(ProposerEntry {
                proposer: proposer.public_key.to_string(),
                fee_recipient: proposer.fee_recipient,
                gas_limit: proposer.gas_limit,
                min_value: proposer.min_value,
                reset_relays: if proposer.reset_relays {
                    Some(true)
                } else {
                    None
                },
                relays: if proposer_relays_map.is_empty() {
                    None
                } else {
                    Some(proposer_relays_map)
                },
            });
        }
    }

    // 4. Load pattern-based configs by tags (OR logic)
    if let Some(tags_str) = &query.tags {
        let tags: Vec<&str> = tags_str.split(',').map(|s| s.trim()).collect();

        if !tags.is_empty() {
            let pattern_configs = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
                "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
                 FROM vouch_proposer_patterns WHERE tags && $1",
            )
            .bind(&tags.iter().map(|s| s.to_string()).collect::<Vec<String>>())
            .fetch_all(&state.pool)
            .await?;

            for pattern in pattern_configs {
                // Load pattern's relays
                let pattern_relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
                    "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value
                     FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
                )
                .bind(&pattern.name)
                .fetch_all(&state.pool)
                .await?;

                let pattern_relays_map: HashMap<String, RelayConfig> = pattern_relays
                    .into_iter()
                    .map(|r| (r.url.clone(), r.into()))
                    .collect();

                proposers.push(ProposerEntry {
                    proposer: pattern.pattern,
                    fee_recipient: pattern.fee_recipient,
                    gas_limit: pattern.gas_limit,
                    min_value: pattern.min_value,
                    reset_relays: if pattern.reset_relays {
                        Some(true)
                    } else {
                        None
                    },
                    relays: if pattern_relays_map.is_empty() {
                        None
                    } else {
                        Some(pattern_relays_map)
                    },
                });
            }
        }
    }

    Ok(Json(ExecutionConfigResponse {
        version: 2,
        fee_recipient: default_config.fee_recipient,
        gas_limit: default_config.gas_limit,
        min_value: default_config.min_value,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        proposers: if proposers.is_empty() {
            None
        } else {
            Some(proposers)
        },
    }))
}
