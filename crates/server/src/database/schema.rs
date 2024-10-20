use std::string::ToString;

use sqlx::{Error as SqlxError, Postgres, Transaction};
use tracing::{info, instrument};

use crate::auth::utils::{generate_salt, hash_password_sha256};
use crate::database::commands::create_user;
use crate::database::connection::DbConnection;
use crate::models::user::{CreateUser, UserRole};

fn default_origin_user() -> CreateUser {
    let salt = generate_salt();
    let hash = hash_password_sha256("changepassword", salt);
    CreateUser {
        alias: "origin".to_string(),
        display_name: "Origin User".to_string(),
        role: UserRole::Admin,
        password_hash: hash,
        password_salt: salt,
        invited_by: None,
    }
}

impl DbConnection {
    pub async fn create_all(&self) -> Result<(), SqlxError> {
        let mut transaction = self.pool().begin().await?;
        create_all_types(&mut transaction).await?;
        create_all_tables(&mut transaction).await?;
        create_origin_user(&mut transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
    pub async fn drop_all(&self) -> Result<(), SqlxError> {
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
    // info!("invoked create_all_types");
    sqlx::query("CREATE TYPE user_role AS ENUM ('admin', 'regular');")
        .execute(transaction.as_mut())
        .await?;
    sqlx::query("CREATE TYPE chat_kind AS ENUM ('private', 'group');")
        .execute(transaction.as_mut())
        .await?;
    Ok(())
}

#[instrument(skip_all)]
pub async fn drop_all_types(transaction: &mut Transaction<'_, Postgres>) -> Result<(), SqlxError> {
    let statements = [
        "DROP TYPE IF EXISTS chat_kind;",
        "DROP TYPE IF EXISTS user_role;",
    ];
    for statement in &statements {
        sqlx::query(statement).execute(transaction.as_mut()).await?;
    }
    Ok(())
}

#[instrument(skip_all)]
pub async fn create_all_tables(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SqlxError> {
    sqlx::query(
        "
            CREATE TABLE users (
                id              int PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
                alias           VARCHAR(30) NOT NULL UNIQUE,
                display_name    VARCHAR(30) NOT NULL,
                password_salt   BYTEA NOT NULL,
                password_hash   BYTEA NOT NULL,
                created_at      TIMESTAMP WITHOUT TIME ZONE NOT NULL,
                role            user_role NOT NULL,
                bio             VARCHAR(255),
                invited_by      int
            );
        ",
    )
    .execute(transaction.as_mut())
    .await?;
    sqlx::query(
        "
            CREATE TABLE chats (
                id                        uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
                display_name              VARCHAR(50),
                description               VARCHAR(255),
                kind                      chat_kind NOT NULL,
                created_at                TIMESTAMP WITHOUT TIME ZONE NOT NULL
            );
        ",
    )
    .execute(transaction.as_mut())
    .await?;
    sqlx::query(
        "
            CREATE TABLE chats_members (
                user_id             INTEGER NOT NULL REFERENCES users(id) ON UPDATE CASCADE ON DELETE CASCADE,
                chat_id uuid NOT    NULL REFERENCES chats(id) ON UPDATE CASCADE ON DELETE CASCADE,
                CONSTRAINT chat_user_pkey PRIMARY KEY (user_id, chat_id)
            );
        ",
    )
        .execute(transaction.as_mut())
        .await?;
    sqlx::query(
        "
            CREATE TABLE resources (
                id                      bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
                uploaded_by_user_id     INTEGER NOT NULL REFERENCES users(id),
                link                    VARCHAR(255) NOT NULL
            );
        ",
    )
    .execute(transaction.as_mut())
    .await?;
    sqlx::query(
        "
            CREATE TABLE messages (
                id           bigint PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
                user_id      int NOT NULL REFERENCES users(id),
                chat_id      uuid NOT NULL REFERENCES chats(id),
                text         VARCHAR(4096),
                resource_id  bigint REFERENCES resources(id),
                created_at   TIMESTAMP WITHOUT TIME ZONE NOT NULL,
                edited_at    TIMESTAMP WITHOUT TIME ZONE
            );
        ",
    )
    .execute(transaction.as_mut())
    .await?;
    Ok(())
}

#[instrument(skip_all)]
pub async fn drop_all_tables(transaction: &mut Transaction<'_, Postgres>) -> Result<(), SqlxError> {
    // info!("invoked create_all_tables");
    let statements = [
        "DROP TABLE IF EXISTS messages;",
        "DROP TABLE IF EXISTS resources;",
        "DROP TABLE IF EXISTS chats_members;",
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
    create_user(transaction.as_mut(), &default_origin_user())
        .await
        .map(|_| ())
}
