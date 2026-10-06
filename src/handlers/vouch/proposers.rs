// handlers/vouch/proposers.rs - Proposer CRUD handlers
use crate::AppState;
use crate::audit::{AuditAction, AuditChanges, RequestContext, ResourceType};
use crate::audit_log;
use crate::errors::ApiError;
use crate::schema::{
    CreateOrUpdateProposerRequest, PaginatedResponse, ProposerListItem, ProposerRelayConfig,
    ProposerResponse,
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
pub struct ProposerFilters {
    pub public_key: Option<String>,
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
fn push_filters(qb: &mut QueryBuilder<Postgres>, filters: &ProposerFilters) {
    qb.push(" WHERE TRUE");
    if let Some(ref pk) = filters.public_key {
        qb.push(" AND p.public_key LIKE ")
            .push_bind(format!("{pk}%"));
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
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_relays r WHERE r.proposer_public_key = p.public_key AND r.url LIKE ")
            .push_bind(format!("{relay_url}%"))
            .push(")");
    }
    if let Some(ref relay_min_value) = filters.relay_min_value {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_relays r WHERE r.proposer_public_key = p.public_key AND r.min_value = ")
            .push_bind(relay_min_value.clone())
            .push(")");
    }
    if let Some(relay_disabled) = filters.relay_disabled {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_proposer_relays r WHERE r.proposer_public_key = p.public_key AND r.disabled = ")
            .push_bind(relay_disabled)
            .push(")");
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/proposers",
    params(ProposerFilters),
    responses(
        (status = 200, description = "List of proposers", body = PaginatedResponse<ProposerListItem>)
    ),
    tag = "Vouch - Proposers",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_proposers(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<ProposerFilters>,
) -> Result<Json<PaginatedResponse<ProposerListItem>>, ApiError> {
    info!("Listing proposers with filters: {:?}", filters);

    let mut count_query = QueryBuilder::new("SELECT COUNT(*) FROM vouch_proposers p");
    push_filters(&mut count_query, &filters);
    let total: i64 = count_query
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await?;

    let mut data_query = QueryBuilder::new(
        "SELECT p.public_key, p.fee_recipient, p.gas_limit, p.min_value, p.reset_relays, p.created_at, p.updated_at
         FROM vouch_proposers p",
    );
    push_filters(&mut data_query, &filters);
    data_query
        .push(" ORDER BY p.created_at DESC LIMIT ")
        .push_bind(filters.limit)
        .push(" OFFSET ")
        .push_bind(filters.offset);

    let proposers = data_query
        .build_query_as::<crate::models::VouchProposer>()
        .fetch_all(&state.pool)
        .await?;

    // Fetch relays for all proposers in the result
    let pubkeys: Vec<String> = proposers.iter().map(|p| p.public_key.to_string()).collect();
    let relays_map = if !pubkeys.is_empty() {
        let all_relays = sqlx::query_as::<_, crate::models::VouchProposerRelay>(
            "SELECT id, proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled
             FROM vouch_proposer_relays WHERE proposer_public_key = ANY($1)",
        )
        .bind(&pubkeys)
        .fetch_all(&state.pool)
        .await?;

        // Group relays by proposer_public_key
        let mut map: HashMap<String, HashMap<String, ProposerRelayConfig>> = HashMap::new();
        for relay in all_relays {
            map.entry(relay.proposer_public_key.to_string())
                .or_default()
                .insert(relay.url.clone(), relay.into());
        }
        map
    } else {
        HashMap::new()
    };

    let data: Vec<ProposerListItem> = proposers
        .into_iter()
        .map(|p| {
            let relays = relays_map.get(&p.public_key.to_string()).cloned();
            let mut item: ProposerListItem = p.into();
            item.relays = relays;
            item
        })
        .collect();

    Ok(Json(PaginatedResponse {
        data,
        total,
        limit: filters.limit,
        offset: filters.offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/proposers/{public_key}",
    params(
        ("public_key" = String, Path, description = "Proposer public key")
    ),
    responses(
        (status = 200, description = "Proposer details", body = ProposerResponse),
        (status = 404, description = "Proposer not found")
    ),
    tag = "Vouch - Proposers",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_proposer(
    State(state): State<Arc<AppState>>,
    Path(public_key): Path<String>,
) -> Result<Json<ProposerResponse>, ApiError> {
    info!("Getting proposer: {}", public_key);

    let proposer = sqlx::query_as::<_, crate::models::VouchProposer>(
        "SELECT public_key, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposers WHERE public_key = $1",
    )
    .bind(&public_key)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Proposer '{}' not found", public_key)))?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerRelay>(
        "SELECT id, proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled
         FROM vouch_proposer_relays WHERE proposer_public_key = $1",
    )
    .bind(&public_key)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, ProposerRelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    Ok(Json(ProposerResponse {
        public_key: proposer.public_key,
        fee_recipient: proposer.fee_recipient,
        gas_limit: proposer.gas_limit,
        min_value: proposer.min_value,
        reset_relays: proposer.reset_relays,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: proposer.created_at,
        updated_at: proposer.updated_at,
    }))
}

#[utoipa::path(
    put,
    path = "/api/admin/vouch/proposers/{public_key}",
    params(
        ("public_key" = String, Path, description = "Proposer public key")
    ),
    request_body = CreateOrUpdateProposerRequest,
    responses(
        (status = 200, description = "Proposer updated", body = ProposerResponse),
        (status = 201, description = "Proposer created", body = ProposerResponse),
        (status = 400, description = "Invalid field value", body = crate::errors::ErrorResponse)
    ),
    tag = "Vouch - Proposers",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn create_or_update_proposer(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(public_key): Path<String>,
    Json(req): Json<CreateOrUpdateProposerRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating/updating proposer: {}", public_key);

    req.validate()?;

    let mut tx = state.pool.begin().await?;

    // Check if proposer exists
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vouch_proposers WHERE public_key = $1")
            .bind(&public_key)
            .fetch_one(&mut *tx)
            .await?;

    let is_new = existing == 0;

    if is_new {
        sqlx::query(
            "INSERT INTO vouch_proposers (public_key, fee_recipient, gas_limit, min_value, reset_relays)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&public_key)
        .bind(&req.fee_recipient)
        .bind(&req.gas_limit)
        .bind(&req.min_value)
        .bind(req.reset_relays)
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "UPDATE vouch_proposers
             SET fee_recipient = $2, gas_limit = $3, min_value = $4, reset_relays = $5
             WHERE public_key = $1",
        )
        .bind(&public_key)
        .bind(&req.fee_recipient)
        .bind(&req.gas_limit)
        .bind(&req.min_value)
        .bind(req.reset_relays)
        .execute(&mut *tx)
        .await?;
    }

    // Handle relays - delete existing and insert new
    sqlx::query("DELETE FROM vouch_proposer_relays WHERE proposer_public_key = $1")
        .bind(&public_key)
        .execute(&mut *tx)
        .await?;

    if let Some(relays) = &req.relays {
        for (url, relay) in relays {
            sqlx::query(
                "INSERT INTO vouch_proposer_relays
                 (proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&public_key)
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
            fee_recipient: req.fee_recipient.as_ref().map(|a| a.to_string()),
            min_value: req.min_value.clone(),
            gas_limit: req.gas_limit.clone(),
            reset_relays: Some(req.reset_relays),
            relays_count: req.relays.as_ref().map(|r| r.len()),
            ..Default::default()
        };
        let action = if is_new {
            AuditAction::Create
        } else {
            AuditAction::Update
        };
        audit_log!(
            ctx,
            action,
            ResourceType::VouchProposer,
            &public_key,
            changes
        );
    }

    // Fetch the result
    let proposer = sqlx::query_as::<_, crate::models::VouchProposer>(
        "SELECT public_key, fee_recipient, gas_limit, min_value, reset_relays, created_at, updated_at
         FROM vouch_proposers WHERE public_key = $1",
    )
    .bind(&public_key)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchProposerRelay>(
        "SELECT id, proposer_public_key, url, public_key, fee_recipient, gas_limit, min_value, disabled
         FROM vouch_proposer_relays WHERE proposer_public_key = $1",
    )
    .bind(&public_key)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, ProposerRelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    let response = ProposerResponse {
        public_key: proposer.public_key,
        fee_recipient: proposer.fee_recipient,
        gas_limit: proposer.gas_limit,
        min_value: proposer.min_value,
        reset_relays: proposer.reset_relays,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: proposer.created_at,
        updated_at: proposer.updated_at,
    };

    if is_new {
        Ok((StatusCode::CREATED, Json(response)))
    } else {
        Ok((StatusCode::OK, Json(response)))
    }
}

#[utoipa::path(
    delete,
    path = "/api/admin/vouch/proposers/{public_key}",
    params(
        ("public_key" = String, Path, description = "Proposer public key")
    ),
    responses(
        (status = 204, description = "Proposer deleted"),
        (status = 404, description = "Proposer not found")
    ),
    tag = "Vouch - Proposers",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn delete_proposer(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(public_key): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Deleting proposer: {}", public_key);

    let result = sqlx::query("DELETE FROM vouch_proposers WHERE public_key = $1")
        .bind(&public_key)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Proposer '{}' not found",
            public_key
        )));
    }

    // Audit log
    if state.config.audit_enabled {
        audit_log!(
            ctx,
            AuditAction::Delete,
            ResourceType::VouchProposer,
            &public_key
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
