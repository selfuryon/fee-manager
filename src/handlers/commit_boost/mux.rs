// handlers/commit_boost/mux.rs - Mux config CRUD handlers
use crate::addresses::BlsPubkey;
use crate::audit::{AuditAction, AuditChanges, RequestContext, ResourceType};
use crate::audit_log;
use crate::errors::ApiError;
use crate::schema::{
    CreateMuxConfigRequest, MuxConfigListItem, MuxConfigResponse, MuxKeysRequest, MuxKeysResponse,
    PaginatedResponse, UpdateMuxConfigRequest,
};
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;
use tracing::{info, instrument};
use utoipa::IntoParams;

#[derive(Debug, Deserialize, IntoParams)]
pub struct MuxConfigFilters {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

// ============================================================================
// Public Endpoint
// ============================================================================

#[utoipa::path(
    get,
    path = "/commit-boost/v1/mux/{name}",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    responses(
        (status = 200, description = "List of validator public keys", body = Vec<BlsPubkey>),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Public"
)]
#[instrument(skip(state))]
pub async fn get_mux_keys_public(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<BlsPubkey>>, ApiError> {
    info!("Getting mux keys (public): {}", name);

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Mux config '{}' not found",
            name
        )));
    }

    let keys = sqlx::query_scalar::<_, BlsPubkey>(
        "SELECT public_key FROM commit_boost_mux_keys WHERE mux_name = $1 ORDER BY id",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(keys))
}

// ============================================================================
// Admin Endpoints
// ============================================================================

#[utoipa::path(
    get,
    path = "/api/admin/commit-boost/mux",
    params(MuxConfigFilters),
    responses(
        (status = 200, description = "List of mux configs", body = PaginatedResponse<MuxConfigListItem>)
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn list_mux_configs(
    State(state): State<Arc<AppState>>,
    Query(filters): Query<MuxConfigFilters>,
) -> Result<Json<PaginatedResponse<MuxConfigListItem>>, ApiError> {
    info!("Listing mux configs");

    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM commit_boost_mux_configs")
        .fetch_one(&state.pool)
        .await?;

    let configs = sqlx::query_as::<_, crate::models::CommitBoostMuxConfig>(
        "SELECT name, created_at, updated_at
         FROM commit_boost_mux_configs
         ORDER BY name ASC
         LIMIT $1 OFFSET $2",
    )
    .bind(filters.limit)
    .bind(filters.offset)
    .fetch_all(&state.pool)
    .await?;

    let mut data = Vec::new();
    for config in configs {
        let key_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM commit_boost_mux_keys WHERE mux_name = $1",
        )
        .bind(&config.name)
        .fetch_one(&state.pool)
        .await?;

        data.push(MuxConfigListItem {
            name: config.name,
            key_count,
            created_at: config.created_at,
            updated_at: config.updated_at,
        });
    }

    Ok(Json(PaginatedResponse {
        data,
        total,
        limit: filters.limit,
        offset: filters.offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin/commit-boost/mux/{name}",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    responses(
        (status = 200, description = "Mux config details", body = MuxConfigResponse),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state))]
pub async fn get_mux_config(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<MuxConfigResponse>, ApiError> {
    info!("Getting mux config: {}", name);

    let config = sqlx::query_as::<_, crate::models::CommitBoostMuxConfig>(
        "SELECT name, created_at, updated_at FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("Mux config '{}' not found", name)))?;

    let keys = sqlx::query_scalar::<_, BlsPubkey>(
        "SELECT public_key FROM commit_boost_mux_keys WHERE mux_name = $1 ORDER BY id",
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(MuxConfigResponse {
        name: config.name,
        keys,
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/admin/commit-boost/mux",
    request_body = CreateMuxConfigRequest,
    responses(
        (status = 201, description = "Mux config created", body = MuxConfigListItem),
        (status = 409, description = "Mux config already exists")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn create_mux_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Json(req): Json<CreateMuxConfigRequest>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Creating mux config: {}", req.name);

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&mut *tx)
    .await?;

    if existing > 0 {
        return Err(ApiError::InvalidData(format!(
            "Mux config '{}' already exists",
            req.name
        )));
    }

    sqlx::query("INSERT INTO commit_boost_mux_configs (name) VALUES ($1)")
        .bind(&req.name)
        .execute(&mut *tx)
        .await?;

    for key in &req.keys {
        sqlx::query("INSERT INTO commit_boost_mux_keys (mux_name, public_key) VALUES ($1, $2)")
            .bind(&req.name)
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            key_count: Some(req.keys.len() as i64),
            ..Default::default()
        };
        audit_log!(ctx, AuditAction::Create, ResourceType::CommitBoostMux, &req.name, changes);
    }

    let config = sqlx::query_as::<_, crate::models::CommitBoostMuxConfig>(
        "SELECT name, created_at, updated_at FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&req.name)
    .fetch_one(&state.pool)
    .await?;

    let response = MuxConfigListItem {
        name: config.name,
        key_count: req.keys.len() as i64,
        created_at: config.created_at,
        updated_at: config.updated_at,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

#[utoipa::path(
    put,
    path = "/api/admin/commit-boost/mux/{name}",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    request_body = UpdateMuxConfigRequest,
    responses(
        (status = 200, description = "Mux config updated", body = MuxConfigResponse),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn update_mux_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(name): Path<String>,
    Json(req): Json<UpdateMuxConfigRequest>,
) -> Result<Json<MuxConfigResponse>, ApiError> {
    info!("Updating mux config: {}", name);

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Mux config '{}' not found",
            name
        )));
    }

    // Replace all keys
    sqlx::query("DELETE FROM commit_boost_mux_keys WHERE mux_name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    for key in &req.keys {
        sqlx::query("INSERT INTO commit_boost_mux_keys (mux_name, public_key) VALUES ($1, $2)")
            .bind(&name)
            .bind(key)
            .execute(&mut *tx)
            .await?;
    }

    // Touch updated_at
    sqlx::query("UPDATE commit_boost_mux_configs SET updated_at = NOW() WHERE name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            key_count: Some(req.keys.len() as i64),
            ..Default::default()
        };
        audit_log!(ctx, AuditAction::Update, ResourceType::CommitBoostMux, &name, changes);
    }

    let config = sqlx::query_as::<_, crate::models::CommitBoostMuxConfig>(
        "SELECT name, created_at, updated_at FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(MuxConfigResponse {
        name: config.name,
        keys: req.keys,
        created_at: config.created_at,
        updated_at: config.updated_at,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/commit-boost/mux/{name}",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    responses(
        (status = 204, description = "Mux config deleted"),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn delete_mux_config(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    info!("Deleting mux config: {}", name);

    let result = sqlx::query("DELETE FROM commit_boost_mux_configs WHERE name = $1")
        .bind(&name)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound(format!(
            "Mux config '{}' not found",
            name
        )));
    }

    // Audit log
    if state.config.audit_enabled {
        audit_log!(ctx, AuditAction::Delete, ResourceType::CommitBoostMux, &name);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/admin/commit-boost/mux/{name}/keys",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    request_body = MuxKeysRequest,
    responses(
        (status = 200, description = "Keys added", body = MuxKeysResponse),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn add_mux_keys(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(name): Path<String>,
    Json(req): Json<MuxKeysRequest>,
) -> Result<Json<MuxKeysResponse>, ApiError> {
    info!("Adding keys to mux config: {}", name);

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Mux config '{}' not found",
            name
        )));
    }

    let mut added = 0i64;
    for key in &req.keys {
        // Use ON CONFLICT to avoid duplicates
        let result = sqlx::query(
            "INSERT INTO commit_boost_mux_keys (mux_name, public_key) VALUES ($1, $2)
             ON CONFLICT (mux_name, public_key) DO NOTHING",
        )
        .bind(&name)
        .bind(key)
        .execute(&mut *tx)
        .await?;
        added += result.rows_affected() as i64;
    }

    // Touch updated_at
    sqlx::query("UPDATE commit_boost_mux_configs SET updated_at = NOW() WHERE name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let total_keys: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM commit_boost_mux_keys WHERE mux_name = $1")
            .bind(&name)
            .fetch_one(&state.pool)
            .await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            key_count: Some(added),
            ..Default::default()
        };
        audit_log!(ctx, AuditAction::AddKeys, ResourceType::CommitBoostMux, &name, changes);
    }

    Ok(Json(MuxKeysResponse {
        added: Some(added),
        removed: None,
        total_keys,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/admin/commit-boost/mux/{name}/keys",
    params(
        ("name" = String, Path, description = "Mux config name")
    ),
    request_body = MuxKeysRequest,
    responses(
        (status = 200, description = "Keys removed", body = MuxKeysResponse),
        (status = 404, description = "Mux config not found")
    ),
    tag = "Commit-Boost - Mux",
    security(("bearer_auth" = []))
)]
#[instrument(skip(state, ctx))]
pub async fn remove_mux_keys(
    State(state): State<Arc<AppState>>,
    ctx: RequestContext,
    Path(name): Path<String>,
    Json(req): Json<MuxKeysRequest>,
) -> Result<Json<MuxKeysResponse>, ApiError> {
    info!("Removing keys from mux config: {}", name);

    let mut tx = state.pool.begin().await?;

    // Check if config exists
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM commit_boost_mux_configs WHERE name = $1",
    )
    .bind(&name)
    .fetch_one(&mut *tx)
    .await?;

    if existing == 0 {
        return Err(ApiError::NotFound(format!(
            "Mux config '{}' not found",
            name
        )));
    }

    let result = sqlx::query(
        "DELETE FROM commit_boost_mux_keys WHERE mux_name = $1 AND public_key = ANY($2)",
    )
    .bind(&name)
    .bind(&req.keys)
    .execute(&mut *tx)
    .await?;

    let removed = result.rows_affected() as i64;

    // Touch updated_at
    sqlx::query("UPDATE commit_boost_mux_configs SET updated_at = NOW() WHERE name = $1")
        .bind(&name)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    let total_keys: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM commit_boost_mux_keys WHERE mux_name = $1")
            .bind(&name)
            .fetch_one(&state.pool)
            .await?;

    // Audit log
    if state.config.audit_enabled {
        let changes = AuditChanges {
            key_count: Some(removed),
            ..Default::default()
        };
        audit_log!(ctx, AuditAction::RemoveKeys, ResourceType::CommitBoostMux, &name, changes);
    }

    Ok(Json(MuxKeysResponse {
        added: None,
        removed: Some(removed),
        total_keys,
    }))
}
