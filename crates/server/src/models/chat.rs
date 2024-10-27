use crate::models::user::UserId;
use chrono::{DateTime, Utc};

pub type ChatId = i64;

#[derive(Clone, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "chat_kind")]
#[sqlx(rename_all = "snake_case")]
pub enum ChatKind {
    WithSelf,
    Private,
    Group,
    Channel,
}

#[derive(Clone, Debug, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "chat_role")]
#[sqlx(rename_all = "snake_case")]
pub enum ChatRole {
    Owner,
    Moderator,
    Member,
}

#[derive(Clone, Debug)]
pub struct CreateChatRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub kind: ChatKind,
}

#[derive(Clone, Debug)]
pub struct AddMemberToChatRequest {
    pub user_id: UserId,
    pub chat_id: ChatId,
    pub role: ChatRole,
}

#[derive(Clone, Debug)]
pub struct UpdateMemberChatRoleRequest {
    pub user_id: UserId,
    pub chat_id: ChatId,
    pub new_role: ChatRole,
}

#[derive(Clone, Debug)]
pub struct ListChatsRequest {
    pub user_id: UserId,
    pub page_size: i32,
    pub page_num: i32,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct ChatResponse {
    pub id: ChatId,
    pub display_name: Option<String>,
    pub kind: ChatKind,
}

#[derive(Clone, Debug)]
pub struct ListChatsResponse {
    pub chats: Vec<ChatResponse>,
}

#[derive(Clone, Debug)]
pub struct IsUserInChatRequest {
    pub user_id: UserId,
    pub chat_id: ChatId,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct IsUserInChatResponse {
    pub is_in_chat: bool,
}

#[derive(Clone, Debug)]
pub struct PrivateChatExistsRequest {
    pub user_id_a: UserId,
    pub user_id_b: UserId,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct PrivateChatExistsResponse {
    pub chat_exists: bool,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Chat {
    pub id: ChatId,
    pub kind: ChatKind,
    pub created_at: DateTime<Utc>,
}
