use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::message::MessageId;

pub type ChatId = i64;

#[derive(Clone, Debug, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "chat_kind")]
#[sqlx(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ChatKind {
    WithSelf,
    Private,
    Group,
    Channel,
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "chat_role")]
#[sqlx(rename_all = "snake_case")]
pub enum ChatRole {
    Owner,
    Moderator,
    Member,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ChatResponse {
    pub id: ChatId,
    pub display_name: Option<String>,
    pub kind: ChatKind,
    pub last_message_id: Option<MessageId>,
    pub last_message_text: Option<String>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub unread_count: i64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ListChatsResponse {
    pub chats: Vec<ChatResponse>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct MarkChatReadRequest {
    pub up_to_message_id: MessageId,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct IsUserInChatResponse {
    pub is_in_chat: bool,
}
