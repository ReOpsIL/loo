mod cli;
mod commands;
mod config;
mod engine;
mod execution_stack;
mod llm_intent_recognition;
mod llm_schemas;
mod openrouter;
mod plan_display;
mod prompts;
mod semantic_engine;
mod story;
mod tools;

use clap::Parser;
use cli::{Cli, Commands, ConfigCommand};
use config::ConfigManager;
use semantic_engine::SemanticEngine;
use llm_intent_recognition::{LLMIntentRecognizer, UserIntent};
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Config { config_command }) => {
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
        None => {
            // Start the new semantic conversation system
            start_semantic_chat(cli).await?;
        }
    }

    Ok(())
}

async fn start_semantic_chat(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    let config = ConfigManager::load_config()?;
    
    // Determine working directory from CLI, config, or current directory
    let working_dir = cli.dir
        .or_else(|| config.preferences.default_directory.clone())
        .unwrap_or_else(|| ".".to_string());
    
    let working_dir = fs::canonicalize(&working_dir)?
        .to_string_lossy()
        .to_string();

    let mut engine = SemanticEngine::new(working_dir.clone(), cli.model, cli.verbose).await?;
    let intent_recognizer = LLMIntentRecognizer::new(engine.openrouter_client.clone());

    println!("🚀 Starting LOO with Semantic Intelligence");
    println!("📁 Working directory: {}", working_dir);
    println!("🆔 Session ID: {}", engine.session_id);
    println!();
    println!("🎯 Intelligent conversation mode activated!");
    println!("💡 Tips:");
    println!("   • Just talk naturally - I'll understand what you want to do");
    println!("   • Say 'clear context' to reset our conversation");
    println!("   • Say 'change model to <name>' to switch AI models");
    println!("   • Say 'list models' to see available models");
    println!("   • Use @ for file path autocomplete (e.g., 'edit @src/main.rs')");
    println!("   • Press Ctrl+C three times to exit");
    println!();

    // Interactive chat loop with semantic understanding
    let mut exit_attempts = 0;
    
    loop {
        use inquire::Text;
        use crate::semantic_engine::CustomTextAutocomplete;
        
        let user_input = Text::new("💬 You:")
            .with_help_message("Speak naturally (Ctrl+C 3x to exit, Tab for autocomplete)")
            .with_autocomplete(CustomTextAutocomplete::new(working_dir.clone()))
            .prompt();

        match user_input {
            Ok(user_message) => {
                exit_attempts = 0;
                let user_message = user_message.trim();
                
                if user_message.is_empty() {
                    continue;
                }
                
                // Recognize user intent using LLM instead of parsing commands
                let intent = match intent_recognizer.recognize_intent(user_message).await {
                    Ok(intent) => intent,
                    Err(e) => {
                        println!("⚠️ Intent recognition failed: {}, using regular conversation", e);
                        UserIntent::RegularConversation(user_message.to_string())
                    }
                };
                
                match intent {
                    UserIntent::ClearContext => {
                        let result = engine.clear_context();
                        println!("{}", result);
                    }
                    UserIntent::ChangeModel(model) => {
                        match engine.change_model(&model).await {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("❌ {}", e),
                        }
                    }
                    UserIntent::ListModels(search_term) => {
                        let search = search_term.unwrap_or_default();
                        match engine.list_models(&search).await {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("❌ {}", e),
                        }
                    }
                    _ => {
                        // Process all other intents through semantic conversation
                        if let Err(e) = engine.process_conversation(user_message).await {
                            println!("❌ Error: {}", e);
                        }
                    }
                }
            }
            Err(inquire::InquireError::OperationCanceled) => {
                exit_attempts += 1;
                if exit_attempts >= 3 {
                    println!("\n👋 Goodbye! Saving session story...");
                    break;
                } else {
                    println!("\n⚠️ Press Ctrl+C {} more time(s) to exit", 3 - exit_attempts);
                    continue;
                }
            }
            Err(inquire::InquireError::OperationInterrupted) => {
                exit_attempts += 1;
                if exit_attempts >= 3 {
                    println!("\n👋 Goodbye! Saving session story...");
                    break;
                } else {
                    println!("\n⚠️ Press Ctrl+C {} more time(s) to exit", 3 - exit_attempts);
                    continue;
                }
            }
            Err(e) => {
                println!("❌ Input error: {}", e);
                exit_attempts = 0;
                continue;
            }
        }
    }

    // Generate story file at the end of session
    if let Err(e) = engine.story_logger.write_story_file() {
        eprintln!("Warning: Failed to write story file: {}", e);
    } else {
        println!("📝 Session story saved to story.md");
    }

    Ok(())
}
