// Token CRUD handlers

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::{service, TokenInfo};
use crate::{errors::ApiError, AppState};

/// Request body for creating a new token
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTokenRequest {
    /// Short identifier for the token
    pub name: String,
    /// Optional longer description
    pub description: Option<String>,
}

/// Response when a token is created (includes plaintext token)
#[derive(Debug, Serialize, ToSchema)]
pub struct CreateTokenResponse {
    pub id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The plaintext token - shown only once!
    pub token: String,
}

/// Create token routes
pub fn token_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_tokens).post(create_token))
        .route("/{id}", delete(delete_token))
}

/// List all tokens
#[utoipa::path(
    get,
    path = "/api/admin/tokens",
    tag = "Auth",
    responses(
        (status = 200, description = "List of tokens", body = Vec<TokenInfo>),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_tokens(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TokenInfo>>, ApiError> {
    let tokens = service::list_tokens(&state.pool).await?;
    let token_infos: Vec<TokenInfo> = tokens.into_iter().map(TokenInfo::from).collect();
    Ok(Json(token_infos))
}

/// Create a new token
#[utoipa::path(
    post,
    path = "/api/admin/tokens",
    tag = "Auth",
    request_body = CreateTokenRequest,
    responses(
        (status = 201, description = "Token created", body = CreateTokenResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_token(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CreateTokenRequest>,
) -> Result<Json<CreateTokenResponse>, ApiError> {
    let (token, plaintext) =
        service::create_token(&state.pool, &request.name, request.description.as_deref()).await?;

    Ok(Json(CreateTokenResponse {
        id: token.id,
        name: token.name,
        description: token.description,
        token: plaintext,
    }))
}

/// Delete a token by ID
#[utoipa::path(
    delete,
    path = "/api/admin/tokens/{id}",
    tag = "Auth",
    params(
        ("id" = Uuid, Path, description = "Token ID to delete")
    ),
    responses(
        (status = 204, description = "Token deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Token not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_token(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, ApiError> {
    let deleted = service::delete_token(&state.pool, id).await?;

    if deleted {
        Ok(axum::http::StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(format!("Token {} not found", id)))
    }
}
