use crate::models::chat::ChatId;
use crate::models::resource::ResourceId;
use crate::models::user::UserId;
use chrono::{DateTime, Utc};

pub type MessageId = i64;

#[derive(Clone, Debug)]
pub struct CreateMessageRequest {
    pub chat_id: ChatId,
    pub user_id: UserId,
    pub text: Option<String>,
    pub reply_to: Option<MessageId>,
    pub resource_id: Option<ResourceId>,
}

#[derive(Clone, Debug)]
pub struct ListMessagesRequest {
    pub user_id: UserId,
    pub chat_id: ChatId,
    pub page_size: i32,
    pub page_num: i32,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct MessageResponse {
    pub id: MessageId,
    pub text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub user_id: Option<UserId>,
    pub user_display_name: Option<String>,
    // pub resource_url: Option<ResourceId>,
}

#[derive(Clone, Debug)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Message {
    pub id: MessageId,
    pub chat_id: ChatId,
    pub user_id: Option<UserId>,
    pub text: Option<String>,
    pub reply_to: Option<MessageId>,
    pub resource_id: Option<ResourceId>,
}
