use std::sync::Arc;

use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use tracing::info;

use crate::auth::token::{AuthPayload, Claims, TokenExchangePayload};
use crate::config::AppConfig;
use crate::error::RequestError;
use crate::server::state::AppState;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server.address.clone();
    let app = Router::new()
        // .route("/", get(client))
        .route("/protected", get(protected))
        .route("/login", post(login))
        // .route("/websocket", get(websocket_handler))
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

pub async fn protected(claims: Claims) -> impl IntoResponse {
    format!("Hello, {}!", claims.user_id)
}
