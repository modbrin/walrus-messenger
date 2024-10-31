use crate::config::AppConfig;
use crate::server::session::chat_demo;

pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod server;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = AppConfig::from_yaml_file("config.yaml")?;
    server::run_all(&config).await?;

    Ok(())
}
