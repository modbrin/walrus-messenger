use std::string::ToString;

use sqlx::migrate::Migrator;
use sqlx::{Error as SqlxError, Postgres, Transaction};
use tracing::info;

use crate::auth::utils::hash_password;
use crate::config::{optional_env, ENV_ORIGIN_PASSWORD};
use crate::database::commands::{create_user, create_with_self_chat};
use crate::database::connection::DbConnection;
use crate::models::user::{CreateUserRequest, UserId, UserRole};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

fn origin_user_from_env() -> Result<CreateUserRequest, SqlxError> {
    let Some(password) = optional_env(ENV_ORIGIN_PASSWORD) else {
        return Err(SqlxError::Protocol(format!(
            "missing required env var `{ENV_ORIGIN_PASSWORD}` for initial origin-user bootstrap"
        )));
    };
    Ok(CreateUserRequest {
        alias: "origin".to_string(),
        display_name: "Origin User".to_string(),
        role: UserRole::Admin,
        password_hash: hash_password(&password),
        invited_by: None,
    })
}

impl DbConnection {
    pub async fn init_schema(&self) -> Result<(), SqlxError> {
        MIGRATOR.run(self.pool()).await?;
        info!("database migrations applied");
        self.ensure_origin_user_exists().await?;
        Ok(())
    }

    #[cfg(test)]
    pub async fn drop_schema(&self) -> Result<(), SqlxError> {
        // Revert all applied reversible migrations (versions > -1 includes 0-prefixed migration).
        MIGRATOR.undo(self.pool(), -1).await?;
        Ok(())
    }

    async fn ensure_origin_user_exists(&self) -> Result<(), SqlxError> {
        let origin_user_id = sqlx::query_scalar::<_, UserId>(
            "SELECT origin_user_id FROM system_state WHERE singleton = TRUE;",
        )
        .fetch_optional(self.pool())
        .await?;
        if let Some(origin_user_id) = origin_user_id {
            let origin_exists: bool =
                sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = $1);")
                    .bind(origin_user_id)
                    .fetch_one(self.pool())
                    .await?;
            if origin_exists {
                return Ok(());
            }
            return Err(SqlxError::Protocol(
                "system_state.origin_user_id points to missing user".to_string(),
            ));
        }

        let mut transaction = self.pool().begin().await?;
        create_origin_user(&mut transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
}

pub async fn create_origin_user(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), SqlxError> {
    let user = origin_user_from_env()?;
    let origin_user_id = create_user(
        transaction.as_mut(),
        &user.alias,
        &user.display_name,
        &user.password_hash,
        user.role,
        user.invited_by,
    )
    .await?;
    let _ = create_with_self_chat(transaction, origin_user_id).await?;
    sqlx::query(
        "
        INSERT INTO system_state (singleton, origin_user_id)
        VALUES (TRUE, $1);
        ",
    )
    .bind(origin_user_id)
    .execute(transaction.as_mut())
    .await?;
    info!("created origin user from {ENV_ORIGIN_PASSWORD} bootstrap secret");
    Ok(())
}
