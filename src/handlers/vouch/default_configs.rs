// handlers/vouch/default_configs.rs - Default Config CRUD handlers
use crate::AppState;
use crate::audit::{AuditAction, AuditChanges, RequestContext, ResourceType};
use crate::audit_log;
use crate::errors::ApiError;
use crate::extract::{AppJson, AppPath, AppQuery};
use crate::pagination::{ListResponse, check_limit};
use crate::schema::{
    CreateDefaultConfigRequest, DefaultConfigListItem, DefaultConfigResponse, RelayConfig,
    UpdateDefaultConfigRequest,
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
pub struct DefaultConfigFilters {
    pub name: Option<String>,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub active: Option<bool>,
    /// Filter by relay URL (prefix match)
    pub relay_url: Option<String>,
    /// Filter by relay min_value (exact match)
    pub relay_min_value: Option<String>,
    /// Maximum number of items to return (1-1000, default 100)
    #[serde(default = "crate::pagination::default_limit")]
    pub limit: i64,
    /// Opaque cursor: the `next_cursor` of the previous page
    pub after: Option<String>,
}

/// Appends the WHERE clause for `filters`; every value is a bind parameter.
fn push_filters(qb: &mut QueryBuilder<Postgres>, filters: &DefaultConfigFilters) {
    qb.push(" WHERE TRUE");
    if let Some(ref name) = filters.name {
        qb.push(" AND c.name LIKE ").push_bind(format!("{name}%"));
    }
    if let Some(ref fr) = filters.fee_recipient {
        qb.push(" AND c.fee_recipient = ").push_bind(fr.clone());
    }
    if let Some(ref gl) = filters.gas_limit {
        qb.push(" AND c.gas_limit = ").push_bind(gl.clone());
    }
    if let Some(ref mv) = filters.min_value {
        qb.push(" AND c.min_value = ").push_bind(mv.clone());
    }
    if let Some(active) = filters.active {
        qb.push(" AND c.active = ").push_bind(active);
    }
    // Relay filters using EXISTS subquery
    if let Some(ref relay_url) = filters.relay_url {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_default_relays r WHERE r.config_name = c.name AND r.url LIKE ")
            .push_bind(format!("{relay_url}%"))
            .push(")");
    }
    if let Some(ref relay_min_value) = filters.relay_min_value {
        qb.push(" AND EXISTS (SELECT 1 FROM vouch_default_relays r WHERE r.config_name = c.name AND r.min_value = ")
            .push_bind(relay_min_value.clone())
            .push(")");
    }
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/configs/default",
    params(DefaultConfigFilters),
    responses(
        (status = 200, description = "List of default configs", body = ListResponse<DefaultConfigListItem>)
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_default_configs(
    State(state): State<Arc<AppState>>,
    AppQuery(filters): AppQuery<DefaultConfigFilters>,
) -> Result<Json<ListResponse<DefaultConfigListItem>>, ApiError> {
    info!("Listing default configs with filters: {:?}", filters);

    check_limit(filters.limit)?;

    let mut data_query = QueryBuilder::new(
        "SELECT c.name, c.fee_recipient, c.gas_limit, c.min_value, c.active, c.created_at, c.updated_at
         FROM vouch_default_configs c",
    );
    push_filters(&mut data_query, &filters);
    if let Some(ref after) = filters.after {
        data_query.push(" AND c.name > ").push_bind(after.clone());
    }
    data_query
        .push(" ORDER BY c.name LIMIT ")
        .push_bind(filters.limit);

    let configs = data_query
        .build_query_as::<crate::models::VouchDefaultConfig>()
        .fetch_all(&state.pool)
        .await?;

    // Fetch relays for all configs in the result
    let config_names: Vec<&str> = configs.iter().map(|c| c.name.as_str()).collect();
    let relays_map = if !config_names.is_empty() {
        let all_relays = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(
            "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
             FROM vouch_default_relays WHERE config_name = ANY($1)",
        )
        .bind(&config_names)
        .fetch_all(&state.pool)
        .await?;

        // Group relays by config_name
        let mut map: HashMap<String, HashMap<String, RelayConfig>> = HashMap::new();
        for relay in all_relays {
            map.entry(relay.config_name.clone())
                .or_default()
                .insert(relay.url.clone(), relay.into());
        }
        map
    } else {
        HashMap::new()
    };

    let data: Vec<DefaultConfigListItem> = configs
        .into_iter()
        .map(|c| {
            let relays = relays_map.get(&c.name).cloned();
            let mut item: DefaultConfigListItem = c.into();
            item.relays = relays;
            item
        })
        .collect();

    Ok(Json(ListResponse::new(data, filters.limit, |item| {
        item.name.to_string()
    })))
}

#[utoipa::path(
    get,
    path = "/api/admin/vouch/configs/default/{name}",
    params(
        ("name" = String, Path, description = "Config name")
    ),
    responses(
        (status = 200, description = "Default config details", body = DefaultConfigResponse),
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_default_config(
    State(state): State<Arc<AppState>>,
    AppPath(name): AppPath<String>,
) -> Result<Json<DefaultConfigResponse>, ApiError> {
    info!("Getting default config: {}", name);

    let config = sqlx::query_as::<_, crate::models::VouchDefaultConfig>(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Default config '{}' not found", name)))?;

    let relays = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(
        "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_default_relays WHERE config_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    Ok(Json(DefaultConfigResponse {
        name: config.name,
        fee_recipient: config.fee_recipient,
        gas_limit: config.gas_limit,
        min_value: config.min_value,
        active: config.active,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/vouch/configs/default",
    request_body = CreateDefaultConfigRequest,
    responses(
        (status = 201, description = "Config created", body = DefaultConfigResponse),
        (status = 400, description = "Invalid field value", body = crate::errors::ErrorResponse),
        (status = 409, description = "Config already exists")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn create_default_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    AppJson(req): AppJson<CreateDefaultConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating default config: {}", req.name);

    req.validate()?;

    let mut tx = state.pool.begin().await?;

    // Check if config already exists
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vouch_default_configs WHERE name = $1")
            .bind(&req.name)
            .fetch_one(&mut *tx)
            .await?;

    if existing > 0 {
        return Err(ApiError::Conflict(format!(
            "Config '{}' already exists",
            req.name
        )));
    }

    sqlx::query(
        "INSERT INTO vouch_default_configs (name, fee_recipient, gas_limit, min_value, active)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(&req.name)
    .bind(&req.fee_recipient)
    .bind(&req.gas_limit)
    .bind(&req.min_value)
    .bind(req.active)
    .execute(&mut *tx)
    .await?;

    if let Some(relays) = &req.relays {
        for (url, relay) in relays {
            sqlx::query(
                "INSERT INTO vouch_default_relays
                 (config_name, url, public_key, fee_recipient, gas_limit, min_value)
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

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            fee_recipient: req.fee_recipient.as_ref().map(|a| a.to_string()),
            min_value: req.min_value.clone(),
            gas_limit: req.gas_limit.clone(),
            active: Some(req.active),
            relays_count: req.relays.as_ref().map(|r| r.len()),
            ..Default::default()
        };
        audit_log!(
            ctx,
            AuditAction::Create,
            ResourceType::VouchDefaultConfig,
            &req.name,
            changes
        );
    }

    // Fetch the created config
    let config = sqlx::query_as::<_, crate::models::VouchDefaultConfig>(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(
        "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_default_relays WHERE config_name = $1",
    )
    .bind(&req.name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    let response = DefaultConfigResponse {
        name: config.name,
        fee_recipient: config.fee_recipient,
        gas_limit: config.gas_limit,
        min_value: config.min_value,
        active: config.active,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: config.created_at,
        updated_at: config.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/admin/vouch/configs/default/{name}",
    params(
        ("name" = String, Path, description = "Config name")
    ),
    request_body = UpdateDefaultConfigRequest,
    responses(
        (status = 200, description = "Config updated", body = DefaultConfigResponse),
        (status = 400, description = "Invalid field value", body = crate::errors::ErrorResponse),
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn update_default_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    AppPath(name): AppPath<String>,
    AppJson(req): AppJson<UpdateDefaultConfigRequest>,
) -> Result<Json<DefaultConfigResponse>, ApiError> {
    info!("Updating default config: {}", name);

    req.validate()?;

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM vouch_default_configs WHERE name = $1")
            .bind(&name)
            .fetch_one(&mut *tx)
            .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Default config '{}' not found",
            name
        )));
    }

    // Full replacement: every column comes from the request, and the relay
    // set is replaced by the request's (none when omitted)
    sqlx::query(
        "UPDATE vouch_default_configs
         SET fee_recipient = $2, gas_limit = $3, min_value = $4, active = $5
         WHERE name = $1",
    )
    .bind(&name)
    .bind(&req.fee_recipient)
    .bind(&req.gas_limit)
    .bind(&req.min_value)
    .bind(req.active)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM vouch_default_relays WHERE config_name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    if let Some(relays) = &req.relays {
        for (url, relay) in relays {
            sqlx::query(
                "INSERT INTO vouch_default_relays
                 (config_name, url, public_key, fee_recipient, gas_limit, min_value)
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

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            fee_recipient: req.fee_recipient.as_ref().map(|a| a.to_string()),
            min_value: req.min_value.clone(),
            gas_limit: req.gas_limit.clone(),
            active: Some(req.active),
            relays_count: req.relays.as_ref().map(|r| r.len()),
            ..Default::default()
        };
        audit_log!(
            ctx,
            AuditAction::Update,
            ResourceType::VouchDefaultConfig,
            &name,
            changes
        );
    }

    // Fetch updated config
    let config = sqlx::query_as::<_, crate::models::VouchDefaultConfig>(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;

    let relays = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(
        "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
         FROM vouch_default_relays WHERE config_name = $1",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    let relays_map: HashMap<String, RelayConfig> = relays
        .into_iter()
        .map(|r| (r.url.clone(), r.into()))
        .collect();

    Ok(Json(DefaultConfigResponse {
        name: config.name,
        fee_recipient: config.fee_recipient,
        gas_limit: config.gas_limit,
        min_value: config.min_value,
        active: config.active,
        relays: if relays_map.is_empty() {
            None
        } else {
            Some(relays_map)
        },
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/vouch/configs/default/{name}",
    params(
        ("name" = String, Path, description = "Config name")
    ),
    responses(
        (status = 204, description = "Config deleted"),
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn delete_default_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    AppPath(name): AppPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Deleting default config: {}", name);

    let result = sqlx::query("DELETE FROM vouch_default_configs WHERE name = $1")
        .bind(&name)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Default config '{}' not found",
            name
        )));
    }

    // Audit log
    if state.config.audit_enabled {
        audit_log!(
            ctx,
            AuditAction::Delete,
            ResourceType::VouchDefaultConfig,
            &name
        );
    }

    Ok(StatusCode::NO_CONTENT)
}
