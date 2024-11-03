use std::fmt::Debug;
use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use sqlx::{Error as SqlxError, Executor, PgExecutor, Postgres, Row, Transaction};
use tracing::{error, info, instrument};

use crate::auth::token::TokenExchangePayload;
use crate::auth::utils::{
    generate_salt, generate_session_token, hash_password_sha256, new_access_token_expiration,
    new_refresh_token_expiration,
};
use crate::database::connection::DbConnection;
use crate::database::queries::{
    get_user_credentials_by_alias, get_user_id_by_alias, get_user_role, is_user_in_chat,
    private_chat_exists,
};
use crate::error::{RequestError, ValidationError};
use crate::models::chat::{
    AddMemberToChatRequest, ChatId, ChatKind, ChatRole, CreateChatRequest, IsUserInChatRequest,
    PrivateChatExistsRequest,
};
use crate::models::message::{CreateMessageRequest, MessageId};
use crate::models::session::SessionId;
use crate::models::user::{
    validate_user_alias, validate_user_display_name, validate_user_password, CreateUserRequest,
    InviteUserRequest, UserId, UserRole,
};

/// Number of sessions single account can have, older sessions will be silently removed when new are added,
/// old sessions are determined by `access_token_expires_at`
pub const MAX_SESSIONS_PER_USER: i32 = 100;

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
        let recipient_id = get_user_id_by_alias(self.pool(), recipient_alias)
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
        // TODO: should be cached?
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

    #[instrument(skip(self, password))]
    pub async fn login(
        &self,
        alias: &str,
        password: &str,
    ) -> Result<TokenExchangePayload, RequestError> {
        let mut transaction = self.pool().begin().await?;
        let Some(creds) = get_user_credentials_by_alias(transaction.as_mut(), alias).await? else {
            return Err(RequestError::BadCredentials);
        };
        if hash_password_sha256(password, creds.password_salt) != creds.password_hash {
            return Err(RequestError::BadCredentials);
        }
        let refresh_token = generate_session_token();
        let refresh_token_expires_at = new_refresh_token_expiration();
        let access_token = generate_session_token();
        let access_token_expires_at = new_access_token_expiration();
        let session_id = create_session(
            transaction.as_mut(),
            creds.user_id,
            &IpNetwork::from(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
            Some("Google Pixel"),
            Some("Android 6.0"),
            Some("Walrus Messenger for Android 0.0.1"),
            &refresh_token,
            &refresh_token_expires_at,
            &access_token,
            &access_token_expires_at,
        )
        .await?;
        trim_sessions_for_user(transaction.as_mut(), creds.user_id, MAX_SESSIONS_PER_USER).await?;
        transaction.commit().await?;
        info!("successful login");
        Ok(TokenExchangePayload::new(
            &session_id,
            refresh_token,
            refresh_token_expires_at,
            access_token,
            access_token_expires_at,
        ))
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

#[instrument(skip(executor))]
pub async fn create_session<'a, E: PgExecutor<'a>>(
    executor: E,
    user_id: UserId,
    ip: &IpNetwork,
    device_name: Option<&str>,
    os_version: Option<&str>,
    app_version: Option<&str>,
    refresh_token: &[u8],
    refresh_token_expires_at: &DateTime<Utc>,
    access_token: &[u8],
    access_token_expires_at: &DateTime<Utc>,
) -> Result<SessionId, SqlxError> {
    let result = sqlx::query(
        "
        INSERT INTO sessions (id, user_id, ip, first_seen_at, last_seen_at, device_name, os_version, app_version, refresh_token, refresh_token_expires_at, access_token, access_token_expires_at)
        VALUES (gen_random_uuid(), $1, $2, current_timestamp, current_timestamp, $3, $4, $5, $6, $7, $8, $9) RETURNING id;
    ",
    )
        .bind(user_id)
        .bind(ip)
        .bind(device_name)
        .bind(os_version)
        .bind(app_version)
        .bind(refresh_token)
        .bind(refresh_token_expires_at)
        .bind(access_token)
        .bind(access_token_expires_at)
        .fetch_one(executor)
        .await?
        .try_get("id")?;
    Ok(result)
}

#[instrument(skip(executor))]
pub async fn trim_sessions_for_user<'a, E: PgExecutor<'a>>(
    executor: E,
    user_id: UserId,
    max_sessions: i32,
) -> Result<(), SqlxError> {
    sqlx::query(
        "
        DELETE FROM sessions WHERE id IN (
            SELECT id FROM sessions WHERE user_id = $1 ORDER BY access_token_expires_at DESC OFFSET $2
        );
    ")
        .bind(user_id)
        .bind(max_sessions)
        .execute(executor)
        .await?;
    Ok(())
}
