use base64::prelude::BASE64_STANDARD as BASE64;
use base64::Engine;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::auth::token::TokenExchangePayload;
use crate::auth::utils::unpack_session_id_and_token;
use crate::config::ENV_ORIGIN_PASSWORD;
use crate::database::commands::MAX_SESSIONS_PER_USER;
use crate::database::connection::{DbConfig, DbConnection};
use crate::error::{RequestError, SessionError, ValidationError};
use crate::models::chat::{ChatId, ChatKind, ChatResponse};
use crate::models::session::SessionId;
use crate::models::user::UserId;

/// Some tests can't run in parallel, prevent them from breaking each other's state
static SERIAL_LOCK: Lazy<Mutex<()>> = Lazy::new(Mutex::default);
const TEST_ORIGIN_PASSWORD: &str = "test_origin_password";

async fn init_and_get_db() -> DbConnection {
    let _ = tracing_subscriber::fmt::try_init();

    let config = DbConfig::development("walrus_db", "walrus_guest", "walruspass");
    let db = DbConnection::connect(&config).await.unwrap();
    db.drop_schema().await.unwrap();
    std::env::set_var(ENV_ORIGIN_PASSWORD, TEST_ORIGIN_PASSWORD);
    db.init_schema().await.unwrap();
    db
}

async fn invite_regular(db: &DbConnection, alias: &str, pass: &str) -> UserId {
    let origin_user_id = 1;
    db.invite_user(origin_user_id, alias, pass).await.unwrap()
}

async fn resolve_session(
    db: &DbConnection,
    tokens: &TokenExchangePayload,
) -> Result<UserId, SessionError> {
    let packed_bytes = BASE64.decode(&tokens.access_token).unwrap();
    let (session_id, token) = unpack_session_id_and_token(&packed_bytes).unwrap();
    db.resolve_session(session_id, token).await
}

fn unpack_encoded_session_token(token_b64: &str) -> (SessionId, Vec<u8>) {
    let packed_bytes = BASE64.decode(token_b64).unwrap();
    let (session_id, token) = unpack_session_id_and_token(&packed_bytes).unwrap();
    (session_id, token.to_vec())
}

async fn list_user_chats(db: &DbConnection, user_id: UserId) -> Vec<ChatResponse> {
    db.list_chats(user_id, 100, 1).await.unwrap().chats
}

async fn find_matching_chats(
    db: &DbConnection,
    user_id: UserId,
    kind: ChatKind,
    display_name: Option<&str>,
) -> Vec<ChatResponse> {
    list_user_chats(db, user_id)
        .await
        .into_iter()
        .filter(|chat| chat.kind == kind && chat.display_name.as_deref() == display_name)
        .collect()
}

async fn find_chat_id(
    db: &DbConnection,
    user_id: UserId,
    kind: ChatKind,
    display_name: Option<&str>,
) -> ChatId {
    find_matching_chats(db, user_id, kind, display_name)
        .await
        .into_iter()
        .next()
        .unwrap_or_else(|| {
            panic!(
                "expected chat not found for user_id={user_id}, kind={kind:?}, display_name={display_name:?}"
            )
        })
        .id
}

async fn count_chats_by_kind(db: &DbConnection, user_id: UserId, kind: ChatKind) -> usize {
    list_user_chats(db, user_id)
        .await
        .iter()
        .filter(|chat| chat.kind == kind)
        .count()
}

#[tokio::test]
async fn create_chat_with_self() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let msg_a_1 = "Hi chat with self, here I will be sending messages for myself!";
    let msg_a_2 = "It seems lonely here :((";

    let user_a = invite_regular(&db, "user_a", "passfora").await;

    let chats = list_user_chats(&db, user_a).await;
    assert_eq!(chats.len(), 2);
    assert!(!find_matching_chats(&db, user_a, ChatKind::WithSelf, None)
        .await
        .is_empty());

    let self_chat_a_id = find_chat_id(&db, user_a, ChatKind::WithSelf, None).await;
    db.send_message(user_a, self_chat_a_id, msg_a_1)
        .await
        .unwrap();
    db.send_message(user_a, self_chat_a_id, msg_a_2)
        .await
        .unwrap();

    let messages = db
        .list_messages(user_a, self_chat_a_id, 100, 1)
        .await
        .unwrap()
        .messages;
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].text.as_deref(), Some(msg_a_1));
    assert_eq!(messages[1].text.as_deref(), Some(msg_a_2));

    // try to read A's chat from B
    let user_b = invite_regular(&db, "user_b", "passforb").await;
    db.list_messages(user_b, self_chat_a_id, 100, 1)
        .await
        .unwrap_err();
}

#[tokio::test]
async fn create_private_chat() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let msg_a_1 = "Oh hi there baguette, just wanted to check if you still have those bakery?";
    let msg_b_2 = "Hi Ben!";
    let msg_b_3 = "Let me check... Seems I have eaten all of it :(";
    let msg_a_4 = "That's sad, I'm sad";
    let msg_a_5 = "Please let me know when you'll have more";
    let msg_b_6 = "Sure thing!";
    let msg_c_7 = "Hi all?";

    let (alias_a, alias_b, alias_c) = ("its_benjamin", "fuance", "thirdparty");
    let user_a = invite_regular(&db, alias_a, "kobrabor").await;
    let user_b = invite_regular(&db, alias_b, "bobrabor").await;
    let user_c = invite_regular(&db, alias_c, "borborbor").await;

    let chat_id = find_chat_id(&db, user_a, ChatKind::Private, Some(alias_b)).await;
    db.send_message(user_a, chat_id, msg_a_1).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_2).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_3).await.unwrap();
    db.send_message(user_a, chat_id, msg_a_4).await.unwrap();
    db.send_message(user_a, chat_id, msg_a_5).await.unwrap();
    db.send_message(user_b, chat_id, msg_b_6).await.unwrap();
    let reading_a = db.list_messages(user_a, chat_id, 100, 1).await.unwrap();
    assert_eq!(reading_a.messages.len(), 6);
    let reading_b = db.list_messages(user_b, chat_id, 100, 1).await.unwrap();
    assert_eq!(reading_b.messages.len(), 6);
    assert_eq!(reading_a.messages[0].text.as_deref(), Some(msg_a_1));
    assert_eq!(reading_a.messages[1].text.as_deref(), Some(msg_b_2));
    assert_eq!(reading_a.messages[2].text.as_deref(), Some(msg_b_3));
    assert_eq!(reading_a.messages[3].text.as_deref(), Some(msg_a_4));
    assert_eq!(reading_a.messages[4].text.as_deref(), Some(msg_a_5));
    assert_eq!(reading_a.messages[5].text.as_deref(), Some(msg_b_6));

    // try to send and read messages from uninvited user
    db.send_message(user_c, chat_id, msg_c_7).await.unwrap_err();
    db.list_messages(user_c, chat_id, 100, 1).await.unwrap_err();
    // check that number of messages in fact hasn't changed
    let reading_b = db.list_messages(user_b, chat_id, 100, 1).await.unwrap();
    assert_eq!(reading_b.messages.len(), 6);

    // try to create same chat but in reverse
    let duplicate = db.create_private_chat(user_b, alias_a).await.unwrap_err();
    assert!(matches!(
        duplicate,
        RequestError::Validation(ValidationError::AlreadyExists)
    ));
    assert_eq!(count_chats_by_kind(&db, user_a, ChatKind::Private).await, 3);
    let user_a_chats = list_user_chats(&db, user_a).await;
    let user_a_private_chat = user_a_chats.iter().find(|c| c.id == chat_id).unwrap();
    assert_eq!(user_a_private_chat.display_name.as_deref(), Some(alias_b));

    assert_eq!(count_chats_by_kind(&db, user_b, ChatKind::Private).await, 3);
    let user_b_chats = list_user_chats(&db, user_b).await;
    let user_b_private_chat = user_b_chats.iter().find(|c| c.id == chat_id).unwrap();
    assert_eq!(user_b_private_chat.display_name.as_deref(), Some(alias_a));
}

#[tokio::test]
async fn invite_user_creates_private_chats_with_all_existing_users() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let origin_user_id = 1;
    let user_a = invite_regular(&db, "existing_a", "passfora").await;
    let user_b = invite_regular(&db, "existing_b", "passforb").await;

    let new_user_alias = "new_joiner";
    let new_user_id = db
        .invite_user(origin_user_id, new_user_alias, "passfornewjoiner")
        .await
        .unwrap();

    let new_user_chats = list_user_chats(&db, new_user_id).await;
    assert_eq!(new_user_chats.len(), 4);
    assert_eq!(
        count_chats_by_kind(&db, new_user_id, ChatKind::WithSelf).await,
        1
    );

    let mut private_peers: Vec<String> = new_user_chats
        .iter()
        .filter(|chat| chat.kind == ChatKind::Private)
        .map(|chat| chat.display_name.clone().unwrap())
        .collect();
    private_peers.sort();
    assert_eq!(
        private_peers,
        vec!["Origin User", "existing_a", "existing_b"]
    );

    assert!(
        !find_matching_chats(&db, user_a, ChatKind::Private, Some(new_user_alias))
            .await
            .is_empty()
    );
    assert!(
        !find_matching_chats(&db, user_b, ChatKind::Private, Some(new_user_alias))
            .await
            .is_empty()
    );
    assert!(
        !find_matching_chats(&db, origin_user_id, ChatKind::Private, Some(new_user_alias))
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn invite_user_requires_admin_role() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let inviter = invite_regular(&db, "not_admin", "passforadmin").await;

    let err = db
        .invite_user(inviter, "should_fail", "passfornewuser")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        RequestError::Validation(ValidationError::InsufficientPermissions { .. })
    ));
}

#[tokio::test]
async fn list_messages_pagination() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let user_a = invite_regular(&db, "pager_a", "pagerpassa").await;
    let _user_b = invite_regular(&db, "pager_b", "pagerpassb").await;
    let chat_id = find_chat_id(&db, user_a, ChatKind::Private, Some("pager_b")).await;

    db.send_message(user_a, chat_id, "msg_1").await.unwrap();
    db.send_message(user_a, chat_id, "msg_2").await.unwrap();
    db.send_message(user_a, chat_id, "msg_3").await.unwrap();
    db.send_message(user_a, chat_id, "msg_4").await.unwrap();
    db.send_message(user_a, chat_id, "msg_5").await.unwrap();

    let page_1 = db
        .list_messages(user_a, chat_id, 2, 1)
        .await
        .unwrap()
        .messages;
    assert_eq!(page_1.len(), 2);
    assert_eq!(page_1[0].text.as_deref(), Some("msg_1"));
    assert_eq!(page_1[1].text.as_deref(), Some("msg_2"));

    let page_2 = db
        .list_messages(user_a, chat_id, 2, 2)
        .await
        .unwrap()
        .messages;
    assert_eq!(page_2.len(), 2);
    assert_eq!(page_2[0].text.as_deref(), Some("msg_3"));
    assert_eq!(page_2[1].text.as_deref(), Some("msg_4"));

    let page_3 = db
        .list_messages(user_a, chat_id, 2, 3)
        .await
        .unwrap()
        .messages;
    assert_eq!(page_3.len(), 1);
    assert_eq!(page_3[0].text.as_deref(), Some("msg_5"));

    let after_3 = db
        .list_messages_after(user_a, chat_id, 3, 10)
        .await
        .unwrap()
        .messages;
    assert_eq!(after_3.len(), 2);
    assert_eq!(after_3[0].text.as_deref(), Some("msg_4"));
    assert_eq!(after_3[1].text.as_deref(), Some("msg_5"));
}

#[tokio::test]
async fn login_and_resolve_session() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias_a, pass_a) = ("existing_user_a", "existing_password_a");
    let (alias_b, pass_b) = ("existing_user_b", "existing_password_b");
    let user_id_a = invite_regular(&db, alias_a, pass_a).await;
    let user_id_b = invite_regular(&db, alias_b, pass_b).await;

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
async fn change_password() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass) = ("existing_user_a", "existing_password_a");
    let user_id = invite_regular(&db, alias, pass).await;
    let new_password = "updated_password_a";

    let current_session = db.login(alias, pass).await.unwrap();
    let (current_session_id, _token) = unpack_encoded_session_token(&current_session.access_token);
    let other_session = db.login(alias, pass).await.unwrap();

    let result = db
        .change_password(
            user_id,
            current_session_id,
            "wrong_current_password",
            new_password,
        )
        .await
        .unwrap_err();
    assert!(matches!(result, RequestError::BadCredentials));

    db.change_password(user_id, current_session_id, pass, new_password)
        .await
        .unwrap();

    let old_login_result = db.login(alias, pass).await.unwrap_err();
    assert!(matches!(old_login_result, RequestError::BadCredentials));

    let still_valid = resolve_session(&db, &current_session).await.unwrap();
    assert_eq!(still_valid, user_id);
    let revoked = resolve_session(&db, &other_session).await.unwrap_err();
    assert!(matches!(revoked, SessionError::TokenNotFound));

    let new_login_result = db.login(alias, new_password).await.unwrap();
    let resolved_user = resolve_session(&db, &new_login_result).await.unwrap();
    assert_eq!(resolved_user, user_id);
}

#[tokio::test]
async fn change_alias() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (old_alias, pass) = ("existing_user_a", "existing_password_a");
    let user_id = invite_regular(&db, old_alias, pass).await;
    let taken_alias = "existing_user_b";
    let _other_user = invite_regular(&db, taken_alias, "existing_password_b").await;

    let new_alias = "renamed_user_a";
    db.change_alias(user_id, new_alias).await.unwrap();

    let old_login_result = db.login(old_alias, pass).await.unwrap_err();
    assert!(matches!(old_login_result, RequestError::BadCredentials));

    let new_login_result = db.login(new_alias, pass).await.unwrap();
    let resolved_user = resolve_session(&db, &new_login_result).await.unwrap();
    assert_eq!(resolved_user, user_id);

    let duplicate_err = db.change_alias(user_id, taken_alias).await.unwrap_err();
    assert!(matches!(
        duplicate_err,
        RequestError::Validation(ValidationError::AlreadyExists)
    ));

    let invalid_err = db.change_alias(user_id, "bad alias").await.unwrap_err();
    assert!(matches!(
        invalid_err,
        RequestError::Validation(ValidationError::InvalidInput { .. })
    ));
}

#[tokio::test]
async fn change_display_name() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let user_a = invite_regular(&db, "existing_user_a", "existing_password_a").await;
    let user_b_alias = "existing_user_b";
    let user_b = invite_regular(&db, user_b_alias, "existing_password_b").await;

    assert!(
        !find_matching_chats(&db, user_a, ChatKind::Private, Some(user_b_alias))
            .await
            .is_empty()
    );

    let new_display_name = "Baker Ben";
    db.change_display_name(user_b, new_display_name)
        .await
        .unwrap();

    assert!(
        find_matching_chats(&db, user_a, ChatKind::Private, Some(user_b_alias))
            .await
            .is_empty()
    );
    assert!(
        !find_matching_chats(&db, user_a, ChatKind::Private, Some(new_display_name))
            .await
            .is_empty()
    );

    let user_b_login = db.login(user_b_alias, "existing_password_b").await.unwrap();
    let resolved_user_b = resolve_session(&db, &user_b_login).await.unwrap();
    assert_eq!(resolved_user_b, user_b);

    let empty_err = db.change_display_name(user_b, "").await.unwrap_err();
    assert!(matches!(
        empty_err,
        RequestError::Validation(ValidationError::InvalidInput { .. })
    ));

    let padded_err = db
        .change_display_name(user_b, " Display Name ")
        .await
        .unwrap_err();
    assert!(matches!(
        padded_err,
        RequestError::Validation(ValidationError::InvalidInput { .. })
    ));

    let too_long_display_name = "x".repeat(31);
    let too_long_err = db
        .change_display_name(user_b, &too_long_display_name)
        .await
        .unwrap_err();
    assert!(matches!(
        too_long_err,
        RequestError::Validation(ValidationError::InvalidInput { .. })
    ));
}

#[tokio::test]
async fn limit_sessions_count() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass) = ("existing_user_a", "existing_password_a");
    let _ = invite_regular(&db, alias, pass).await;

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

    let (alias, pass) = ("existing_user_a", "existing_pass_a");
    let _ = invite_regular(&db, alias, pass).await;

    let session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &session).await.unwrap();

    let (session_id, _token) = unpack_encoded_session_token(&session.access_token);
    db.logout(session_id).await.unwrap();

    let err = resolve_session(&db, &session).await.unwrap_err();
    assert!(matches!(err, SessionError::TokenNotFound));
}

#[tokio::test]
async fn refresh_token() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let (alias, pass) = ("existing_user_a", "existing_pass_a");
    let _ = invite_regular(&db, alias, pass).await;

    let first_session = db.login(alias, pass).await.unwrap();
    let _ok = resolve_session(&db, &first_session).await.unwrap();

    let (session_id, token) = unpack_encoded_session_token(&first_session.refresh_token);
    let second_session = db.refresh_session(session_id, &token).await.unwrap();
    assert_ne!(second_session.refresh_token, first_session.refresh_token);
    assert_ne!(second_session.access_token, first_session.access_token);

    let _ok = resolve_session(&db, &second_session).await.unwrap();
    resolve_session(&db, &first_session).await.unwrap_err();
}
