use loo_cli::config::*;
use tempfile::TempDir;
use std::fs;
use std::env;

#[test]
fn test_config_default_values() {
    let config = Config::default();
    
    assert_eq!(config.openrouter.model, "meta-llama/llama-3.1-8b-instruct:free");
    assert_eq!(config.openrouter.base_url, "https://openrouter.ai/api/v1");
    assert_eq!(config.openrouter.api_key, None);
    
    assert_eq!(config.preferences.verbose, false);
    assert_eq!(config.preferences.auto_confirm, false);
    assert_eq!(config.preferences.default_directory, None);
    
    assert_eq!(config.tools.filesystem, true);
    assert_eq!(config.tools.commands, true);
    assert_eq!(config.tools.git, true);
    assert_eq!(config.tools.command_timeout, 300);
}

#[test]
fn test_config_serialization() {
    let config = Config::default();
    let toml_string = toml::to_string(&config).unwrap();
    
    assert!(toml_string.contains("[openrouter]"));
    assert!(toml_string.contains("[preferences]"));
    assert!(toml_string.contains("[tools]"));
    assert!(toml_string.contains("model = \"meta-llama/llama-3.1-8b-instruct:free\""));
}

#[test]
fn test_config_deserialization() {
    let toml_content = r#"
[openrouter]
api_key = "test-key"
model = "anthropic/claude-3.5-sonnet"
base_url = "https://api.example.com"

[preferences]
verbose = true
auto_confirm = true
default_directory = "/home/user/projects"

[tools]
filesystem = false
commands = true
git = false
command_timeout = 600
"#;

    let config: Config = toml::from_str(toml_content).unwrap();
    
    assert_eq!(config.openrouter.api_key, Some("test-key".to_string()));
    assert_eq!(config.openrouter.model, "anthropic/claude-3.5-sonnet");
    assert_eq!(config.openrouter.base_url, "https://api.example.com");
    
    assert_eq!(config.preferences.verbose, true);
    assert_eq!(config.preferences.auto_confirm, true);
    assert_eq!(config.preferences.default_directory, Some("/home/user/projects".to_string()));
    
    assert_eq!(config.tools.filesystem, false);
    assert_eq!(config.tools.commands, true);
    assert_eq!(config.tools.git, false);
    assert_eq!(config.tools.command_timeout, 600);
}

#[test]
fn test_config_manager_save_and_load() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("loo");
    fs::create_dir_all(&config_dir)?;
    
    // Temporarily set XDG_CONFIG_HOME to our temp directory
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create a test config
    let mut test_config = Config::default();
    test_config.openrouter.api_key = Some("test-api-key".to_string());
    test_config.openrouter.model = "test-model".to_string();
    test_config.preferences.verbose = true;
    
    // Save the config
    ConfigManager::save_config(&test_config)?;
    
    // Load the config back
    let loaded_config = ConfigManager::load_config()?;
    
    assert_eq!(loaded_config.openrouter.api_key, Some("test-api-key".to_string()));
    assert_eq!(loaded_config.openrouter.model, "test-model");
    assert_eq!(loaded_config.preferences.verbose, true);
    
    // Restore original XDG_CONFIG_HOME
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    
    Ok(())
}

#[test]
fn test_config_environment_variable_override() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let config_dir = temp_dir.path().join("loo");
    fs::create_dir_all(&config_dir)?;
    
    // Set up environment
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    let original_api_key = env::var("OPENROUTER_API_KEY").ok();
    let original_model = env::var("OPENROUTER_MODEL").ok();
    
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    env::set_var("OPENROUTER_API_KEY", "env-api-key");
    env::set_var("OPENROUTER_MODEL", "env-model");
    
    // Save a config with different values
    let mut config = Config::default();
    config.openrouter.api_key = Some("file-api-key".to_string());
    config.openrouter.model = "file-model".to_string();
    ConfigManager::save_config(&config)?;
    
    // Load config - should have environment variables override file values
    let loaded_config = ConfigManager::load_config()?;
    
    assert_eq!(loaded_config.openrouter.api_key, Some("env-api-key".to_string()));
    assert_eq!(loaded_config.openrouter.model, "env-model");
    
    // Restore environment
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    match original_api_key {
        Some(val) => env::set_var("OPENROUTER_API_KEY", val),
        None => env::remove_var("OPENROUTER_API_KEY"),
    }
    match original_model {
        Some(val) => env::set_var("OPENROUTER_MODEL", val),
        None => env::remove_var("OPENROUTER_MODEL"),
    }
    
    Ok(())
}

#[test]
fn test_config_manager_set_value() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Initialize with default config
    ConfigManager::init_config()?;
    
    // Test setting various config values
    ConfigManager::set_config_value("openrouter.model", "new-model")?;
    ConfigManager::set_config_value("preferences.verbose", "true")?;
    ConfigManager::set_config_value("tools.command_timeout", "600")?;
    
    // Load and verify changes
    let config = ConfigManager::load_config()?;
    assert_eq!(config.openrouter.model, "new-model");
    assert_eq!(config.preferences.verbose, true);
    assert_eq!(config.tools.command_timeout, 600);
    
    // Test invalid key
    let result = ConfigManager::set_config_value("invalid.key", "value");
    assert!(result.is_err());
    
    // Test invalid value type
    let result = ConfigManager::set_config_value("preferences.verbose", "not-a-boolean");
    assert!(result.is_err());
    
    // Restore environment
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    
    Ok(())
}

#[test]
fn test_config_validation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    let original_api_key = env::var("OPENROUTER_API_KEY").ok();
    
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Test validation without API key
    env::remove_var("OPENROUTER_API_KEY");
    let result = ConfigManager::validate_config();
    assert!(result.is_err());
    
    // Test validation with API key in environment
    env::set_var("OPENROUTER_API_KEY", "test-key");
    let result = ConfigManager::validate_config();
    assert!(result.is_ok());
    
    // Test validation with API key in config file
    env::remove_var("OPENROUTER_API_KEY");
    ConfigManager::init_config()?;
    ConfigManager::set_config_value("openrouter.api_key", "file-key")?;
    let result = ConfigManager::validate_config();
    assert!(result.is_ok());
    
    // Restore environment
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    match original_api_key {
        Some(val) => env::set_var("OPENROUTER_API_KEY", val),
        None => env::remove_var("OPENROUTER_API_KEY"),
    }
    
    Ok(())
}

#[test]
fn test_config_init_idempotent() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // First init should create the config
    let result1 = ConfigManager::init_config();
    assert!(result1.is_ok());
    
    // Second init should not fail (idempotent)
    let result2 = ConfigManager::init_config();
    assert!(result2.is_ok());
    
    // Config should still be loadable
    let config = ConfigManager::load_config()?;
    assert_eq!(config.openrouter.model, "meta-llama/llama-3.1-8b-instruct:free");
    
    // Restore environment
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    
    Ok(())
}

#[test] 
fn test_config_partial_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let original_xdg = env::var("XDG_CONFIG_HOME").ok();
    env::set_var("XDG_CONFIG_HOME", temp_dir.path());
    
    // Create config directory
    let config_dir = temp_dir.path().join("loo");
    fs::create_dir_all(&config_dir)?;
    
    // Create a partial config file (missing some sections)
    let partial_config = r#"
[openrouter]
model = "custom-model"

[preferences]
verbose = true
"#;
    
    let config_path = config_dir.join("config.toml");
    fs::write(config_path, partial_config)?;
    
    // Should load with defaults for missing values
    let config = ConfigManager::load_config()?;
    
    assert_eq!(config.openrouter.model, "custom-model");
    assert_eq!(config.preferences.verbose, true);
    // Should use defaults for missing values
    assert_eq!(config.tools.filesystem, true);
    assert_eq!(config.tools.command_timeout, 300);
    assert_eq!(config.openrouter.base_url, "https://openrouter.ai/api/v1");
    
    // Restore environment
    match original_xdg {
        Some(val) => env::set_var("XDG_CONFIG_HOME", val),
        None => env::remove_var("XDG_CONFIG_HOME"),
    }
    
    Ok(())
}