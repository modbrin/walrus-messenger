use serde::Serialize;
use strum_macros::Display;

use crate::error::ValidationError;

pub type UserId = i32;
const USER_DISPLAY_NAME_LENGTH_LIMIT: usize = 30;
const USER_ALIAS_LENGTH_LIMIT: usize = 30;
const USER_PASSWORD_MIN_LENGTH: usize = 8;
const USER_PASSWORD_MAX_LENGTH: usize = 80;

#[derive(Clone, Debug, Serialize)]
pub struct WhoAmIResponse {
    pub user_id: UserId,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Display, sqlx::Type)]
#[sqlx(type_name = "user_role")]
#[sqlx(rename_all = "snake_case")]
pub enum UserRole {
    Admin,
    Regular,
}

#[derive(Clone, Debug)]
pub struct CreateUserRequest {
    pub alias: String,
    pub display_name: String,
    pub role: UserRole,
    pub password_salt: [u8; 16],
    pub password_hash: [u8; 32],
    pub invited_by: Option<UserId>,
}

#[derive(Clone, Debug)]
pub struct InviteUserRequest {
    pub alias: String,
    pub display_name: String,
    pub role: UserRole,
    pub initial_password: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct GetUserRoleResponse {
    pub role: UserRole,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct GetUserIdByAliasResponse {
    pub user_id: UserId,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct GetUserCredentialsByAliasResponse {
    pub user_id: UserId,
    pub password_salt: [u8; 16],
    pub password_hash: [u8; 32],
}

// TODO: remove
// #[derive(Clone, Debug, sqlx::FromRow)]
// pub struct User {
//     pub user_id: UserId,
//     pub display_name: String,
//     pub role: UserRole,
//     pub created_at: DateTime<Utc>,
//     pub invited_by: UserId,
// }

// TODO: add regexes
pub fn validate_user_alias(alias: &str) -> Result<(), ValidationError> {
    for ch in alias.chars() {
        if !(ch.is_alphanumeric() || ch == '_') {
            return Err(ValidationError::InvalidInput {
                value: alias.to_string(),
                reason: "alias can only contain letters, numbers and underscores".to_string(),
            });
        }
    }
    if alias.is_empty() {
        return Err(ValidationError::InvalidInput {
            value: alias.to_string(),
            reason: "user alias cannot be empty".to_string(),
        });
    }
    if alias.len() > USER_ALIAS_LENGTH_LIMIT {
        return Err(ValidationError::InvalidInput {
            value: alias.to_string(),
            reason: format!(
                "user alias cannot be longer than {} chars",
                USER_DISPLAY_NAME_LENGTH_LIMIT
            ),
        });
    }
    Ok(())
}

pub fn validate_user_display_name(display_name: &str) -> Result<(), ValidationError> {
    if display_name.trim().len() != display_name.len() {
        return Err(ValidationError::InvalidInput {
            value: display_name.to_string(),
            reason: "user display name cannot be surrounded with whitespace characters".to_string(),
        });
    }
    if display_name.is_empty() {
        return Err(ValidationError::InvalidInput {
            value: display_name.to_string(),
            reason: "user display name cannot be empty".to_string(),
        });
    }
    if display_name.len() > USER_DISPLAY_NAME_LENGTH_LIMIT {
        return Err(ValidationError::InvalidInput {
            value: display_name.to_string(),
            reason: format!(
                "user display name cannot be longer than {} chars",
                USER_DISPLAY_NAME_LENGTH_LIMIT
            ),
        });
    }
    Ok(())
}

pub fn validate_user_password(password: &str) -> Result<(), ValidationError> {
    if password.len() < USER_PASSWORD_MIN_LENGTH || password.len() > USER_PASSWORD_MAX_LENGTH {
        return Err(ValidationError::InvalidInput {
            value: "<password>".to_string(),
            reason: format!(
                "password should be at least {} and at most {} characters long",
                USER_PASSWORD_MIN_LENGTH, USER_PASSWORD_MAX_LENGTH
            ),
        });
    }
    Ok(())
}
