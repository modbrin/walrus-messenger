use sqlx::{Error as SqlxError, Executor, PgExecutor, Postgres, Row, Transaction};
use tracing::{info, instrument};

use crate::models::user::{CreateUser, UserId};

#[instrument(skip_all)]
pub async fn create_user<'a, E: PgExecutor<'a>>(
    executor: E,
    user: &CreateUser,
) -> Result<UserId, SqlxError> {
    let result = sqlx::query(
        "
            INSERT INTO users (alias, display_name, password_salt, password_hash, created_at, role, invited_by)
            VALUES ($1, $2, $3, $4, current_timestamp, $5, $6) RETURNING id;
        ",
    )
        .bind(&user.alias)
        .bind(&user.display_name)
        .bind(&user.password_salt)
        .bind(&user.password_hash)
        .bind(&user.role)
        .bind(user.invited_by.as_ref())
        .fetch_one(executor)
        .await?
        .try_get("id")?;
    info!("created origin user with id: {}", result);
    Ok(result)
}
