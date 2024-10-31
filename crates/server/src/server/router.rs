use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;
use tracing::info;

use crate::auth::token::{authorize, protected};
use crate::config::AppConfig;
use crate::server::state::AppState;

pub async fn serve(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server.address.clone();
    let app = Router::new()
        // .route("/", get(client))
        .route("/protected", get(protected))
        .route("/login", post(authorize))
        // .route("/websocket", get(websocket_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("starting server on: {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
