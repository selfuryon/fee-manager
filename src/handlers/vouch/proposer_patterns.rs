// handlers/vouch/proposer_patterns.rs - Proposer Pattern CRUD handlers
use crate::errors::ApiError;
use crate::schema::{
    CreateProposerPatternRequest, PaginatedResponse, ProposerPatternListItem,
    ProposerPatternResponse, RelayConfig, UpdateProposerPatternRequest,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ProposerPatternFilters {
    pub name: Option<String>,
    pub pattern: Option<String>,
    pub tag: Option<String>,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub reset_relays: Option<bool>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/proposer-patterns",
    params(ProposerPatternFilters),
    responses(
        (status = 200, description = "List of proposer patterns", body = PaginatedResponse<ProposerPatternListItem>)
    ),
    tag = "Vouch - Proposer Patterns"
)]
#[instrument(skip(state))]
pub async fn list_proposer_patterns(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<ProposerPatternFilters>,
) -> Result<Json<PaginatedResponse<ProposerPatternListItem>>, ApiError> {
    info!("Listing proposer patterns with filters: {:?}", filters);

    let mut conditions = Vec::new();

    if let Some(ref name) = filters.name {
        conditions.push(format!("name LIKE '{}%'", name.replace('\'', "''")));
    }
    if let Some(ref pattern) = filters.pattern {
        conditions.push(format!("pattern LIKE '%{}%'", pattern.replace('\'', "''")));
    }
    if let Some(ref tag) = filters.tag {
        conditions.push(format!("'{}' = ANY(tags)", tag.replace('\'', "''")));
    }
    if let Some(ref fr) = filters.fee_recipient {
        conditions.push(format!("fee_recipient = '{}'", fr.replace('\'', "''")));
    }
    if let Some(ref gl) = filters.gas_limit {
        conditions.push(format!("gas_limit = '{}'", gl.replace('\'', "''")));
    }
    if let Some(ref mv) = filters.min_value {
        conditions.push(format!("min_value = '{}'", mv.replace('\'', "''")));
    }
    if let Some(rr) = filters.reset_relays {
        conditions.push(format!(
            "reset_relays = {}",
            if rr { "true" } else { "false" }
        ));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) as count FROM vouch_proposer_patterns {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns {}
         ORDER BY name ASC
         LIMIT {} OFFSET {}",
        where_clause, filters.limit, filters.offset
    );

    let patterns = sqlx::query_as::<_, crate::models::VouchProposerPattern>(&data_sql)
        .fetch_all(&state.pool)
        .await?;

    let data: Vec<ProposerPatternListItem> = patterns.into_iter().map(Into::into).collect();

    Ok(Json(PaginatedResponse {
        data,
        total,
        limit: filters.limit,
        offset: filters.offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/proposer-patterns/{name}",
    params(
        ("name" = String, Path, description = "Pattern name")
    ),
    responses(
        (status = 200, description = "Proposer pattern details", body = ProposerPatternResponse),
        (status = 404, description = "Pattern not found")
    ),
    tag = "Vouch - Proposer Patterns"
)]
#[instrument(skip(state))]
pub async fn get_proposer_pattern(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<ProposerPatternResponse>, ApiError> {
    info!("Getting proposer pattern: {}", name);

    let pattern = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Proposer pattern '{}' not found", name)))?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    Ok(Json(ProposerPatternResponse {
        name: pattern.name,
        pattern: pattern.pattern,
        tags: pattern.tags,
        fee_recipient: pattern.fee_recipient,
        gas_limit: pattern.gas_limit,
        min_value: pattern.min_value,
        reset_relays: pattern.reset_relays,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: pattern.created_at,
        updated_at: pattern.updated_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/vouch/proposer-patterns",
    request_body = CreateProposerPatternRequest,
    responses(
        (status = 201, description = "Pattern created", body = ProposerPatternResponse),
        (status = 409, description = "Pattern already exists")
    ),
    tag = "Vouch - Proposer Patterns"
)]
#[instrument(skip(state))]
pub async fn create_proposer_pattern(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProposerPatternRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating proposer pattern: {}", req.name);

    let mut tx = state.pool.begin().await?;

    // Check if pattern already exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Err(ApiError::InvalidData(format!(
            "Pattern '{}' already exists",
            req.name
        )));
    }

    sqlx::query(
        "INSERT INTO vouch_proposer_patterns (name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&req.name)
    .bind(&req.pattern)
    .bind(&req.tags)
    .bind(&req.fee_recipient)
    .bind(&req.gas_limit)
    .bind(&req.min_value)
    .bind(req.reset_relays)
    .execute(&mut *tx)
    .await?;

    if let Some(relays) = &req.relays {
        for (url, relay) in relays {
            sqlx::query(
                "INSERT INTO vouch_proposer_pattern_relays
                 (pattern_name, url, public_key, fee_recipient, gas_limit, min_value)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&req.name)
            .bind(url)
            .bind(&relay.public_key)
            .bind(&relay.fee_recipient)
            .bind(&relay.gas_limit)
            .bind(&relay.min_value)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // Fetch created pattern
    let pattern = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&req.name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    let response = ProposerPatternResponse {
        name: pattern.name,
        pattern: pattern.pattern,
        tags: pattern.tags,
        fee_recipient: pattern.fee_recipient,
        gas_limit: pattern.gas_limit,
        min_value: pattern.min_value,
        reset_relays: pattern.reset_relays,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: pattern.created_at,
        updated_at: pattern.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/admin/vouch/proposer-patterns/{name}",
    params(
        ("name" = String, Path, description = "Pattern name")
    ),
    request_body = UpdateProposerPatternRequest,
    responses(
        (status = 200, description = "Pattern updated", body = ProposerPatternResponse),
        (status = 404, description = "Pattern not found")
    ),
    tag = "Vouch - Proposer Patterns"
)]
#[instrument(skip(state))]
pub async fn update_proposer_pattern(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateProposerPatternRequest>,
) -> Result<Json<ProposerPatternResponse>, ApiError> {
    info!("Updating proposer pattern: {}", name);

    let mut tx = state.pool.begin().await?;

    // Check if pattern exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Proposer pattern '{}' not found",
            name
        )));
    }

    // Build update query dynamically
    let mut set_clauses = Vec::new();
    let mut param_index = 2;

    if req.pattern.is_some() {
        set_clauses.push(format!("pattern = ${}", param_index));
        param_index += 1;
    }
    if req.tags.is_some() {
        set_clauses.push(format!("tags = ${}", param_index));
        param_index += 1;
    }
    if req.fee_recipient.is_some() {
        set_clauses.push(format!("fee_recipient = ${}", param_index));
        param_index += 1;
    }
    if req.gas_limit.is_some() {
        set_clauses.push(format!("gas_limit = ${}", param_index));
        param_index += 1;
    }
    if req.min_value.is_some() {
        set_clauses.push(format!("min_value = ${}", param_index));
        param_index += 1;
    }
    if req.reset_relays.is_some() {
        set_clauses.push(format!("reset_relays = ${}", param_index));
    }

    if !set_clauses.is_empty() {
        let update_sql = format!(
            "UPDATE vouch_proposer_patterns SET {} WHERE name = $1",
            set_clauses.join(", ")
        );

        let mut query = sqlx::query(&update_sql).bind(&name);

        if let Some(ref p) = req.pattern {
            query = query.bind(p);
        }
        if let Some(ref t) = req.tags {
            query = query.bind(t);
        }
        if let Some(ref fr) = req.fee_recipient {
            query = query.bind(fr);
        }
        if let Some(ref gl) = req.gas_limit {
            query = query.bind(gl);
        }
        if let Some(ref mv) = req.min_value {
            query = query.bind(mv);
        }
        if let Some(rr) = req.reset_relays {
            query = query.bind(rr);
        }

        query.execute(&mut *tx).await?;
    }

    // Handle relays if provided
    if let Some(relays) = &req.relays {
        sqlx::query("DELETE FROM vouch_proposer_pattern_relays WHERE pattern_name = $1")
            .bind(&name)
            .execute(&mut *tx)
            .await?;

        for (url, relay) in relays {
            sqlx::query(
                "INSERT INTO vouch_proposer_pattern_relays
                 (pattern_name, url, public_key, fee_recipient, gas_limit, min_value)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&name)
            .bind(url)
            .bind(&relay.public_key)
            .bind(&relay.fee_recipient)
            .bind(&relay.gas_limit)
            .bind(&relay.min_value)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // Fetch updated pattern
    let pattern = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    Ok(Json(ProposerPatternResponse {
        name: pattern.name,
        pattern: pattern.pattern,
        tags: pattern.tags,
        fee_recipient: pattern.fee_recipient,
        gas_limit: pattern.gas_limit,
        min_value: pattern.min_value,
        reset_relays: pattern.reset_relays,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: pattern.created_at,
        updated_at: pattern.updated_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/vouch/proposer-patterns/{name}",
    params(
        ("name" = String, Path, description = "Pattern name")
    ),
    responses(
        (status = 204, description = "Pattern deleted"),
        (status = 404, description = "Pattern not found")
    ),
    tag = "Vouch - Proposer Patterns"
)]
#[instrument(skip(state))]
pub async fn delete_proposer_pattern(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Deleting proposer pattern: {}", name);

    let result = sqlx::query("DELETE FROM vouch_proposer_patterns WHERE name = $1")
        .bind(&name)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Proposer pattern '{}' not found",
            name
        )));
    }

    Ok(StatusCode::NO_CONTENT)
}
