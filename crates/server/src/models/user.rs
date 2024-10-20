use std::time::SystemTime;

use chrono::{DateTime, Utc};
use strum_macros::Display;

use crate::error::ValidationError;

pub type UserId = i32;
const USER_DISPLAY_NAME_LENGTH_LIMIT: usize = 30;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Display, sqlx::Type)]
#[sqlx(type_name = "user_role")]
#[sqlx(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Regular,
}

#[derive(Clone, Debug)]
pub struct CreateUser {
    pub alias: String,
    pub display_name: String,
    pub role: UserRole,
    pub password_salt: [u8; 16],
    pub password_hash: [u8; 32],
    pub invited_by: Option<UserId>,
}

pub struct UpdateUserAlias {
    new_alias: String,
}

pub struct UpdateUserDisplayName {
    new_display_name: String,
}

pub struct UpdateUserPassword {
    password_salt: String,
    password_hash: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct User {
    pub user_id: UserId,
    pub display_name: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub invited_by: UserId,
}

impl User {
    pub fn check_role(&self, required: UserRole) -> Result<(), ValidationError> {
        if self.role != required {
            return Err(ValidationError::InsufficientPermissions {
                current: self.role,
                required: UserRole::Admin,
            });
        }
        Ok(())
    }

    pub fn validate_name(name: &str) -> Result<(), ValidationError> {
        if name.trim().len() != name.len() {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
                reason: "user display name cannot be surrounded with whitespace characters"
                    .to_string(),
            });
        }
        if name.is_empty() {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
                reason: "user display name cannot be empty".to_string(),
            });
        }
        if name.len() > USER_DISPLAY_NAME_LENGTH_LIMIT {
            return Err(ValidationError::InvalidName {
                name: name.to_string(),
                reason: format!(
                    "user display name cannot be longer than {} chars",
                    USER_DISPLAY_NAME_LENGTH_LIMIT
                ),
            });
        }
        Ok(())
    }
}
