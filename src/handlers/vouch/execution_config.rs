// handlers/vouch/execution_config.rs - Public execution config endpoint
use crate::AppState;
use crate::addresses::BlsPubkey;
use crate::errors::ApiError;
use crate::extract::{AppJson, AppPath, AppQuery};
use crate::models::{
    VouchDefaultConfig, VouchDefaultRelay, VouchProposer, VouchProposerPattern,
    VouchProposerPatternRelay, VouchProposerRelay,
};
use crate::schema::{ExecutionConfigResponse, ProposerEntry, RelayConfig};
use axum::{Json, extract::State};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use tracing::{info, instrument};

#[derive(Debug, Deserialize)]
pub struct ExecutionConfigQuery {
    pub tags: Option<String>,
}

type RelayMap = BTreeMap<String, RelayConfig>;

fn non_empty(relays: Option<RelayMap>) -> Option<RelayMap> {
    relays.filter(|r| !r.is_empty())
}

#[utoipa::path(
    post,
    path = "/vouch/v2/execution-config/{config}",
    params(
        ("config" = String, Path, description = "Default config name"),
        ("tags" = Option<String>, Query, description = "Comma-separated list of tags")
    ),
    request_body = Vec<BlsPubkey>,
    responses(
        (status = 200, description = "Execution configuration", body = ExecutionConfigResponse),
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Public"
)]
#[instrument(skip(state, keys), fields(keys = keys.len()))]
pub async fn get_execution_config(
    State(state): State<Arc<AppState>>,
    AppPath(config_name): AppPath<String>,
    AppQuery(query): AppQuery<ExecutionConfigQuery>,
    AppJson(keys): AppJson<Vec<BlsPubkey>>,
) -> Result<Json<ExecutionConfigResponse>, ApiError> {
    info!(
        "Getting execution config: {} with tags: {:?}, keys: {}",
        config_name,
        query.tags,
        keys.len()
    );

    // Every read below sees one snapshot, so a concurrent admin change is
    // either fully in the response or not at all. Read-only REPEATABLE READ
    // transactions never fail with serialization errors in Postgres.
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;

    // 1. Default config and its relays
    let default_config = sqlx::query_as::<_, VouchDefaultConfig>(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs WHERE name = $1 AND active = true",
    )
    .bind(&config_name)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Default config '{}' not found", config_name)))?;

    let default_relays: RelayMap = sqlx::query_as::<_, VouchDefaultRelay>(
        "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_default_relays WHERE config_name = $1",
    )
    .bind(&config_name)
    .fetch_all(&mut *tx)
    .await?
    .into_iter()
    .map(|r| (r.url.clone(), r.into()))
    .collect();

    let mut proposers: Vec<ProposerEntry> = Vec::new();

    // 2. Proposer configs for the requested keys, ordered by public key, with
    //    all their relays in one query
    if !keys.is_empty() {
        let proposer_configs = sqlx::query_as::<_, VouchProposer>(
            "SELECT public_key, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
             FROM vouch_proposers WHERE public_key = ANY($1) ORDER BY public_key",
        )
        .bind(&keys)
        .fetch_all(&mut *tx)
        .await?;

        let mut relays_by_key: HashMap<String, RelayMap> = HashMap::new();
        if !proposer_configs.is_empty() {
            let found: Vec<&BlsPubkey> = proposer_configs.iter().map(|p| &p.public_key).collect();
            // Disabled relays are included: Vouch handles the flag itself
            let relays = sqlx::query_as::<_, VouchProposerRelay>(
                "SELECT id, proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled
                 FROM vouch_proposer_relays WHERE proposer_public_key = ANY($1)",
            )
            .bind(&found)
            .fetch_all(&mut *tx)
            .await?;
            for r in relays {
                relays_by_key
                    .entry(r.proposer_public_key.to_string())
                    .or_default()
                    .insert(
                        r.url,
                        RelayConfig {
                            public_key: r.public_key,
                            fee_recipient: r.fee_recipient,
                            gas_limit: r.gas_limit,
                            min_value: r.min_value,
                            disabled: r.disabled,
                        },
                    );
            }
        }

        for proposer in proposer_configs {
            let key = proposer.public_key.to_string();
            proposers.push(ProposerEntry {
                relays: non_empty(relays_by_key.remove(&key)),
                proposer: key,
                fee_recipient: proposer.fee_recipient,
                gas_limit: proposer.gas_limit,
                min_value: proposer.min_value,
                reset_relays: proposer.reset_relays.then_some(true),
            });
        }
    }

    // 3. Pattern configs carrying any requested tag (OR logic), ordered by the
    //    position of their first matching tag in the request, ties by name
    if let Some(tags_str) = &query.tags {
        let tags: Vec<String> = tags_str.split(',').map(|s| s.trim().to_string()).collect();

        let mut pattern_configs = sqlx::query_as::<_, VouchProposerPattern>(
            "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
             FROM vouch_proposer_patterns WHERE tags && $1 ORDER BY name",
        )
        .bind(&tags)
        .fetch_all(&mut *tx)
        .await?;

        // Stable sort keeps the name order from the query for equal positions
        pattern_configs.sort_by_key(|p| {
            p.tags
                .iter()
                .filter_map(|t| tags.iter().position(|req_tag| req_tag == t))
                .min()
                .unwrap_or(usize::MAX)
        });

        let mut relays_by_pattern: HashMap<String, RelayMap> = HashMap::new();
        if !pattern_configs.is_empty() {
            let names: Vec<&str> = pattern_configs.iter().map(|p| p.name.as_str()).collect();
            let relays = sqlx::query_as::<_, VouchProposerPatternRelay>(
                "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled
                 FROM vouch_proposer_pattern_relays WHERE pattern_name = ANY($1)",
            )
            .bind(&names)
            .fetch_all(&mut *tx)
            .await?;
            for r in relays {
                relays_by_pattern
                    .entry(r.pattern_name.clone())
                    .or_default()
                    .insert(r.url.clone(), r.into());
            }
        }

        for pattern in pattern_configs {
            proposers.push(ProposerEntry {
                relays: non_empty(relays_by_pattern.remove(&pattern.name)),
                proposer: pattern.pattern,
                fee_recipient: pattern.fee_recipient,
                gas_limit: pattern.gas_limit,
                min_value: pattern.min_value,
                reset_relays: pattern.reset_relays.then_some(true),
            });
        }
    }

    tx.commit().await?;

    Ok(Json(ExecutionConfigResponse {
        version: 2,
        fee_recipient: default_config.fee_recipient,
        gas_limit: default_config.gas_limit,
        min_value: default_config.min_value,
        relays: non_empty(Some(default_relays)),
        proposers: (!proposers.is_empty()).then_some(proposers),
    }))
}
