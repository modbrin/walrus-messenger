use std::sync::Arc;

use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use tracing::info;

use crate::auth::token::{AuthPayload, Claims, RefreshPayload, TokenExchangePayload};
use crate::auth::utils::unpack_session_id_and_token;
use crate::error::{RequestError, ValidationError};
use crate::models::chat::{ChatId, ListChatsResponse};
use crate::models::listing::{ListingMode, ListingQuery};
use crate::models::message::{
    validate_message_text, ListMessagesResponse, SendMessageRequest, SendMessageResponse,
};
use crate::models::user::{
    ChangeAliasRequest, ChangeDisplayNameRequest, ChangePasswordRequest, InviteUserRequest,
    InviteUserResponse, WhoAmIResponse,
};
use crate::server::constants::MAX_REQUEST_BODY_BYTES;
use crate::server::state::AppState;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server.address.clone();
    let app = Router::new()
        .route("/health", get(health))
        .route("/auth/whoami", get(whoami))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/change-password", post(change_password))
        .route("/auth/change-alias", post(change_alias))
        .route("/auth/change-display-name", post(change_display_name))
        .route("/auth/logout", post(logout))
        .route("/users/invite", post(invite_user))
        .route("/chats", get(list_chats))
        .route(
            "/chats/:chat_id/messages",
            get(list_messages).post(send_message),
        )
        .layer(DefaultBodyLimit::max(MAX_REQUEST_BODY_BYTES))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("starting server on: {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}

pub async fn health() -> StatusCode {
    StatusCode::OK
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AuthPayload>,
) -> Result<Json<TokenExchangePayload>, RequestError> {
    state.rate_limiter.check_login_alias(&payload.alias)?;
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
    state.rate_limiter.check_refresh_session(session_id)?;
    let payload = state
        .db_connection
        .refresh_session(session_id, refresh_token)
        .await?;
    Ok(Json(payload))
}

pub async fn logout(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> Result<StatusCode, RequestError> {
    state.db_connection.logout(claims.session_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(payload): Json<ChangePasswordRequest>,
) -> Result<StatusCode, RequestError> {
    state
        .rate_limiter
        .check_change_password_user(claims.user_id)?;
    state
        .db_connection
        .change_password(
            claims.user_id,
            claims.session_id,
            &payload.current_password,
            &payload.new_password,
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_alias(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(payload): Json<ChangeAliasRequest>,
) -> Result<StatusCode, RequestError> {
    state
        .db_connection
        .change_alias(claims.user_id, &payload.new_alias)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn change_display_name(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(payload): Json<ChangeDisplayNameRequest>,
) -> Result<StatusCode, RequestError> {
    state
        .db_connection
        .change_display_name(claims.user_id, &payload.new_display_name)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn whoami(claims: Claims) -> Json<WhoAmIResponse> {
    Json(WhoAmIResponse {
        user_id: claims.user_id,
    })
}

pub async fn invite_user(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(payload): Json<InviteUserRequest>,
) -> Result<(StatusCode, Json<InviteUserResponse>), RequestError> {
    let user_id = state
        .db_connection
        .invite_user(claims.user_id, &payload.alias, &payload.password)
        .await?;
    Ok((StatusCode::CREATED, Json(InviteUserResponse { user_id })))
}

pub async fn list_chats(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Query(params): Query<ListingQuery>,
) -> Result<Json<ListChatsResponse>, RequestError> {
    let (page_size, page_num) = match ListingMode::from_query(params)? {
        ListingMode::Page { limit, page } => (limit, page),
        ListingMode::Offset { .. } => {
            return Err(ValidationError::InvalidInput {
                value: "offset".to_string(),
                reason: "offset mode is not supported for chats listing".to_string(),
            }
            .into())
        }
    };
    let response = state
        .db_connection
        .list_chats(claims.user_id, page_size, page_num)
        .await?;
    Ok(Json(response))
}

pub async fn list_messages(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(chat_id): Path<ChatId>,
    Query(params): Query<ListingQuery>,
) -> Result<Json<ListMessagesResponse>, RequestError> {
    let response = match ListingMode::from_query(params)? {
        ListingMode::Offset { offset, limit } => {
            state
                .db_connection
                .list_messages_after(claims.user_id, chat_id, offset, limit)
                .await?
        }
        ListingMode::Page { limit, page } => {
            state
                .db_connection
                .list_messages(claims.user_id, chat_id, limit, page)
                .await?
        }
    };
    Ok(Json(response))
}

pub async fn send_message(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(chat_id): Path<ChatId>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<SendMessageResponse>), RequestError> {
    validate_message_text(&payload.text)?;
    let message_id = state
        .db_connection
        .send_message(claims.user_id, chat_id, &payload.text)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(SendMessageResponse { message_id }),
    ))
}
