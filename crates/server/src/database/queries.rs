use crate::database::connection::DbConnection;
use crate::models::chat::{ChatResponse, ListChatsRequest, ListChatsResponse};
use crate::models::message::{
    CreateMessageRequest, ListMessagesRequest, ListMessagesResponse, MessageId, MessageResponse,
};
use crate::models::user::UserId;
use futures::{StreamExt, TryStreamExt};
use sqlx::{Error as SqlxError, PgExecutor};
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
    ) -> Result<ListMessagesResponse, SqlxError> {
        list_messages_for_user(self.pool(), request).await
    }
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
pub async fn list_messages_for_user<'a, E: PgExecutor<'a>>(
    executor: E,
    request: &ListMessagesRequest,
) -> Result<ListMessagesResponse, SqlxError> {
    let messages: Vec<MessageResponse> = sqlx::query_as(
        "
    SELECT
        messages.id AS id, messages.text AS text, messages.created_at AS created_at, messages.edited_at AS edited_at,
        users.display_name AS user_display_name
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
