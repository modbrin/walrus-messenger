use futures::{StreamExt, TryStreamExt};
use sha2::digest::typenum::private::IsGreaterPrivate;
use sqlx::{Error as SqlxError, PgExecutor, Row};
use tracing::{debug, error, info, instrument};

use crate::auth::token::SessionToken;
use crate::auth::utils::current_time;
use crate::database::connection::DbConnection;
use crate::database::utils::map_not_found_as_none;
use crate::error::{RequestError, SessionError, ValidationError};
use crate::models::chat::{
    ChatResponse, IsUserInChatRequest, IsUserInChatResponse, ListChatsRequest, ListChatsResponse,
    PrivateChatExistsRequest, PrivateChatExistsResponse,
};
use crate::models::message::{
    CreateMessageRequest, ListMessagesRequest, ListMessagesResponse, MessageId, MessageResponse,
};
use crate::models::session::{RefreshTokenResponse, ResolveSessionResponse, SessionId};
use crate::models::user::{
    GetUserCredentialsByAliasResponse, GetUserIdByAliasResponse, GetUserRoleResponse, UserId,
    UserRole,
};

impl DbConnection {
    pub async fn list_chats(
        &self,
        request: &ListChatsRequest,
    ) -> Result<ListChatsResponse, SqlxError> {
        list_chats_for_user(self.pool(), request).await
    }

    pub async fn list_messages(
        &self,
        request: &ListMessagesRequest,
    ) -> Result<ListMessagesResponse, RequestError> {
        if is_user_in_chat(
            self.pool(),
            &IsUserInChatRequest {
                chat_id: request.chat_id,
                user_id: request.user_id,
            },
        )
        .await?
        .is_in_chat
        {
            Ok(list_messages_for_user(self.pool(), request).await?)
        } else {
            Err(ValidationError::NotFound.into())
        }
    }

    pub async fn resolve_session(
        &self,
        session_id: &SessionId,
        access_token: &[u8],
    ) -> Result<UserId, SessionError> {
        let Some(token) = get_access_token(self.pool(), session_id)
            .await
            .map_err(|e| {
                error!("{e}");
                SessionError::Internal
            })?
        else {
            return Err(SessionError::TokenNotFound);
        };
        if access_token != token.access_token {
            return Err(SessionError::TokenNotFound);
        }
        if token.access_token_expires_at <= current_time() {
            return Err(SessionError::TokenExpired);
        }
        Ok(token.user_id)
    }
}

#[instrument(skip(executor))]
pub(super) async fn get_user_role<'a, E: PgExecutor<'a>>(
    executor: E,
    user_id: UserId,
) -> Result<GetUserRoleResponse, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT role FROM users WHERE id = $1;
    ",
    )
    .bind(&user_id)
    .fetch_one(executor)
    .await?;
    Ok(result)
}

#[instrument(skip(executor))]
pub(super) async fn get_user_id_by_alias<'a, E: PgExecutor<'a>>(
    executor: E,
    alias: &str,
) -> Result<GetUserIdByAliasResponse, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT id AS user_id FROM users WHERE alias = $1;
    ",
    )
    .bind(alias)
    .fetch_one(executor)
    .await?;
    Ok(result)
}

#[instrument(skip(executor))]
pub(super) async fn get_user_credentials_by_alias<'a, E: PgExecutor<'a>>(
    executor: E,
    alias: &str,
) -> Result<Option<GetUserCredentialsByAliasResponse>, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT id AS user_id, password_hash, password_salt FROM users WHERE alias = $1;
    ",
    )
    .bind(alias)
    .fetch_one(executor)
    .await;
    map_not_found_as_none(result)
}

#[instrument(skip(executor))]
pub(super) async fn list_chats_for_user<'a, E: PgExecutor<'a>>(
    executor: E,
    request: &ListChatsRequest,
) -> Result<ListChatsResponse, SqlxError> {
    let chats: Vec<ChatResponse> = sqlx::query_as(
        "
    SELECT
        chats.id AS id, chats.display_name AS display_name, chats.kind AS kind
    FROM
        chats_members JOIN chats ON chats_members.chat_id = chats.id
    WHERE
        chats_members.user_id = $1
    ORDER BY
        chats.id
    LIMIT $2 OFFSET ($3 - 1) * $2;
    ",
    )
    .bind(&request.user_id)
    .bind(&request.page_size)
    .bind(&request.page_num)
    .fetch_all(executor)
    .await?;
    Ok(ListChatsResponse { chats })
}

#[instrument(skip(executor))]
pub(super) async fn is_user_in_chat<'a, E: PgExecutor<'a>>(
    executor: E,
    request: &IsUserInChatRequest,
) -> Result<IsUserInChatResponse, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT EXISTS(SELECT 1 FROM chats_members WHERE chat_id = $1 AND user_id = $2) AS is_in_chat;
    ",
    )
    .bind(&request.chat_id)
    .bind(&request.user_id)
    .fetch_one(executor)
    .await?;
    Ok(result)
}

#[instrument(skip(executor))]
pub(super) async fn private_chat_exists<'a, E: PgExecutor<'a>>(
    executor: E,
    request: &PrivateChatExistsRequest,
) -> Result<PrivateChatExistsResponse, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT EXISTS(
        SELECT
            1
        FROM
            chats_members a JOIN chats_members b ON a.chat_id = b.chat_id AND a.user_id != b.user_id
        WHERE a.user_id = $1 AND b.user_id = $2
    ) as chat_exists;
    ",
    )
    .bind(&request.user_id_a)
    .bind(&request.user_id_b)
    .fetch_one(executor)
    .await?;
    Ok(result)
}

#[instrument(skip(executor))]
pub(super) async fn list_messages_for_user<'a, E: PgExecutor<'a>>(
    executor: E,
    request: &ListMessagesRequest,
) -> Result<ListMessagesResponse, SqlxError> {
    let messages: Vec<MessageResponse> = sqlx::query_as(
        "
    SELECT
        messages.id AS id, messages.text AS text, messages.created_at AS created_at, messages.edited_at AS edited_at,
        messages.user_id as user_id, users.display_name AS user_display_name
    FROM
        messages LEFT JOIN users ON messages.user_id = users.id
    WHERE
        messages.chat_id = $1
    ORDER BY
        messages.id
    LIMIT $2 OFFSET ($3 - 1) * $2;
    ",
    )
    .bind(&request.chat_id)
    .bind(&request.page_size)
    .bind(&request.page_num)
    .fetch_all(executor)
    .await?;
    Ok(ListMessagesResponse { messages })
}

#[instrument(skip(executor))]
pub(super) async fn get_access_token<'a, E: PgExecutor<'a>>(
    executor: E,
    session_id: &SessionId,
) -> Result<Option<ResolveSessionResponse>, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT user_id, access_token, access_token_expires_at FROM sessions WHERE id = $1;
    ",
    )
    .bind(session_id)
    .fetch_one(executor)
    .await;
    map_not_found_as_none(result)
}

#[instrument(skip(executor))]
pub(super) async fn get_refresh_token<'a, E: PgExecutor<'a>>(
    executor: E,
    session_id: &SessionId,
) -> Result<Option<RefreshTokenResponse>, SqlxError> {
    let result = sqlx::query_as(
        "
    SELECT refresh_token, refresh_token_expires_at, refresh_counter FROM sessions WHERE id = $1;
    ",
    )
    .bind(session_id)
    .fetch_one(executor)
    .await;
    map_not_found_as_none(result)
}
