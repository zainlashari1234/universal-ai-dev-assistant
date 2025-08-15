use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub ai: AIConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_request_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIConfig {
    pub model_path: PathBuf,
    pub model_name: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub context_window: usize,
    pub use_gpu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<PathBuf>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
                max_request_size: 10 * 1024 * 1024, // 10MB
            },
            ai: AIConfig {
                model_path: PathBuf::from("./models"),
                model_name: "codellama-7b-instruct".to_string(),
                max_tokens: 2048,
                temperature: 0.1,
                context_window: 4096,
                use_gpu: true,
            },
            database: DatabaseConfig {
                url: "sqlite://./data/uaida.db".to_string(),
                max_connections: 10,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                file: None,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        // Try to load from config file, fallback to defaults
        let config_path = std::env::var("UAIDA_CONFIG")
            .unwrap_or_else(|_| "./config.toml".to_string());

        if std::path::Path::new(&config_path).exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config file
            let default_config = Config::default();
            let toml_content = toml::to_string_pretty(&default_config)?;
            std::fs::write(&config_path, toml_content)?;
            Ok(default_config)
        }
    }

    pub fn save(&self, path: &str) -> Result<()> {
        let toml_content = toml::to_string_pretty(self)?;
        std::fs::write(path, toml_content)?;
        Ok(())
    }
}