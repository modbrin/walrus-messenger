use anyhow::{anyhow, Context};

use crate::database::connection::DbConfig;

const ENV_DB_USERNAME: &str = "WALRUS_DB_USERNAME";
const ENV_DB_PASSWORD: &str = "WALRUS_DB_PASSWORD";
const ENV_DB_NAME: &str = "WALRUS_DB_NAME";
const ENV_DB_ADDRESS: &str = "WALRUS_DB_ADDRESS";
const ENV_DB_MAX_CONNECTIONS: &str = "WALRUS_DB_MAX_CONNECTIONS";

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub address: String,
}

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DbConfig,
}

impl AppConfig {
    pub fn from_env_with_address(server_address: String) -> Result<Self, anyhow::Error> {
        if server_address.trim().is_empty() {
            return Err(anyhow!("server address cannot be empty"));
        }
        let max_connections = match optional_env(ENV_DB_MAX_CONNECTIONS) {
            Some(raw) => Some(
                raw.parse::<u32>()
                    .with_context(|| format!("invalid `{ENV_DB_MAX_CONNECTIONS}` value `{raw}`"))?,
            ),
            None => None,
        };
        Ok(Self {
            server: ServerConfig {
                address: server_address,
            },
            database: DbConfig {
                username: required_env(ENV_DB_USERNAME)?,
                password: required_env(ENV_DB_PASSWORD)?,
                dbname: required_env(ENV_DB_NAME)?,
                address: optional_env(ENV_DB_ADDRESS),
                max_connections,
            },
        })
    }
}

fn required_env(name: &str) -> Result<String, anyhow::Error> {
    std::env::var(name).with_context(|| format!("missing required env var `{name}`"))
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
