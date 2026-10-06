// handlers/vouch/proposer_patterns.rs - Proposer Pattern CRUD handlers
use crate::AppState;
use crate::audit::{AuditAction, AuditChanges, RequestContext, ResourceType};
use crate::audit_log;
use crate::errors::ApiError;
use crate::schema::{
    CreateProposerPatternRequest, PaginatedResponse, ProposerPatternListItem,
    ProposerPatternResponse, ProposerRelayConfig, UpdateProposerPatternRequest,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use sqlx::{Postgres, QueryBuilder};
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
    /// Filter by relay URL (prefix match)
    pub relay_url: Option<String>,
    /// Filter by relay min_value (exact match)
    pub relay_min_value: Option<String>,
    /// Filter by relay disabled status
    pub relay_disabled: Option<bool>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Appends the WHERE clause for `filters`; every value is a bind parameter.
fn push_filters(qb: &mut QueryBuilder<Postgres>, filters: &ProposerPatternFilters) {
    qb.push(" WHERE TRUE");
    if let Some(ref name) = filters.name {
        qb.push(" AND p.name LIKE ").push_bind(format!("{name}%"));
    }
    if let Some(ref pattern) = filters.pattern {
        qb.push(" AND p.pattern LIKE ")
            .push_bind(format!("%{pattern}%"));
    }
    if let Some(ref tag) = filters.tag {
        qb.push(" AND ")
            .push_bind(tag.clone())
            .push(" = ANY(p.tags)");
    }
    if let Some(ref fr) = filters.fee_recipient {
        qb.push(" AND p.fee_recipient = ").push_bind(fr.clone());
    }
    if let Some(ref gl) = filters.gas_limit {
        qb.push(" AND p.gas_limit = ").push_bind(gl.clone());
    }
    if let Some(ref mv) = filters.min_value {
        qb.push(" AND p.min_value = ").push_bind(mv.clone());
    }
    if let Some(rr) = filters.reset_relays {
        qb.push(" AND p.reset_relays = ").push_bind(rr);
    }
    // Relay filters using EXISTS subquery
    if let Some(ref relay_url) = filters.relay_url {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_pattern_relays r WHERE r.pattern_name = p.name AND r.url LIKE ")
            .push_bind(format!("{relay_url}%"))
            .push(")");
    }
    if let Some(ref relay_min_value) = filters.relay_min_value {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_pattern_relays r WHERE r.pattern_name = p.name AND r.min_value = ")
            .push_bind(relay_min_value.clone())
            .push(")");
    }
    if let Some(relay_disabled) = filters.relay_disabled {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_pattern_relays r WHERE r.pattern_name = p.name AND r.disabled = ")
            .push_bind(relay_disabled)
            .push(")");
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/proposer-patterns",
    params(ProposerPatternFilters),
    responses(
        (status = 200, description = "List of proposer patterns", body = PaginatedResponse<ProposerPatternListItem>)
    ),
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_proposer_patterns(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<ProposerPatternFilters>,
) -> Result<Json<PaginatedResponse<ProposerPatternListItem>>, ApiError> {
    info!("Listing proposer patterns with filters: {:?}", filters);

    let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM vouch_proposer_patterns p");
    push_filters(&mut count_query, &filters);
    let total: i64 = count_query
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await?;

    let mut data_query = QueryBuilder::new(
        "SELECT p.name, p.pattern, p.tags, p.fee_recipient, p.gas_limit, p.min_value, p.reset_relays, p.created_at, p.updated_at
         FROM vouch_proposer_patterns p",
    );
    push_filters(&mut data_query, &filters);
    data_query
        .push(" ORDER BY p.name ASC LIMIT ")
        .push_bind(filters.limit)
        .push(" OFFSET ")
        .push_bind(filters.offset);

    let patterns = data_query
        .build_query_as::<crate::models::VouchProposerPattern>()
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
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
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
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, ProposerRelayConfig> = relays
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
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn create_proposer_pattern(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
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
                 (pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&req.name)
            .bind(url)
            .bind(&relay.public_key)
            .bind(&relay.fee_recipient)
            .bind(&relay.gas_limit)
            .bind(&relay.min_value)
            .bind(relay.disabled)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            pattern: Some(req.pattern.clone()),
            tags: Some(req.tags.clone()),
            fee_recipient: req.fee_recipient.as_ref().map(|a| a.to_string()),
            min_value: req.min_value.clone(),
            gas_limit: req.gas_limit.clone(),
            reset_relays: Some(req.reset_relays),
            relays_count: req.relays.as_ref().map(|r| r.len()),
            ..Default::default()
        };
        audit_log!(
            ctx,
            AuditAction::Create,
            ResourceType::VouchProposerPattern,
            &req.name,
            changes
        );
    }

    // Fetch created pattern
    let pattern = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&req.name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, ProposerRelayConfig> = relays
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
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn update_proposer_pattern(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
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

    // Omitted fields keep their current value
    let has_updates = req.pattern.is_some()
        || req.tags.is_some()
        || req.fee_recipient.is_some()
        || req.gas_limit.is_some()
        || req.min_value.is_some()
        || req.reset_relays.is_some();

    if has_updates {
        sqlx::query(
            "UPDATE vouch_proposer_patterns SET
                pattern = COALESCE($2, pattern),
                tags = COALESCE($3, tags),
                fee_recipient = COALESCE($4, fee_recipient),
                gas_limit = COALESCE($5, gas_limit),
                min_value = COALESCE($6, min_value),
                reset_relays = COALESCE($7, reset_relays)
             WHERE name = $1",
        )
        .bind(&name)
        .bind(&req.pattern)
        .bind(&req.tags)
        .bind(&req.fee_recipient)
        .bind(&req.gas_limit)
        .bind(&req.min_value)
        .bind(req.reset_relays)
        .execute(&mut *tx)
        .await?;
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
                 (pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&name)
            .bind(url)
            .bind(&relay.public_key)
            .bind(&relay.fee_recipient)
            .bind(&relay.gas_limit)
            .bind(&relay.min_value)
            .bind(relay.disabled)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            pattern: req.pattern.clone(),
            tags: req.tags.clone(),
            fee_recipient: req.fee_recipient.as_ref().map(|a| a.to_string()),
            min_value: req.min_value.clone(),
            gas_limit: req.gas_limit.clone(),
            reset_relays: req.reset_relays,
            relays_count: req.relays.as_ref().map(|r| r.len()),
            ..Default::default()
        };
        audit_log!(
            ctx,
            AuditAction::Update,
            ResourceType::VouchProposerPattern,
            &name,
            changes
        );
    }

    // Fetch updated pattern
    let pattern = sqlx::query_as::<_, crate::models::VouchProposerPattern>(
        "SELECT name, pattern, tags, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerPatternRelay>(
        "SELECT id, pattern_name, url, public_key, fee_recipient, gas_limit, min_value, disabled
         FROM vouch_proposer_pattern_relays WHERE pattern_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, ProposerRelayConfig> = relays
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
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn delete_proposer_pattern(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
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

    // Audit log
    if state.config.audit_enabled {
        audit_log!(
            ctx,
            AuditAction::Delete,
            ResourceType::VouchProposerPattern,
            &name
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
