use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use tracing::info;

use crate::auth::token::{AuthPayload, Claims, RefreshPayload, TokenExchangePayload};
use crate::auth::utils::unpack_session_id_and_token;
use crate::error::RequestError;
use crate::models::user::WhoAmIResponse;
use crate::server::state::AppState;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server.address.clone();
    let app = Router::new()
        .route("/auth/whoami", get(whoami))
        .route("/auth/login", post(login))
        .route("/auth/refresh", post(refresh))
        .route("/auth/logout", post(logout))
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
