use clap::Parser;

use crate::config::AppConfig;

pub(crate) mod auth;
pub(crate) mod config;
pub(crate) mod database;
pub(crate) mod error;
pub(crate) mod models;
pub(crate) mod server;

#[cfg(test)]
mod tests;

#[derive(Debug, Parser)]
#[command(name = "walrus-server")]
struct CliArgs {
    #[arg(short, long, value_name = "HOST:PORT")]
    address: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = CliArgs::parse();
    let config = AppConfig::from_env_with_address(args.address)?;
    server::run_all(&config).await?;

    Ok(())
}
