use std::string::ToString;

use sqlx::{Error as SqlxError, Postgres, Transaction};
use tracing::instrument;

use crate::auth::utils::{generate_salt, hash_password_sha256};
use crate::database::commands::create_user;
use crate::database::connection::DbConnection;
use crate::models::user::{CreateUserRequest, UserRole};

fn default_origin_user() -> CreateUserRequest {
    let salt = generate_salt();
    let hash = hash_password_sha256("changepassword", salt);
    CreateUserRequest {
        alias: "origin".to_string(),
        display_name: "Origin User".to_string(),
        role: UserRole::Admin,
        password_hash: hash,
        password_salt: salt,
        invited_by: None,
    }
}

impl DbConnection {
    pub async fn init_schema(&self) -> Result<(), SqlxError> {
        let mut transaction = self.pool().begin().await?;
        create_all_types(&mut transaction).await?;
        create_all_tables(&mut transaction).await?;
        create_origin_user(&mut transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
    pub async fn drop_schema(&self) -> Result<(), SqlxError> {
        let mut transaction = self.pool().begin().await?;
        drop_all_tables(&mut transaction).await?;
        drop_all_types(&mut transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
}

#[instrument(skip_all)]
pub async fn create_all_types(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SqlxError> {
    let statements = [
        "CREATE TYPE user_role AS ENUM ('admin', 'regular');",
        "CREATE TYPE chat_kind AS ENUM ('with_self', 'private', 'group', 'channel');",
        "CREATE TYPE chat_role AS ENUM ('owner', 'moderator', 'member');",
    ];
    for statement in statements {
        sqlx::query(statement).execute(transaction.as_mut()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn drop_all_types(transaction: &mut Transaction<'_, Postgres>) -> Result<(), SqlxError> {
    let statements = [
        "DROP TYPE IF EXISTS chat_role;",
        "DROP TYPE IF EXISTS chat_kind;",
        "DROP TYPE IF EXISTS user_role;",
    ];
    for statement in statements {
        sqlx::query(statement).execute(transaction.as_mut()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn create_all_tables(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SqlxError> {
    let statements = [
        "
        CREATE TABLE users (
            id               int PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
            alias            VARCHAR(30) NOT NULL UNIQUE,
            display_name     VARCHAR(30) NOT NULL,
            password_salt    BYTEA NOT NULL,
            password_hash    BYTEA NOT NULL,
            created_at       TIMESTAMPTZ NOT NULL,
            role             user_role NOT NULL,
            bio              VARCHAR(255),
            invited_by       int REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL
        );
    ",
        "
        CREATE TABLE chats (
            id              bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
            display_name    VARCHAR(50),
            description     VARCHAR(255),
            kind            chat_kind NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL
        );
    ",
        "
        CREATE TABLE sessions (
            id              uuid PRIMARY KEY,
            user_id         int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
            ip              inet NOT NULL,
            first_seen_at   TIMESTAMPTZ NOT NULL,
            last_seen_at    TIMESTAMPTZ NOT NULL,
            device_name     VARCHAR(100),
            os_version      VARCHAR(100),
            app_version     VARCHAR(100),
            refresh_token             BYTEA NOT NULL,
            refresh_token_expires_at  TIMESTAMPTZ NOT NULL,
            access_token              BYTEA NOT NULL,
            access_token_expires_at   TIMESTAMPTZ NOT NULL,
            refresh_counter           int NOT NULL
        );
    ",
        "
        CREATE TABLE chats_members (
            chat_id   bigint NOT NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
            user_id   int NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
            role      chat_role NOT NULL,
            CONSTRAINT chat_user_pkey PRIMARY KEY (user_id, chat_id)
        );
    ",
        "
        CREATE TABLE resources (
            id                      bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
            uploaded_by_user_id     INTEGER NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
            url                     VARCHAR(255) NOT NULL
        );
    ",
        "
        CREATE TABLE messages (
            id           bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
            chat_id      bigint NOT NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
            user_id      int REFERENCES users(id) ON UPDATE CASCADE ON DELETE SET NULL,
            text         VARCHAR(4096),
            reply_to     bigint REFERENCES messages(id) ON UPDATE CASCADE ON DELETE SET NULL,
            resource_id  bigint REFERENCES resources(id) ON UPDATE CASCADE ON DELETE NO ACTION,
            created_at   TIMESTAMPTZ NOT NULL,
            edited_at    TIMESTAMPTZ
        );
    ",
    ];
    for statement in statements {
        sqlx::query(statement).execute(transaction.as_mut()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn drop_all_tables(transaction: &mut Transaction<'_, Postgres>) -> Result<(), SqlxError> {
    let statements = [
        "DROP TABLE IF EXISTS messages;",
        "DROP TABLE IF EXISTS resources;",
        "DROP TABLE IF EXISTS chats_members;",
        "DROP TABLE IF EXISTS sessions;",
        "DROP TABLE IF EXISTS chats;",
        "DROP TABLE IF EXISTS users;",
    ];
    for statement in &statements {
        sqlx::query(statement).execute(transaction.as_mut()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn create_origin_user(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SqlxError> {
    let user = default_origin_user();
    create_user(
        transaction.as_mut(),
        &user.alias,
        &user.display_name,
        &user.password_salt,
        &user.password_hash,
        user.role,
        user.invited_by,
    )
    .await
    .map(|_| ())
}
