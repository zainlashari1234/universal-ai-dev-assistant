use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub providers: HashMap<String, ProviderConfig>,
    pub preferences: PreferencesConfig,
    pub ui: UiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub url: String,
    pub timeout_seconds: u64,
    pub retry_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub api_key: Option<String>,
    pub priority: u8,
    pub preferred_models: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferencesConfig {
    pub default_language: Option<String>,
    pub default_model: Option<String>,
    pub default_provider: Option<String>,
    pub auto_save: bool,
    pub create_backups: bool,
    pub max_tokens: u32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub show_progress: bool,
    pub colored_output: bool,
    pub editor: String,
    pub pager: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut providers = HashMap::new();
        
        providers.insert("openrouter".to_string(), ProviderConfig {
            enabled: false,
            api_key: None,
            priority: 9,
            preferred_models: vec!["anthropic/claude-3.5-sonnet".to_string()],
        });
        
        providers.insert("openai".to_string(), ProviderConfig {
            enabled: false,
            api_key: None,
            priority: 8,
            preferred_models: vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()],
        });
        
        providers.insert("anthropic".to_string(), ProviderConfig {
            enabled: false,
            api_key: None,
            priority: 8,
            preferred_models: vec!["claude-3-5-sonnet-20241022".to_string()],
        });
        
        providers.insert("google".to_string(), ProviderConfig {
            enabled: false,
            api_key: None,
            priority: 7,
            preferred_models: vec!["gemini-pro".to_string()],
        });
        
        providers.insert("groq".to_string(), ProviderConfig {
            enabled: false,
            api_key: None,
            priority: 6,
            preferred_models: vec!["llama-3.1-70b-versatile".to_string()],
        });
        
        providers.insert("ollama".to_string(), ProviderConfig {
            enabled: true,
            api_key: None,
            priority: 3,
            preferred_models: vec!["qwen2.5-coder:7b".to_string()],
        });

        Self {
            server: ServerConfig {
                url: "http://localhost:8080".to_string(),
                timeout_seconds: 30,
                retry_attempts: 3,
            },
            providers,
            preferences: PreferencesConfig {
                default_language: None,
                default_model: None,
                default_provider: None,
                auto_save: true,
                create_backups: true,
                max_tokens: 1000,
                temperature: 0.7,
            },
            ui: UiConfig {
                theme: "default".to_string(),
                show_progress: true,
                colored_output: true,
                editor: std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string()),
                pager: std::env::var("PAGER").unwrap_or_else(|_| "less".to_string()),
            },
        }
    }
}

impl Config {
    pub fn load(config_path: Option<&Path>) -> Result<Self> {
        let config_file = if let Some(path) = config_path {
            path.to_path_buf()
        } else {
            Self::default_config_path()?
        };

        if config_file.exists() {
            let content = std::fs::read_to_string(&config_file)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            // Create default config
            let config = Self::default();
            config.save(&config_file)?;
            Ok(config)
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn default_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?
            .join("uaida");
        
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.toml"))
    }

    pub fn get_enabled_providers(&self) -> Vec<String> {
        self.providers
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(name, _)| name.clone())
            .collect()
    }

    pub fn get_provider_api_key(&self, provider: &str) -> Option<&String> {
        self.providers.get(provider)?.api_key.as_ref()
    }

    pub fn set_provider_api_key(&mut self, provider: &str, api_key: String) {
        if let Some(config) = self.providers.get_mut(provider) {
            config.api_key = Some(api_key);
            config.enabled = true;
        }
    }

    pub fn get_preferred_model(&self, provider: &str) -> Option<String> {
        self.providers
            .get(provider)?
            .preferred_models
            .first()
            .cloned()
    }
}