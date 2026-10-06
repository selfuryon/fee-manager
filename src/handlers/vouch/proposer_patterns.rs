// handlers/vouch/proposer_patterns.rs - Proposer Pattern CRUD handlers
use crate::AppState;
use crate::audit::{AuditAction, AuditChanges, RequestContext, ResourceType};
use crate::audit_log;
use crate::errors::ApiError;
use crate::extract::{AppJson, AppPath, AppQuery};
use crate::pagination::{ListResponse, check_limit};
use crate::schema::{
    CreateProposerPatternRequest, ProposerPatternListItem, ProposerPatternResponse,
    ProposerRelayConfig, UpdateProposerPatternRequest,
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use sqlx::{Postgres, QueryBuilder};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, instrument};
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
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
    /// Maximum number of items to return (1-1000, default 100)
    #[serde(default = "crate::pagination::default_limit")]
    pub limit: i64,
    /// Opaque cursor: the `next_cursor` of the previous page
    pub after: Option<String>,
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
        (status = 200, description = "List of proposer patterns", body = ListResponse<ProposerPatternListItem>)
    ),
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_proposer_patterns(
    State(state): State<Arc<AppState>>,
    AppQuery(filters): AppQuery<ProposerPatternFilters>,
) -> Result<Json<ListResponse<ProposerPatternListItem>>, ApiError> {
    info!("Listing proposer patterns with filters: {:?}", filters);

    check_limit(filters.limit)?;

    let mut data_query = QueryBuilder::new(
        "SELECT p.name, p.pattern, p.tags, p.fee_recipient, p.gas_limit, p.min_value, p.reset_relays, p.created_at, p.updated_at
         FROM vouch_proposer_patterns p",
    );
    push_filters(&mut data_query, &filters);
    if let Some(ref after) = filters.after {
        data_query.push(" AND p.name > ").push_bind(after.clone());
    }
    data_query
        .push(" ORDER BY p.name LIMIT ")
        .push_bind(filters.limit);

    let patterns = data_query
        .build_query_as::<crate::models::VouchProposerPattern>()
        .fetch_all(&state.pool)
        .await?;

    let data: Vec<ProposerPatternListItem> = patterns.into_iter().map(Into::into).collect();

    Ok(Json(ListResponse::new(data, filters.limit, |item| {
        item.name.to_string()
    })))
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
    AppPath(name): AppPath<String>,
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
        (status = 400, description = "Invalid field value", body = crate::errors::ErrorResponse),
        (status = 409, description = "Pattern already exists")
    ),
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn create_proposer_pattern(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    AppJson(req): AppJson<CreateProposerPatternRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating proposer pattern: {}", req.name);

    req.validate()?;

    let mut tx = state.pool.begin().await?;

    // Check if pattern already exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM vouch_proposer_patterns WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Err(ApiError::Conflict(format!(
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
        (status = 400, description = "Invalid field value", body = crate::errors::ErrorResponse),
        (status = 404, description = "Pattern not found")
    ),
    tag = "Vouch - Proposer Patterns",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn update_proposer_pattern(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    AppPath(name): AppPath<String>,
    AppJson(req): AppJson<UpdateProposerPatternRequest>,
) -> Result<Json<ProposerPatternResponse>, ApiError> {
    info!("Updating proposer pattern: {}", name);

    req.validate()?;

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

    // Full replacement: every column comes from the request, and the relay
    // set is replaced by the request's (none when omitted)
    sqlx::query(
        "UPDATE vouch_proposer_patterns
         SET pattern = $2, tags = $3, fee_recipient = $4, gas_limit = $5,
             min_value = $6, reset_relays = $7
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

    sqlx::query("DELETE FROM vouch_proposer_pattern_relays WHERE pattern_name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    if let Some(relays) = &req.relays {
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
    AppPath(name): AppPath<String>,
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
