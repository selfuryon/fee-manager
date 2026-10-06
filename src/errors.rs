// errors.rs
use axum::{
    Json,
    body::Body,
    extract::rejection::{JsonRejection, PathRejection, QueryRejection},
    http::{Response, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Resource not found: {0}")]
    NotFound(String),

    #[allow(dead_code)]
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Body is not JSON, or not sent as JSON
    #[error("Invalid JSON: {0}")]
    InvalidJson(String),

    /// Body or path parameter parses but holds an invalid value
    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Payload too large")]
    PayloadTooLarge,

    #[error("Method not allowed")]
    MethodNotAllowed,

    #[error("Service unavailable")]
    ServiceUnavailable,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

/// Postgres SQLSTATE for unique_violation
const UNIQUE_VIOLATION: &str = "23505";

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        match rejection {
            JsonRejection::JsonDataError(e) => ApiError::InvalidData(e.body_text()),
            JsonRejection::BytesRejection(e) if e.status() == StatusCode::PAYLOAD_TOO_LARGE => {
                ApiError::PayloadTooLarge
            }
            other => ApiError::InvalidJson(other.body_text()),
        }
    }
}

impl From<QueryRejection> for ApiError {
    fn from(rejection: QueryRejection) -> Self {
        ApiError::InvalidQuery(rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        ApiError::InvalidData(rejection.body_text())
    }
}

impl ApiError {
    fn status_and_detail(&self) -> (StatusCode, &'static str, String) {
        match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            ApiError::InternalError(msg) => {
                error!("Internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Internal server error".to_string(),
                )
            }
            ApiError::InvalidJson(msg) => (StatusCode::BAD_REQUEST, "INVALID_JSON", msg.clone()),
            ApiError::InvalidData(msg) => (StatusCode::BAD_REQUEST, "INVALID_DATA", msg.clone()),
            ApiError::InvalidQuery(msg) => (StatusCode::BAD_REQUEST, "INVALID_QUERY", msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            ApiError::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "PAYLOAD_TOO_LARGE",
                "Request body is too large".to_string(),
            ),
            ApiError::MethodNotAllowed => (
                StatusCode::METHOD_NOT_ALLOWED,
                "METHOD_NOT_ALLOWED",
                "Method not allowed".to_string(),
            ),
            ApiError::ServiceUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                "Service is not ready".to_string(),
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
            ),
            ApiError::DatabaseError(sqlx::Error::RowNotFound) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Resource not found".to_string(),
            ),
            ApiError::DatabaseError(e)
                if e.as_database_error().and_then(|d| d.code()).as_deref()
                    == Some(UNIQUE_VIOLATION) =>
            {
                // A create that raced past the handler's existence check
                (
                    StatusCode::CONFLICT,
                    "CONFLICT",
                    "Resource already exists".to_string(),
                )
            }
            ApiError::DatabaseError(e) => {
                error!("Database error: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Internal server error".to_string(),
                )
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response<Body> {
        let (status, code, message) = self.status_and_detail();
        let body = ErrorResponse {
            error: ErrorDetail {
                code: code.to_string(),
                message,
            },
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detail(err: ApiError) -> (StatusCode, String, String) {
        let (status, code, message) = err.status_and_detail();
        (status, code.to_string(), message)
    }

    #[test]
    fn input_errors_are_400_with_distinct_codes() {
        for (err, code) in [
            (ApiError::InvalidJson("x".into()), "INVALID_JSON"),
            (ApiError::InvalidData("x".into()), "INVALID_DATA"),
            (ApiError::InvalidQuery("x".into()), "INVALID_QUERY"),
        ] {
            let (status, got, _) = detail(err);
            assert_eq!(status, StatusCode::BAD_REQUEST);
            assert_eq!(got, code);
        }
    }

    #[test]
    fn other_statuses() {
        assert_eq!(
            detail(ApiError::Conflict("x".into())).0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            detail(ApiError::PayloadTooLarge).0,
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            detail(ApiError::MethodNotAllowed).0,
            StatusCode::METHOD_NOT_ALLOWED
        );
        assert_eq!(detail(ApiError::Unauthorized).0, StatusCode::UNAUTHORIZED);
        let (status, code, _) = detail(ApiError::ServiceUnavailable);
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(code, "SERVICE_UNAVAILABLE");
        assert_eq!(
            detail(ApiError::DatabaseError(sqlx::Error::RowNotFound)).0,
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn database_error_text_is_not_exposed() {
        let err = ApiError::DatabaseError(sqlx::Error::Protocol(
            "relation \"secret_table\" does not exist".into(),
        ));
        let (status, code, message) = detail(err);
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(code, "INTERNAL_ERROR");
        assert!(!message.contains("secret_table"));
    }
}
