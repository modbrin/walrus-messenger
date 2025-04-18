use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Error as SqlxError;
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DbConfig {
    pub username: String,
    pub password: String,
    pub dbname: String,
    pub address: Option<String>,
    pub max_connections: Option<u32>,
}

impl DbConfig {
    const ADDRESS_FALLBACK: &'static str = "localhost";
    const MAX_CONN_FALLBACK: u32 = 5;

    pub fn development(dbname: &str, username: &str, password: &str) -> Self {
        Self {
            dbname: dbname.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            address: None,
            max_connections: None,
        }
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap_or(Self::ADDRESS_FALLBACK)
    }

    pub fn get_url(&self) -> String {
        format!(
            "postgresql://{}:{}@{}/{}",
            self.username,
            self.password,
            self.address(),
            self.dbname,
        )
    }
    pub fn max_connections(&self) -> u32 {
        self.max_connections.unwrap_or(Self::MAX_CONN_FALLBACK)
    }
}

pub struct DbConnection {
    pool: PgPool,
}

impl DbConnection {
    pub async fn connect(config: &DbConfig) -> Result<Self, SqlxError> {
        debug!("Connecting to database at `{}`...", config.address());
        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections())
            .connect(&config.get_url())
            .await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}
