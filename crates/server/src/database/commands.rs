use crate::database::connection::DbConnection;
use crate::models::chat::{AddMemberToChatRequest, ChatId, ChatKind, ChatRole, CreateChatRequest};
use crate::models::message::{CreateMessageRequest, MessageId};
use crate::models::user::{CreateUser, UserId};
use sqlx::{Error as SqlxError, Executor, PgExecutor, Postgres, Row, Transaction};
use std::fmt::Debug;
use tracing::{info, instrument};

impl DbConnection {
    pub async fn invite_user(&self) -> Result<(), SqlxError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn create_with_self_chat(&self, caller: UserId) -> Result<ChatId, SqlxError> {
        let mut transaction = self.pool().begin().await?;
        let chat_id = create_chat(
            transaction.as_mut(),
            &CreateChatRequest {
                kind: ChatKind::WithSelf,
                description: None,
                display_name: None,
            },
        )
        .await?;
        add_member_to_chat(
            transaction.as_mut(),
            &AddMemberToChatRequest {
                chat_id,
                user_id: caller,
                role: ChatRole::Owner,
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(chat_id)
    }

    #[instrument(skip(self))]
    pub async fn create_private_chat(
        &self,
        caller: UserId,
        recipient: UserId,
    ) -> Result<ChatId, SqlxError> {
        let mut transaction = self.pool().begin().await?;
        let chat_id = create_chat(
            transaction.as_mut(),
            &CreateChatRequest {
                kind: ChatKind::Private,
                description: None,
                display_name: None,
            },
        )
        .await?;
        add_member_to_chat(
            transaction.as_mut(),
            &AddMemberToChatRequest {
                chat_id,
                user_id: caller,
                role: ChatRole::Member,
            },
        )
        .await?;
        add_member_to_chat(
            transaction.as_mut(),
            &AddMemberToChatRequest {
                chat_id,
                user_id: recipient,
                role: ChatRole::Member,
            },
        )
        .await?;
        transaction.commit().await?;
        Ok(chat_id)
    }

    #[instrument(skip(self))]
    pub async fn create_group_chat(&self) -> Result<(), SqlxError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn create_channel_chat(&self) -> Result<(), SqlxError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn send_message(
        &self,
        caller: UserId,
        chat_id: ChatId,
        text: impl Into<String> + Debug,
    ) -> Result<MessageId, SqlxError> {
        // TODO: check if member in chat
        let message_id = create_message(
            self.pool(),
            &CreateMessageRequest {
                user_id: caller,
                chat_id,
                text: Some(text.into()),
                resource_id: None,
                reply_to: None,
            },
        )
        .await?;
        info!("sent message in chat");
        Ok(message_id)
    }
}

#[instrument(skip(executor))]
pub async fn create_user<'a, E: PgExecutor<'a>>(
    executor: E,
    user: &CreateUser,
) -> Result<UserId, SqlxError> {
    let result = sqlx::query("
        INSERT INTO users (alias, display_name, password_salt, password_hash, role, invited_by, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, current_timestamp) RETURNING id;
    ")
    .bind(&user.alias)
    .bind(&user.display_name)
    .bind(&user.password_salt)
    .bind(&user.password_hash)
    .bind(&user.role)
    .bind(&user.invited_by)
    .fetch_one(executor)
    .await?
    .try_get("id")?;
    info!("created user with id: {}", result);
    Ok(result)
}

#[instrument(skip(executor))]
pub async fn create_chat<'a, E: PgExecutor<'a>>(
    executor: E,
    chat: &CreateChatRequest,
) -> Result<ChatId, SqlxError> {
    let result = sqlx::query(
        "
        INSERT INTO chats (display_name, description, kind, created_at)
        VALUES ($1, $2, $3, current_timestamp) RETURNING id;
    ",
    )
    .bind(&chat.display_name)
    .bind(&chat.description)
    .bind(&chat.kind)
    .fetch_one(executor)
    .await?
    .try_get("id")?;
    info!("created new chat");
    Ok(result)
}

#[instrument(skip(executor))]
pub async fn add_member_to_chat<'a, E: PgExecutor<'a>>(
    executor: E,
    add: &AddMemberToChatRequest,
) -> Result<(), SqlxError> {
    sqlx::query(
        "
        INSERT INTO chats_members (user_id, chat_id, role)
        VALUES ($1, $2, $3);
    ",
    )
    .bind(&add.user_id)
    .bind(&add.chat_id)
    .bind(&add.role)
    .execute(executor)
    .await?;
    info!("added member to chat");
    Ok(())
}

#[instrument(skip(executor))]
pub async fn create_message<'a, E: PgExecutor<'a>>(
    executor: E,
    message: &CreateMessageRequest,
) -> Result<MessageId, SqlxError> {
    let result = sqlx::query(
        "
        INSERT INTO messages (chat_id, user_id, text, reply_to, resource_id, created_at)
        VALUES ($1, $2, $3, $4, $5, current_timestamp) RETURNING id;
    ",
    )
    .bind(&message.chat_id)
    .bind(&message.user_id)
    .bind(&message.text)
    .bind(&message.reply_to)
    .bind(&message.resource_id)
    .fetch_one(executor)
    .await?
    .try_get("id")?;
    Ok(result)
}
