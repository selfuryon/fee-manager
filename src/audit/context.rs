//! Request context for audit logging

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use uuid::Uuid;

use super::ActorInfo;
use crate::errors::ApiError;

/// Request context containing actor info and request ID
/// Extracted by handlers that need to perform audit logging
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: Uuid,
    pub actor: ActorInfo,
}

impl<S> FromRequestParts<S> for RequestContext
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let request_id = parts
            .extensions
            .get::<Uuid>()
            .copied()
            .unwrap_or_else(Uuid::new_v4);

        let actor = parts
            .extensions
            .get::<ActorInfo>()
            .cloned()
            .unwrap_or_default();

        Ok(RequestContext { request_id, actor })
    }
}
