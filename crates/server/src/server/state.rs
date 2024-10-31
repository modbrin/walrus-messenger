use crate::config::AppConfig;
use crate::database::connection::DbConnection;

pub struct AppState {
    pub config: AppConfig,
    pub db_connection: DbConnection,
}

impl AppState {
    pub async fn try_init(config: &AppConfig) -> anyhow::Result<Self> {
        let db_connection = DbConnection::connect(&config.database).await?;
        Ok(Self {
            config: config.clone(),
            db_connection,
        })
    }
}
