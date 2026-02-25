use crate::error::{Error, Result};
use config::{Config as ConfigBuilder, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;

fn default_max_tool_iterations() -> u32 {
    50
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub model: String,
    pub api_key: Option<String>,
    pub provider: String,
    pub max_tokens: u32,
    pub temperature: f32,
    #[serde(default = "default_max_tool_iterations")]
    pub max_tool_iterations: u32, // default: 10
}

impl Config {
    pub async fn load() -> Result<Self> {
        dotenvy::dotenv().ok();

        let config_dir = Self::config_dir()?;
        let default_config_path = config_dir.join("default.toml");

        let builder = ConfigBuilder::builder()
            .add_source(File::from(default_config_path).required(false))
            .add_source(File::with_name("nanocode").required(false))
            .add_source(Environment::with_prefix("NANOCODE"))
            .build()
            .map_err(|e| Error::ConfigLoad(e.to_string()))?;

        builder
            .try_deserialize()
            .map_err(|e| Error::ConfigLoad(e.to_string()))
    }

    fn config_dir() -> Result<PathBuf> {
        let mut path = dirs::config_dir()
            .ok_or_else(|| Error::ConfigLoad("Could not find config directory".to_string()))?;
        path.push("nanocode");
        std::fs::create_dir_all(&path).map_err(|e| Error::ConfigLoad(e.to_string()))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_default_config() {
        let config = Config::load().await.unwrap();
        assert_eq!(config.model, "claude-3-haiku-20240307");
    }

    #[tokio::test]
    async fn test_config_loads_with_max_tool_iterations() {
        // Temporarily set environment variable to test config loading
        std::env::set_var("NANOCODE_MAX_TOOL_ITERATIONS", "15");
        let config = Config::load().await.unwrap();
        assert_eq!(config.max_tool_iterations, 15);
        std::env::remove_var("NANOCODE_MAX_TOOL_ITERATIONS");
    }
}
