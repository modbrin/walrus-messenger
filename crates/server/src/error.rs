use thiserror::Error;

use crate::models::user::UserRole;

#[derive(Debug, Error)]
pub enum RequestError {
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
