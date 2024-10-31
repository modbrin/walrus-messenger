use std::sync::Arc;

use crate::config::AppConfig;
use crate::server::state::AppState;

pub mod router;
pub mod session;
pub mod state;

pub async fn run_all(config: &AppConfig) -> anyhow::Result<()> {
    let app_state = Arc::new(AppState::try_init(config).await?);
    router::serve(app_state).await?;
    Ok(())
}
