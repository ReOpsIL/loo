use dirs;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub openrouter: OpenRouterConfig,
    pub preferences: PreferencesConfig,
    pub tools: ToolsConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct OpenRouterConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub base_url: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PreferencesConfig {
    pub default_directory: Option<String>,
    pub verbose: bool,
    pub auto_confirm: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ToolsConfig {
    pub filesystem: bool,
    pub commands: bool,
    pub git: bool,
    pub command_timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openrouter: OpenRouterConfig {
                api_key: None,
                model: "meta-llama/llama-3.1-8b-instruct:free".to_string(),
                base_url: "https://openrouter.ai/api/v1".to_string(),
            },
            preferences: PreferencesConfig {
                default_directory: None,
                verbose: false,
                auto_confirm: false,
            },
            tools: ToolsConfig {
                filesystem: true,
                commands: true,
                git: true,
                command_timeout: 300,
            },
        }
    }
}

pub struct ConfigManager;

impl ConfigManager {
    pub fn config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not find config directory")?
            .join("loo");
        
        fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("config.toml"))
    }
    
    pub fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        
        if !config_path.exists() {
            return Ok(Config::default());
        }
        
        let config_content = fs::read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&config_content)?;
        
        // Override with environment variables
        if let Ok(api_key) = env::var("OPENROUTER_API_KEY") {
            config.openrouter.api_key = Some(api_key);
        }
        if let Ok(model) = env::var("OPENROUTER_MODEL") {
            config.openrouter.model = model;
        }
        
        Ok(config)
    }
    
    pub fn save_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        let toml_string = toml::to_string_pretty(config)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }
    
    pub fn init_config() -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path()?;
        
        if config_path.exists() {
            println!("Configuration file already exists at: {}", config_path.display());
            return Ok(());
        }
        
        let default_config = Config::default();
        Self::save_config(&default_config)?;
        
        println!("✅ Configuration initialized at: {}", config_path.display());
        println!("📝 Please edit the config file to set your OpenRouter API key:");
        println!("   openrouter.api_key = \"sk-or-v1-your-key-here\"");
        
        Ok(())
    }
    
    pub fn set_config_value(key: &str, value: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut config = Self::load_config()?;
        
        match key {
            "openrouter.api_key" => config.openrouter.api_key = Some(value.to_string()),
            "openrouter.model" => config.openrouter.model = value.to_string(),
            "openrouter.base_url" => config.openrouter.base_url = value.to_string(),
            "preferences.default_directory" => config.preferences.default_directory = Some(value.to_string()),
            "preferences.verbose" => config.preferences.verbose = value.parse()?,
            "preferences.auto_confirm" => config.preferences.auto_confirm = value.parse()?,
            "tools.filesystem" => config.tools.filesystem = value.parse()?,
            "tools.commands" => config.tools.commands = value.parse()?,
            "tools.git" => config.tools.git = value.parse()?,
            "tools.command_timeout" => config.tools.command_timeout = value.parse()?,
            _ => return Err(format!("Unknown config key: {}", key).into()),
        }
        
        Self::save_config(&config)?;
        println!("✅ Updated {}: {}", key, value);
        Ok(())
    }
    
    pub fn validate_config() -> Result<(), Box<dyn std::error::Error>> {
        let config = Self::load_config()?;
        
        // Check if API key is available
        let has_api_key = config.openrouter.api_key.is_some() 
            || env::var("OPENROUTER_API_KEY").is_ok();
        
        if has_api_key {
            println!("✅ Configuration is valid");
            println!("🔧 Model: {}", config.openrouter.model);
            println!("🔧 Base URL: {}", config.openrouter.base_url);
            Ok(())
        } else {
            println!("❌ OpenRouter API key not found");
            println!("💡 Set it in config: loo config set openrouter.api_key <your-key>");
            println!("💡 Or environment: export OPENROUTER_API_KEY=<your-key>");
            Err("Missing API key".into())
        }
    }
}