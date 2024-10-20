use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::database::connection::{DbConfig, DbConnection};
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

mod auth;
mod database;
mod error;
mod models;
mod server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    test_db().await;
}

async fn test_db() {
    let config = DbConfig::development("walrus_db", "walrus_guest", "walruspass");
    let db = DbConnection::connect(&config).await.unwrap();
    db.drop_all().await.unwrap();
    db.create_all().await.unwrap();

    // db.create_user("race_car_joe", "Brakus Merck", &[2; 16], &[8; 32]).await.unwrap();
}
