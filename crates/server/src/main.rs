use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::{Html, IntoResponse};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;
use tracing_subscriber::fmt::format;

struct AppState {
    users: Mutex<HashSet<String>>,
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (tx, _rx) = broadcast::channel(32);

    let app_state = Arc::new(AppState {
        tx,
        users: Mutex::new(HashSet::new()),
    });

    let app = Router::new()
        .route("/", get(client))
        .route("/websocket", get(websocket_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    info!("starting server on: {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| websocket(socket, state))
}

async fn websocket(ws: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = ws.split();

    let mut username = String::new();

    while let Some(Ok(Message::Text(name))) = receiver.next().await {
        match add_user(&state, &name) {
            Ok(normalized) => {
                username = normalized;
                break;
            }
            Err(e) => {
                let _ = sender.send(Message::Text(e)).await;
                return;
            }
        }
    }

    let mut rx = state.tx.subscribe();
    let msg = format!("{username} joined the chat");
    tracing::debug!("{msg}");
    let _ = state.tx.send(msg);

    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let _ = sender.send(Message::Text(msg)).await;
        }
    });

    let tx = state.tx.clone();
    let name = username.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(msg))) = receiver.next().await {
            let _ = tx.send(format!("{name}: {msg}"));
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }

    let msg = format!("{username} left the chat");
    tracing::debug!(msg);
    let _ = state.tx.send(msg);
}

async fn client() -> impl IntoResponse {
    Html(include_str!("../../../assets/client.html"))
}

fn add_user(state: &AppState, username: &str) -> Result<String, String> {
    let normalized = username.trim();
    let mut users = state.users.lock().unwrap();
    if users.contains(normalized) {
        return Err("username is already taken".to_string());
    }
    users.insert(normalized.to_string());
    Ok(normalized.to_string())
}

fn remove_user(state: &AppState, username: &str) {
    let _ = state.users.lock().unwrap().remove(username);
}
