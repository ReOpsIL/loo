use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "loo")]
#[command(about = "LLM-based coding CLI that acts as a bridge between AI reasoning and filesystem/tools")]
#[command(version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
    
    /// Working directory for the session
    #[arg(long)]
    pub dir: Option<String>,
    
    /// Override default model from config
    #[arg(long)]
    pub model: Option<String>,
    
    /// Enable verbose output
    #[arg(long, short)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Manage LOO CLI configuration")]
    Config {
        #[command(subcommand)]
        config_command: ConfigCommand,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    #[command(about = "Initialize configuration with defaults")]
    Init,
    #[command(about = "Display current configuration")]
    Get,
    #[command(about = "Set a configuration value")]
    Set { 
        #[arg(help = "Configuration key (e.g., 'openrouter.model')")]
        key: String, 
        #[arg(help = "Configuration value")]
        value: String 
    },
    #[command(about = "Validate current configuration")]
    Validate,
}