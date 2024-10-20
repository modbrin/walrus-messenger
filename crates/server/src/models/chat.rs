use chrono::{DateTime, Utc};

pub type ChatId = String;

#[derive(Clone, Debug, sqlx::Type)]
#[sqlx(type_name = "chat_kind")]
#[sqlx(rename_all = "snake_case")]
pub enum ChatKind {
    Private,
    Group,
}

#[derive(Clone, Debug)]
pub struct CreateChat {
    pub kind: ChatKind,
    pub name: String,
}

#[derive(Clone, Debug, sqlx::FromRow)]
pub struct Chat {
    pub id: ChatId,
    pub kind: ChatKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
}
