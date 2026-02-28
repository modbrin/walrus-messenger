use chrono::{DateTime, Utc};

use crate::auth::token::SessionToken;
use crate::models::user::UserId;

pub type SessionId = uuid::Uuid;

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ResolveSessionResponse {
    pub user_id: UserId,
    pub access_token: SessionToken,
    pub access_token_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct RefreshTokenResponse {
    pub refresh_token: SessionToken,
    pub refresh_token_expires_at: DateTime<Utc>,
    pub refresh_counter: i32,
}
