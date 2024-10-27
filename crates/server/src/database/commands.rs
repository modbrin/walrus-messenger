use crate::auth::utils::{generate_salt, hash_password_sha256};
use crate::database::connection::DbConnection;
use crate::database::queries::{
    get_user_by_alias, get_user_role, is_user_in_chat, private_chat_exists,
};
use crate::error::{RequestError, ValidationError};
use crate::models::chat::{
    AddMemberToChatRequest, ChatId, ChatKind, ChatRole, CreateChatRequest, IsUserInChatRequest,
    PrivateChatExistsRequest,
};
use crate::models::message::{CreateMessageRequest, MessageId};
use crate::models::user::{
    validate_user_alias, validate_user_display_name, validate_user_password, CreateUserRequest,
    InviteUserRequest, UserId, UserRole,
};
use sqlx::{Error as SqlxError, Executor, PgExecutor, Postgres, Row, Transaction};
use std::fmt::Debug;
use tracing::{error, info, instrument};

impl DbConnection {
    #[instrument(skip(self))]
    pub async fn invite_user(
        &self,
        caller: UserId,
        request: InviteUserRequest,
    ) -> Result<UserId, RequestError> {
        let mut transaction = self.pool().begin().await?;
        let current_role = get_user_role(transaction.as_mut(), caller).await?.role;
        let required_role = UserRole::Admin;
        if current_role != required_role {
            return Err(ValidationError::InsufficientPermissions {
                current: current_role,
                required: required_role,
            }
            .into());
        }
        validate_user_alias(&request.alias)?;
        validate_user_display_name(&request.display_name)?;
        validate_user_password(&request.initial_password)?;
        let password_salt = generate_salt();
        let password_hash = hash_password_sha256(&request.initial_password, password_salt);
        let creation_request = CreateUserRequest {
            invited_by: Some(caller),
            role: request.role,
            alias: request.alias,
            display_name: request.display_name,
            password_salt,
            password_hash,
        };
        let user_id = create_user(transaction.as_mut(), &creation_request).await?;
        let _ = create_with_self_chat(&mut transaction, user_id).await?;
        transaction.commit().await?;
        Ok(user_id)
    }

    #[instrument(skip(self))]
    pub async fn create_private_chat(
        &self,
        caller: UserId,
        recipient_alias: &str,
    ) -> Result<ChatId, RequestError> {
        let recipient_id = get_user_by_alias(self.pool(), recipient_alias)
            .await?
            .user_id;
        if private_chat_exists(
            self.pool(),
            &PrivateChatExistsRequest {
                user_id_a: caller,
                user_id_b: recipient_id,
            },
        )
        .await?
        .chat_exists
        {
            return Err(ValidationError::AlreadyExists.into());
        }
        let mut transaction = self.pool().begin().await?;
        let chat_id = create_private_chat(&mut transaction, caller, recipient_id).await?;
        transaction.commit().await?;
        Ok(chat_id)
    }

    #[instrument(skip(self))]
    pub async fn create_group_chat(&self) -> Result<(), RequestError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn create_channel_chat(&self) -> Result<(), RequestError> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn send_message(
        &self,
        caller: UserId,
        chat_id: ChatId,
        text: impl Into<String> + Debug,
    ) -> Result<MessageId, RequestError> {
        if is_user_in_chat(
            self.pool(),
            &IsUserInChatRequest {
                chat_id,
                user_id: caller,
            },
        )
        .await?
        .is_in_chat
        {
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
        } else {
            info!("attempt to send message but user is not in chat");
            Err(ValidationError::NotFound.into())
        }
    }
}

#[instrument(skip(executor))]
pub async fn create_user<'a, E: PgExecutor<'a>>(
    executor: E,
    user: &CreateUserRequest,
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

#[instrument(skip(transaction))]
pub async fn create_with_self_chat<'a>(
    transaction: &mut Transaction<'a, Postgres>,
    caller: UserId,
) -> Result<ChatId, SqlxError> {
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
    Ok(chat_id)
}

#[instrument(skip(transaction))]
pub async fn create_private_chat<'a>(
    transaction: &mut Transaction<'a, Postgres>,
    caller: UserId,
    recipient: UserId,
) -> Result<ChatId, SqlxError> {
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
    Ok(chat_id)
}
