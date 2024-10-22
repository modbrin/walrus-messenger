use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use crate::database::connection::{DbConfig, DbConnection};
use crate::models::chat::ListChatsRequest;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::chat::ChatKind;
    use crate::models::message::ListMessagesRequest;

    #[tokio::test]
    async fn create_chat_with_self() {
        tracing_subscriber::fmt::init();

        let config = DbConfig::development("walrus_db", "walrus_guest", "walruspass");
        let db = DbConnection::connect(&config).await.unwrap();
        db.drop_schema().await.unwrap();
        db.init_schema().await.unwrap();

        let chat_id = db.create_with_self_chat(1).await.unwrap();
        let msg1 = "Hello, saved messages!";
        let msg2 = "Saving this another text for later :)";
        db.send_message(1, chat_id, msg1).await.unwrap();
        db.send_message(1, chat_id, msg2).await.unwrap();

        let chats = db
            .list_chats(&ListChatsRequest {
                user_id: 1,
                page_size: 100,
                page_num: 1,
            })
            .await
            .unwrap()
            .chats;
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].id, chat_id);
        assert_eq!(chats[0].display_name, None);
        assert_eq!(chats[0].kind, ChatKind::WithSelf);

        let messages = db
            .list_messages(&ListMessagesRequest {
                user_id: 1,
                chat_id: 1,
                page_size: 100,
                page_num: 1,
            })
            .await
            .unwrap()
            .messages;
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text.as_deref(), Some(msg1));
        assert_eq!(messages[1].text.as_deref(), Some(msg2));
    }
}
