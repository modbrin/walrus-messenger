use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::auth::token::TokenExchangePayload;
use crate::auth::utils::unpack_session_id_and_token;
use crate::database::commands::MAX_SESSIONS_PER_USER;
use crate::database::connection::{DbConfig, DbConnection};
use crate::error::{RequestError, SessionError};
use crate::models::chat::{ChatKind, ListChatsRequest};
use crate::models::message::ListMessagesRequest;
use crate::models::user::{InviteUserRequest, UserId, UserRole};

/// Some tests can't run in parallel, prevent them from breaking each other's state
static SERIAL_LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

async fn init_and_get_db() -> DbConnection {
    let _ = tracing_subscriber::fmt::try_init();

    let config = DbConfig::development("walrus_db", "walrus_guest", "walruspass");
    let db = DbConnection::connect(&config).await.unwrap();
    db.drop_schema().await.unwrap();
    db.init_schema().await.unwrap();
    db
}

async fn invite_regular(db: &DbConnection, alias: &str, pass: &str, name: &str) -> UserId {
    let origin_user_id = 1;
    db.invite_user(
        origin_user_id,
        InviteUserRequest {
            initial_password: pass.to_string(),
            alias: alias.to_string(),
            display_name: name.to_string(),
            role: UserRole::Regular,
        },
    )
    .await
    .unwrap()
}

async fn resolve_session(
    db: &DbConnection,
    tokens: &TokenExchangePayload,
) -> Result<UserId, SessionError> {
    let packed_bytes = BASE64.decode(&tokens.access_token).unwrap();
    let (session_id, token) = unpack_session_id_and_token(&packed_bytes).unwrap();
    db.resolve_session(&session_id, token).await
}

#[tokio::test]
async fn create_chat_with_self() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let origin_user_id = 1;
    let msg_a_1 = "Hi chat with self, here I will be sending messages for myself!";
    let msg_a_2 = "It seems lonely here :((";

    let user_a = invite_regular(&db, "user_a", "passfora", "User A").await;

    let chats = db
        .list_chats(&ListChatsRequest {
            user_id: user_a,
            page_size: 100,
            page_num: 1,
        })
        .await
        .unwrap()
        .chats;
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0].id, 1);
    assert_eq!(chats[0].display_name, None);
    assert_eq!(chats[0].kind, ChatKind::WithSelf);

    let self_chat_a_id = chats[0].id;
    db.send_message(user_a, self_chat_a_id, msg_a_1)
        .await
        .unwrap();
    db.send_message(user_a, self_chat_a_id, msg_a_2)
        .await
        .unwrap();

    let messages = db
        .list_messages(&ListMessagesRequest {
            user_id: user_a,
            chat_id: self_chat_a_id,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap()
        .messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text.as_deref(), Some(msg_a_1));
    assert_eq!(messages[1].text.as_deref(), Some(msg_a_2));

    // try to read A's chat from B
    let user_b = invite_regular(&db, "user_b", "passforb", "User B").await;
    db.list_messages(&ListMessagesRequest {
        user_id: user_b,
        chat_id: self_chat_a_id,
        page_num: 1,
        page_size: 100,
    })
    .await
    .unwrap_err();
}

#[tokio::test]
async fn create_private_chat() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let origin_user_id = 1;
    let msg_a_1 = "Oh hi there baguette, just wanted to check if you still have those bakery?";
    let msg_b_2 = "Hi Ben!";
    let msg_b_3 = "Let me check... Seems I have eaten all of it :(";
    let msg_a_4 = "That's sad, I'm sad";
    let msg_a_5 = "Please let me know when you'll have more";
    let msg_b_6 = "Sure thing!";
    let msg_c_7 = "Hi all?";

    let (alias_a, alias_b, alias_c) = ("its_benjamin", "fuance", "thirdparty");
    let user_a = invite_regular(&db, alias_a, "kobrabor", "Benjamin Dover").await;
    let user_b = invite_regular(&db, alias_b, "bobrabor", "Le Baguette").await;
    let user_c = invite_regular(&db, alias_c, "borborbor", "Other User").await;

    let chat_id = db.create_private_chat(user_a, alias_b).await.unwrap();
    db.send_message(user_a, chat_id, msg_a_1).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_2).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_3).await.unwrap();
    db.send_message(user_a, chat_id, msg_a_4).await.unwrap();
    db.send_message(user_a, chat_id, msg_a_5).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_6).await.unwrap();
    let reading_a = db
        .list_messages(&ListMessagesRequest {
            user_id: user_a,
            chat_id,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(reading_a.messages.len(), 6);
    let reading_b = db
        .list_messages(&ListMessagesRequest {
            user_id: user_b,
            chat_id,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(reading_b.messages.len(), 6);
    assert_eq!(reading_a.messages[0].text.as_deref(), Some(msg_a_1));
    assert_eq!(reading_a.messages[1].text.as_deref(), Some(msg_b_2));
    assert_eq!(reading_a.messages[2].text.as_deref(), Some(msg_b_3));
    assert_eq!(reading_a.messages[3].text.as_deref(), Some(msg_a_4));
    assert_eq!(reading_a.messages[4].text.as_deref(), Some(msg_a_5));
    assert_eq!(reading_a.messages[5].text.as_deref(), Some(msg_b_6));

    // try to send and read messages from uninvited user
    db.send_message(user_c, chat_id, msg_c_7).await.unwrap_err();
    db.list_messages(&ListMessagesRequest {
        user_id: user_c,
        chat_id,
        page_num: 1,
        page_size: 100,
    })
    .await
    .unwrap_err();
    // check that number of messages in fact hasn't changed
    let reading_b = db
        .list_messages(&ListMessagesRequest {
            user_id: user_b,
            chat_id,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(reading_b.messages.len(), 6);

    // try to create same chat but in reverse
    let _chat_id = db.create_private_chat(user_b, alias_a).await.unwrap_err();
    let user_a_chats = db
        .list_chats(&ListChatsRequest {
            user_id: user_a,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(
        user_a_chats
            .chats
            .iter()
            .filter(|c| c.kind == ChatKind::Private)
            .count(),
        1
    );
    let user_b_chats = db
        .list_chats(&ListChatsRequest {
            user_id: user_b,
            page_num: 1,
            page_size: 100,
        })
        .await
        .unwrap();
    assert_eq!(
        user_b_chats
            .chats
            .iter()
            .filter(|c| c.kind == ChatKind::Private)
            .count(),
        1
    );
}

#[tokio::test]
async fn login_and_resolve_session() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias_a, pass_a, name_a) = ("existing_user_a", "existing_password_a", "User A");
    let (alias_b, pass_b, name_b) = ("existing_user_b", "existing_password_b", "User B");
    let user_id_a = invite_regular(&db, alias_a, pass_a, name_a).await;
    let user_id_b = invite_regular(&db, alias_b, pass_b, name_b).await;

    // invalid variants
    let result = db
        .login("non_existent", "wrong_password")
        .await
        .unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));
    let result = db.login("non_existent", pass_a).await.unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));
    let result = db.login(alias_a, "wrong_password").await.unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));
    let result = db.login(alias_a, pass_b).await.unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));
    let result = db.login(alias_b, pass_a).await.unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));

    // normal login
    let result_a = db.login(alias_a, pass_a).await.unwrap();
    let resolved_user_a = resolve_session(&db, &result_a).await.unwrap();
    assert_eq!(resolved_user_a, user_id_a);

    let result_b = db.login(alias_b, pass_b).await.unwrap();
    let resolved_user_b = resolve_session(&db, &result_b).await.unwrap();
    assert_eq!(resolved_user_b, user_id_b);
}

#[tokio::test]
async fn limit_sessions_count() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass, name) = ("existing_user_a", "existing_password_a", "User A");
    let _ = invite_regular(&db, alias, pass, name).await;

    let first_session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &first_session).await.unwrap();
    let second_session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &second_session).await.unwrap();

    for _i in 0..MAX_SESSIONS_PER_USER - 2 {
        let session = db.login(alias, pass).await.unwrap();
        let _ok = resolve_session(&db, &session).await.unwrap();
    }

    // creating session number MAX + 1, this should invalidate one (first) session
    let latest_session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &latest_session).await.unwrap();
    let _ok = resolve_session(&db, &second_session).await.unwrap();
    let _ok = resolve_session(&db, &first_session).await.unwrap_err();
}

#[tokio::test]
async fn logout() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass, name) = ("existing_user_a", "existing_pass_a", "User A");
    let _ = invite_regular(&db, alias, pass, name).await;

    let session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &session).await.unwrap();

    let packed_bytes = BASE64.decode(&session.access_token).unwrap();
    let (session_id, _token) = unpack_session_id_and_token(&packed_bytes).unwrap();
    db.logout(&session_id).await.unwrap();

    let err = resolve_session(&db, &session).await.unwrap_err();
    assert!(matches!(err, SessionError::TokenNotFound));
}

#[tokio::test]
async fn refresh_token() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass, name) = ("existing_user_a", "existing_pass_a", "User A");
    let _ = invite_regular(&db, alias, pass, name).await;

    let first_session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &first_session).await.unwrap();

    let packed_bytes = BASE64.decode(&first_session.refresh_token).unwrap();
    let (session_id, token) = unpack_session_id_and_token(&packed_bytes).unwrap();
    let second_session = db.refresh_session(&session_id, token).await.unwrap();
    assert_ne!(second_session.refresh_token, first_session.refresh_token);
    assert_ne!(second_session.access_token, first_session.access_token);

    let _ok = resolve_session(&db, &second_session).await.unwrap();
    resolve_session(&db, &first_session).await.unwrap_err();
}
