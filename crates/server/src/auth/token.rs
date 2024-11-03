use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::auth::utils::{pack_session_id_and_token, unpack_session_id_and_token};
use crate::error::SessionError;
use crate::models::session::SessionId;
use crate::models::user::UserId;
use crate::server::state::AppState;

pub type SessionToken = Vec<u8>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub user_id: UserId,
}

#[async_trait]
impl FromRequestParts<Arc<AppState>> for Claims {
    type Rejection = SessionError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| {
                debug!("malformed auth header token: {e}");
                SessionError::BadToken
            })?;
        let access_token = BASE64.decode(bearer.token()).map_err(|_| {
            debug!("malformed auth header token: bearer is not base64");
            SessionError::BadToken
        })?;
        let (sid, access_token) = unpack_session_id_and_token(&access_token).ok_or_else(|| {
            debug!("malformed auth header token: unable to unpack");
            SessionError::BadToken
        })?;
        let user_id = state
            .db_connection
            .resolve_session(&sid, access_token)
            .await?;
        Ok(Claims { user_id })
    }
}

#[derive(Debug, Serialize)]
pub struct TokenExchangePayload {
    pub refresh_token: String,
    pub refresh_token_expires_at: String,
    pub access_token: String,
    pub access_token_expires_at: String,
}

impl TokenExchangePayload {
    pub fn new<B1: AsRef<[u8]>, B2: AsRef<[u8]>>(
        session_id: &SessionId,
        refresh_token: B1,
        refresh_token_expires_at: DateTime<Utc>,
        access_token: B2,
        access_token_expires_at: DateTime<Utc>,
    ) -> Self {
        let refresh_token = pack_session_id_and_token(session_id, refresh_token.as_ref());
        let access_token = pack_session_id_and_token(session_id, access_token.as_ref());
        Self {
            refresh_token: BASE64.encode(refresh_token),
            refresh_token_expires_at: refresh_token_expires_at.to_rfc3339(),
            access_token: BASE64.encode(access_token),
            access_token_expires_at: access_token_expires_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct AuthPayload {
    pub alias: String,
    pub password: String,
    pub session_id: Option<String>, // TODO: use
}
