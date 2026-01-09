// errors.rs
use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
    Json,
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

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response<Body> {
        let (status, error_response) = match &self {
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "NOT_FOUND".to_string(),
                        message: msg.to_string(),
                    },
                },
            ),
            ApiError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "INTERNAL_ERROR".to_string(),
                        message: msg.to_string(),
                    },
                },
            ),
            ApiError::InvalidData(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse {
                    error: ErrorDetail {
                        code: "INVALID_DATA".to_string(),
                        message: msg.to_string(),
                    },
                },
            ),
            ApiError::DatabaseError(e) => match e {
                sqlx::Error::RowNotFound => (
                    StatusCode::NOT_FOUND,
                    ErrorResponse {
                        error: ErrorDetail {
                            code: "NOT_FOUND".to_string(),
                            message: "Resource not found".to_string(),
                        },
                    },
                ),
                _ => {
                    error!("Database error: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        ErrorResponse {
                            error: ErrorDetail {
                                code: "DATABASE_ERROR".to_string(),
                                message: "Internal server error".to_string(),
                            },
                        },
                    )
                }
            },
            ApiError::JsonError(e) => {
                error!("JSON error: {:?}", e);
                (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse {
                        error: ErrorDetail {
                            code: "INVALID_JSON".to_string(),
                            message: "Invalid JSON format".to_string(),
                        },
                    },
                )
            }
        };

        (status, Json(error_response)).into_response()
    }
}
