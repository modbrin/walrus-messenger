use thiserror::Error;

use crate::models::user::UserRole;

#[derive(Clone, Debug, Error)]
pub enum ValidationError {
    #[error("name is invalid: `{name}`, reason: {reason}")]
    InvalidName { name: String, reason: String },
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
}
