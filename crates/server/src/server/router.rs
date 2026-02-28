use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use serde::Deserialize;
use tracing::info;

use crate::auth::token::{AuthPayload, Claims, RefreshPayload, TokenExchangePayload};
use crate::auth::utils::unpack_session_id_and_token;
use crate::error::{RequestError, ValidationError};
use crate::models::chat::ListChatsResponse;
use crate::models::user::WhoAmIResponse;
use crate::server::state::AppState;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server.address.clone();
    let app = Router::new()
        .route("/auth/whoami", get(whoami))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
        .route("/chats", get(list_chats))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("starting server on: {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<TokenExchangePayload>, RequestError> {
    let payload = state
        .db_connection
        .login(&payload.alias, &payload.password)
        .await?;
    Ok(Json(payload))
}

pub async fn refresh(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RefreshPayload>,
) -> Result<Json<TokenExchangePayload>, RequestError> {
    let packed_bytes = BASE64
        .decode(&payload.refresh_token)
        .map_err(|_| RequestError::BadCredentials)?;
    let (session_id, refresh_token) =
        unpack_session_id_and_token(&packed_bytes).ok_or(RequestError::BadCredentials)?;
    let payload = state
        .db_connection
        .refresh_session(&session_id, refresh_token)
        .await?;
    Ok(Json(payload))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> Result<StatusCode, RequestError> {
    state.db_connection.logout(&claims.session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn whoami(claims: Claims) -> Json<WhoAmIResponse> {
    Json(WhoAmIResponse {
        user_id: claims.user_id,
    })
}

const DEFAULT_PAGE_SIZE: i32 = 100;
const DEFAULT_PAGE_NUM: i32 = 1;

#[derive(Debug, Deserialize)]
pub struct ListChatsRequest {
    pub page_size: Option<i32>,
    pub page_num: Option<i32>,
}

fn resolve_page_params(params: &ListChatsRequest) -> Result<(i32, i32), RequestError> {
    let page_size = params.page_size.unwrap_or(DEFAULT_PAGE_SIZE);
    if page_size < 1 {
        return Err(ValidationError::InvalidInput {
            value: page_size.to_string(),
            reason: "page_size should be >= 1".to_string(),
        }
        .into());
    }
    let page_num = params.page_num.unwrap_or(DEFAULT_PAGE_NUM);
    if page_num < 1 {
        return Err(ValidationError::InvalidInput {
            value: page_num.to_string(),
            reason: "page_num should be >= 1".to_string(),
        }
        .into());
    }
    Ok((page_size, page_num))
}

pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(params): Query<ListChatsRequest>,
) -> Result<Json<ListChatsResponse>, RequestError> {
    let (page_size, page_num) = resolve_page_params(&params)?;
    let response = state
        .db_connection
        .list_chats(claims.user_id, page_size, page_num)
        .await?;
    Ok(Json(response))
}
