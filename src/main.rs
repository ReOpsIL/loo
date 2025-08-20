mod autocomplete;
mod cli;
mod config;
mod engine;
mod openrouter;
mod story;
mod terminal;
mod tools;

use clap::Parser;
use cli::{Cli, Commands, ConfigCommand};
use config::ConfigManager;
use engine::LooEngine;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start { project, dir } => {
            let config = ConfigManager::load_config()?;
            
            // Determine working directory from CLI, config, or current directory
            let working_dir = dir
                .or_else(|| config.preferences.default_directory.clone())
                .unwrap_or_else(|| ".".to_string());
            
            let working_dir = fs::canonicalize(&working_dir)?
                .to_string_lossy()
                .to_string();

            let mut engine = LooEngine::new(working_dir, cli.model, cli.verbose).await?;
            engine.start_session(&project).await?;
        }
        Commands::Resume { session_id } => {
            println!("❌ Resume functionality not yet implemented for session: {}", session_id);
            println!("💡 This feature will be available in a future release");
            std::process::exit(1);
        }
        Commands::Config { config_command } => {
            match config_command {
                ConfigCommand::Init => {
                    ConfigManager::init_config()?;
                }
                ConfigCommand::Get => {
                    let config = ConfigManager::load_config()?;
                    let toml_string = toml::to_string_pretty(&config)?;
                    println!("Current configuration:\n{}", toml_string);
                }
                ConfigCommand::Set { key, value } => {
                    ConfigManager::set_config_value(&key, &value)?;
                }
                ConfigCommand::Validate => {
                    ConfigManager::validate_config()?;
                }
            }
        }
    }

    Ok(())
}
