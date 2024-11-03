use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;

use crate::auth::token::SessionToken;
use crate::models::user::UserId;

pub type SessionId = sqlx::types::Uuid;

// #[derive(Clone, Debug)]
// pub struct CreateSessionRequest {
//     pub user_id: UserId,
//     pub ip: IpNetwork,
//     pub first_seen_at: DateTime<Utc>,
//     pub last_seen_at: DateTime<Utc>,
//     pub device_name: Option<String>,
//     pub os_version: Option<String>,
//     pub app_version: Option<String>,
//     pub refresh_token: SessionToken,
//     pub refresh_token_expires_at: DateTime<Utc>,
//     pub access_token: SessionToken,
//     pub access_token_expires_at: DateTime<Utc>,
// }

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct CreateSessionResponse {
    pub id: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct SessionEntryResponse {
    pub ip: IpNetwork,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub device_name: Option<String>,
    pub os_version: Option<String>,
    pub app_version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ListSessionsResponse {
    pub entries: Vec<SessionEntryResponse>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ResolveSessionResponse {
    pub user_id: UserId,
    pub access_token: SessionToken,
    pub access_token_expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct UpdateTokensRequest {
    pub session_id: SessionId,
    pub refresh_token: SessionToken,
    pub refresh_token_expires_at: DateTime<Utc>,
    pub access_token: SessionToken,
    pub access_token_expires_at: DateTime<Utc>,
}
