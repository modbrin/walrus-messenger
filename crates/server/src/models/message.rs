use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::ValidationError;
use crate::models::user::UserId;

pub type MessageId = i64;
pub const MESSAGE_TEXT_MAX_LENGTH: usize = 4096;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct MessageResponse {
    pub id: MessageId,
    pub text: Option<String>,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub user_id: Option<UserId>,
    pub user_display_name: Option<String>,
    // pub resource_url: Option<ResourceId>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ListMessagesResponse {
    pub messages: Vec<MessageResponse>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct SendMessageRequest {
    pub text: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct SendMessageResponse {
    pub message_id: MessageId,
}

pub fn validate_message_text(text: &str) -> Result<(), ValidationError> {
    if text.trim().is_empty() {
        return Err(ValidationError::InvalidInput {
            value: text.to_string(),
            reason: "text should not be empty".to_string(),
        });
    }
    if text.len() > MESSAGE_TEXT_MAX_LENGTH {
        return Err(ValidationError::LimitExceeded {
            subject: "message text length".to_string(),
            unit: "character".to_string(),
            attempted: text.len(),
            limit: MESSAGE_TEXT_MAX_LENGTH,
        });
    }
    Ok(())
}
