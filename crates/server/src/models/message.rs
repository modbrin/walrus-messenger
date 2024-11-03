use chrono::{DateTime, Utc};

use crate::models::user::UserId;

pub type MessageId = i64;

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
