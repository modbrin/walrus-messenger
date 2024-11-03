use std::fs::read_to_string;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::database::connection::DbConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DbConfig,
}

impl AppConfig {
    pub fn from_yaml_file<P: Into<PathBuf>>(path: P) -> Result<Self, anyhow::Error> {
        let path = path.into();
        let content = read_to_string(&path).with_context(|| format!("path: {path:?}"))?;
        Ok(serde_yaml::from_str(&content)?)
    }
}
