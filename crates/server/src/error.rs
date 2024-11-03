use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::error;

use crate::models::chat::ChatKind::Private;
use crate::models::user::UserRole;

#[derive(Debug, Error)]
pub enum RequestError {
    #[error("bad auth or refresh credentials")]
    BadCredentials,
    #[error("interrupted operation")]
    Interrupted,
    #[error("operation is not valid anymore, likely requires session refresh or re-login")]
    Expired,
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("sqlx error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, Error)]
pub enum ValidationError {
    #[error("input value is invalid: `{value}`, reason: {reason}")]
    InvalidInput { value: String, reason: String },
    #[error("limit exceeded for {subject}, allowed {limit} {unit}(s), got {attempted}")]
    LimitExceeded {
        subject: String,
        unit: String,
        attempted: usize,
        limit: usize,
    },
    #[error(
        "insufficient permissions for action, required role: {required}, current role: {current}"
    )]
    InsufficientPermissions {
        required: UserRole,
        current: UserRole,
    },
    #[error("requested object already exists")]
    AlreadyExists,
    #[error("requested object doesn't exist or the caller doesn't have access")]
    NotFound,
}

impl IntoResponse for RequestError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::Sqlx(e) => match e {
                sqlx::Error::RowNotFound => (StatusCode::NOT_FOUND, "not found".into()),
                e => {
                    error!("received internal error for user request: {e}");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Something went wrong".into(),
                    )
                }
            },
            Self::Validation(e) => (StatusCode::BAD_REQUEST, e.to_string()),
            e @ Self::BadCredentials => (StatusCode::UNAUTHORIZED, e.to_string()),
            e @ Self::Interrupted => (StatusCode::CONFLICT, e.to_string()),
            e @ Self::Expired => (StatusCode::UNAUTHORIZED, e.to_string()),
        };
        let error = json!({ "error": error }).to_string();
        (status, error).into_response()
    }
}

#[derive(Clone, Debug)]
pub enum SessionError {
    BadToken,
    TokenNotFound,
    TokenExpired,
    Internal,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ErrorResponse<'a> {
    error: &'a str,
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        let (status, error) = match self {
            Self::BadToken => (StatusCode::BAD_REQUEST, "Missing or bad token in request"),
            Self::TokenNotFound => (StatusCode::UNAUTHORIZED, "Token cannot be found"),
            Self::TokenExpired => (StatusCode::UNAUTHORIZED, "Token has expired"),
            Self::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong"),
        };
        let error = json!({ "error": error }).to_string();
        (status, error).into_response()
    }
}
