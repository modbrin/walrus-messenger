use serde::Serialize;

use crate::models::user::UserId;

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

#[derive(Clone, Debug)]
pub struct ListChatsRequest {
    pub user_id: UserId,
    pub page_size: i32,
    pub page_num: i32,
}

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct ChatResponse {
    pub id: ChatId,
    pub display_name: Option<String>,
    pub kind: ChatKind,
}

#[derive(Clone, Debug, Serialize)]
pub struct ListChatsResponse {
    pub chats: Vec<ChatResponse>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct IsUserInChatResponse {
    pub is_in_chat: bool,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PrivateChatExistsResponse {
    pub chat_exists: bool,
}
