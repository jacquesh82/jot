use crate::error::CliError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub server_url: Option<String>,
    pub token: Option<String>,
    pub device_id: Option<String>,
    pub identity_id: Option<String>,
}

#[allow(dead_code)]
impl Config {
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("jot").join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            return Self::default();
        }
        let contents = std::fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&contents).unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), CliError> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CliError::Config(e.to_string()))?;
        }
        let contents = toml::to_string(self).map_err(|e| CliError::Config(e.to_string()))?;
        std::fs::write(&path, contents).map_err(|e| CliError::Config(e.to_string()))
    }

    pub fn server_url(&self) -> &str {
        self.server_url
            .as_deref()
            .unwrap_or("http://127.0.0.1:3000")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_when_missing() {
        let cfg = Config::default();
        assert_eq!(cfg.server_url(), "http://127.0.0.1:3000");
        assert!(cfg.token.is_none());
    }

    #[test]
    fn config_save_and_load_round_trip() {
        let cfg = Config {
            server_url: Some("http://127.0.0.1:4242".to_string()),
            token: Some("my-jwt".to_string()),
            device_id: Some("device-abc".to_string()),
            identity_id: Some("identity-xyz".to_string()),
        };
        let contents = toml::to_string(&cfg).unwrap();
        let loaded: Config = toml::from_str(&contents).unwrap();
        assert_eq!(loaded.server_url.as_deref(), Some("http://127.0.0.1:4242"));
        assert_eq!(loaded.token.as_deref(), Some("my-jwt"));
        assert_eq!(loaded.device_id.as_deref(), Some("device-abc"));
        assert_eq!(loaded.identity_id.as_deref(), Some("identity-xyz"));
    }
}
