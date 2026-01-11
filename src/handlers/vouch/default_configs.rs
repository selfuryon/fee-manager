// handlers/vouch/default_configs.rs - Default Config CRUD handlers
use crate::errors::ApiError;
use crate::schema::{
    CreateDefaultConfigRequest, DefaultConfigListItem, DefaultConfigResponse, PaginatedResponse,
    RelayConfig, UpdateDefaultConfigRequest,
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
pub struct DefaultConfigFilters {
    pub name: Option<String>,
    pub fee_recipient: Option<String>,
    pub gas_limit: Option<String>,
    pub min_value: Option<String>,
    pub active: Option<bool>,
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
    path = "/api/admin/vouch/configs/default",
    params(DefaultConfigFilters),
    responses(
        (status = 200, description = "List of default configs", body = PaginatedResponse<DefaultConfigListItem>)
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_default_configs(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<DefaultConfigFilters>,
) -> Result<Json<PaginatedResponse<DefaultConfigListItem>>, ApiError> {
    info!("Listing default configs with filters: {:?}", filters);

    let mut conditions = Vec::new();

    if let Some(ref name) = filters.name {
        conditions.push(format!("name LIKE '{}%'", name.replace('\'', "''")));
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
    if let Some(active) = filters.active {
        conditions.push(format!("active = {}", if active { "true" } else { "false" }));
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) as count FROM vouch_default_configs {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&state.pool)
        .await?;

    let data_sql = format!(
        "SELECT name, fee_recipient, gas_limit, min_value, active, created_at, updated_at
         FROM vouch_default_configs {}
         ORDER BY name ASC
         LIMIT {} OFFSET {}",
        where_clause, filters.limit, filters.offset
    );

    let configs = sqlx::query_as::<_, crate::models::VouchDefaultConfig>(&data_sql)
        .fetch_all(&state.pool)
        .await?;

    // Fetch relays for all configs in the result
    let config_names: Vec<&str> = configs.iter().map(|c| c.name.as_str()).collect();
    let relays_map = if !config_names.is_empty() {
        let placeholders: Vec<String> = config_names.iter().enumerate()
            .map(|(i, _)| format!("${}", i + 1))
            .collect();
        let relays_sql = format!(
            "SELECT id, config_name, url, public_key, fee_recipient, gas_limit, min_value
             FROM vouch_default_relays WHERE config_name IN ({})",
            placeholders.join(", ")
        );
        let mut query = sqlx::query_as::<_, crate::models::VouchDefaultRelay>(&relays_sql);
        for name in &config_names {
            query = query.bind(*name);
        }
        let all_relays = query.fetch_all(&state.pool).await?;

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

    Ok(Json(PaginatedResponse {
        data,
        total,
        limit: filters.limit,
        offset: filters.offset,
    }))
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
    Path(name): Path<String>,
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
        (status = 409, description = "Config already exists")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn create_default_config(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateDefaultConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating default config: {}", req.name);

    let mut tx = state.pool.begin().await?;

    // Check if config already exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM vouch_default_configs WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Err(ApiError::InvalidData(format!(
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
        (status = 404, description = "Config not found")
    ),
    tag = "Vouch - Default Configs",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn update_default_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdateDefaultConfigRequest>,
) -> Result<Json<DefaultConfigResponse>, ApiError> {
    info!("Updating default config: {}", name);

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM vouch_default_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Default config '{}' not found",
            name
        )));
    }

    // Build update query dynamically
    let mut updates = Vec::new();
    if req.fee_recipient.is_some() {
        updates.push("fee_recipient = $2");
    }
    if req.gas_limit.is_some() {
        updates.push("gas_limit = $3");
    }
    if req.min_value.is_some() {
        updates.push("min_value = $4");
    }
    if req.active.is_some() {
        updates.push("active = $5");
    }

    if !updates.is_empty() {
        sqlx::query(&format!(
            "UPDATE vouch_default_configs SET {} WHERE name = $1",
            updates.join(", ")
        ))
        .bind(&name)
        .bind(&req.fee_recipient)
        .bind(&req.gas_limit)
        .bind(&req.min_value)
        .bind(&req.active)
        .execute(&mut *tx)
        .await?;
    }

    // Handle relays if provided
    if let Some(relays) = &req.relays {
        sqlx::query("DELETE FROM vouch_default_relays WHERE config_name = $1")
            .bind(&name)
            .execute(&mut *tx)
            .await?;

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
#[instrument(skip(state))]
pub async fn delete_default_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
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

    Ok(StatusCode::NO_CONTENT)
}
