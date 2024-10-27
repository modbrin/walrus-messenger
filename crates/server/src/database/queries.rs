use crate::database::connection::DbConnection;
use crate::error::{RequestError, ValidationError};
use crate::models::chat::{
    ChatResponse, IsUserInChatRequest, IsUserInChatResponse, ListChatsRequest, ListChatsResponse,
    PrivateChatExistsRequest, PrivateChatExistsResponse,
};
use crate::models::message::{
    CreateMessageRequest, ListMessagesRequest, ListMessagesResponse, MessageId, MessageResponse,
};
use crate::models::user::{GetUserIdByAliasResponse, GetUserRoleResponse, UserId, UserRole};
use futures::{StreamExt, TryStreamExt};
use sqlx::{Error as SqlxError, PgExecutor, Row};
use tracing::instrument;

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
}

#[instrument(skip(executor))]
pub async fn get_user_role<'a, E: PgExecutor<'a>>(
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
pub async fn get_user_by_alias<'a, E: PgExecutor<'a>>(
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
pub async fn list_chats_for_user<'a, E: PgExecutor<'a>>(
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
pub async fn is_user_in_chat<'a, E: PgExecutor<'a>>(
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
pub async fn private_chat_exists<'a, E: PgExecutor<'a>>(
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
pub async fn list_messages_for_user<'a, E: PgExecutor<'a>>(
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
