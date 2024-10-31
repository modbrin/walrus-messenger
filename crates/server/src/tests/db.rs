use once_cell::sync::Lazy;
use tokio::sync::Mutex;

use crate::database::connection::{DbConfig, DbConnection};
use crate::models::chat::{ChatKind, ListChatsRequest};
use crate::models::message::ListMessagesRequest;
use crate::models::user::{InviteUserRequest, UserRole};

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

#[tokio::test]
async fn create_chat_with_self() {
    let _lock = SERIAL_LOCK.lock().await;
    let db = init_and_get_db().await;

    let origin_user_id = 1;
    let msg_a_1 = "Hi chat with self, here I will be sending messages for myself!";
    let msg_a_2 = "It seems lonely here :((";

    let user_a = db
        .invite_user(
            origin_user_id,
            InviteUserRequest {
                initial_password: "kobrabor".to_string(),
                alias: "user_a".to_string(),
                display_name: "User A".to_string(),
                role: UserRole::Regular,
            },
        )
        .await
        .unwrap();

    let chats = db
        .list_chats(&ListChatsRequest {
            user_id: 2,
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

    let user_b = db
        .invite_user(
            origin_user_id,
            InviteUserRequest {
                initial_password: "bobrabor".to_string(),
                alias: "user_b".to_string(),
                display_name: "User B".to_string(),
                role: UserRole::Regular,
            },
        )
        .await
        .unwrap();
    let messages = db
        .list_messages(&ListMessagesRequest {
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
    let msg_b_3 = "Let me check... Seems I have eated all of it :(";
    let msg_a_4 = "That's sad, I'm sad";
    let msg_a_5 = "Please let me know when you'll have more";
    let msg_b_6 = "Sure thing!";

    let user_a = db
        .invite_user(
            origin_user_id,
            InviteUserRequest {
                initial_password: "kobrabor".to_string(),
                alias: "its_benjamin".to_string(),
                display_name: "Benjamin Dover".to_string(),
                role: UserRole::Regular,
            },
        )
        .await
        .unwrap();
    let user_b = db
        .invite_user(
            origin_user_id,
            InviteUserRequest {
                initial_password: "bobrabor".to_string(),
                alias: "fuance".to_string(),
                display_name: "Le Baguette".to_string(),
                role: UserRole::Regular,
            },
        )
        .await
        .unwrap();
    let user_c = db
        .invite_user(
            origin_user_id,
            InviteUserRequest {
                initial_password: "borborbor".to_string(),
                alias: "thirdparty".to_string(),
                display_name: "Other User".to_string(),
                role: UserRole::Regular,
            },
        )
        .await
        .unwrap();

    let chat_id = db.create_private_chat(user_a, "fuance").await.unwrap();
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
    db.send_message(user_c, chat_id, "hi guys")
        .await
        .unwrap_err();
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
    let chat_id = db
        .create_private_chat(user_b, "its_benjamin")
        .await
        .unwrap_err();
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
