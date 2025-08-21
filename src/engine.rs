use crate::config::{Config, ConfigManager};
use crate::openrouter::{Message, OpenRouterClient};
use crate::story::StoryLogger;
use crate::terminal::{TerminalInput, InputEvent};
use crate::tools::ToolExecutor;
use serde_json::json;
use uuid::Uuid;

pub struct LooEngine {
    openrouter_client: OpenRouterClient,
    tool_executor: ToolExecutor,
    story_logger: StoryLogger,
    config: Config,
    working_dir: String,
    session_id: String,
    messages: Vec<Message>,
}

impl LooEngine {
    pub async fn new(
        working_dir: String,
        cli_model: Option<String>,
        cli_verbose: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = ConfigManager::load_config()?;
        
        // Override model from CLI argument if provided
        if let Some(model) = cli_model {
            config.openrouter.model = model;
        }
        
        // Override verbose from CLI if provided
        if cli_verbose {
            config.preferences.verbose = true;
        }
        
        let openrouter_client = OpenRouterClient::new(config.clone()).await?;
        let tool_executor = ToolExecutor::new(working_dir.clone(), config.preferences.verbose);
        let session_id = Uuid::new_v4().to_string();
        let story_logger = StoryLogger::new(working_dir.clone(), session_id.clone());

        Ok(Self {
            openrouter_client,
            tool_executor,
            story_logger,
            config,
            working_dir,
            session_id,
            messages: Vec::new(),
        })
    }

    pub async fn start_session(&mut self, user_prompt: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting Break CLI with OpenRouter");
        println!("📁 Working directory: {}", self.working_dir);
        println!("🆔 Session ID: {}", self.session_id);

        // Add initial system message
        let system_message = Message {
            role: "system".to_string(),
            content: format!(
                "You are a coding assistant with filesystem and command execution tools. \
                Working directory: {}. Session ID: {}. \
                Use the available tools to complete coding tasks step by step. \
                Break down complex tasks into smaller, manageable steps. \
                Always verify your work and provide clear explanations of what you're doing.",
                self.working_dir, self.session_id
            ),
            tool_calls: None,
            tool_call_id: None,
        };

        // Add user message
        let user_message = Message {
            role: "user".to_string(),
            content: user_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        self.messages.push(system_message);
        self.messages.push(user_message);

        // Log the initial user prompt
        self.story_logger.log_user_prompt(user_prompt);

        // Process the initial prompt first
        self.process_conversation_turn().await?;

        // Now enter interactive chat mode
        println!("\n🎯 Interactive chat mode activated!");
        println!("💡 Tips:");
        println!("   • Press Ctrl+C three times to exit");
        println!("   • Press ESC three times to clear your input");
        println!("   • Use /clear to clear conversation context");
        println!("   • Type your messages and press Enter to send\n");

        let mut terminal_input = TerminalInput::new(self.working_dir.clone());

        // Interactive chat loop
        loop {
            match terminal_input.read_user_input().await? {
                InputEvent::UserInput(user_message) => {
                    // Check if this is a slash command
                    if user_message.trim().starts_with('/') {
                        match self.handle_slash_command(&user_message).await {
                            Ok(handled) => {
                                if handled {
                                    // Command was handled internally, continue to next input
                                    continue;
                                }
                                // Command not recognized, fall through to normal processing
                            }
                            Err(e) => {
                                println!("❌ Error executing command: {}", e);
                                continue;
                            }
                        }
                    }

                    // Add user message to conversation
                    let user_msg = Message {
                        role: "user".to_string(),
                        content: user_message.clone(),
                        tool_calls: None,
                        tool_call_id: None,
                    };
                    self.messages.push(user_msg);
                    self.story_logger.log_user_prompt(&user_message);

                    // Process the conversation turn
                    self.process_conversation_turn().await?;
                }
                InputEvent::ExitRequest(_count) => {
                    println!("\n👋 Goodbye! Saving session story...");
                    break;
                }
                InputEvent::ClearPrompt => {
                    // This is handled in the terminal input module
                    continue;
                }
                InputEvent::Interrupt => {
                    println!("\n⚠️ Interrupted");
                    continue;
                }
            }
        }

        // Generate story file at the end of session
        if let Err(e) = self.story_logger.write_story_file() {
            eprintln!("Warning: Failed to write story file: {}", e);
        } else {
            println!("📝 Session story saved to story.md");
        }

        Ok(())
    }

    async fn process_conversation_turn(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Main conversation processing loop
        loop {
            let response = self.openrouter_client
                .chat_completion(self.messages.clone())
                .await?;

            let assistant_message = &response.choices[0].message;
            self.messages.push(assistant_message.clone());

            // Log assistant response if it has content
            if !assistant_message.content.is_empty() {
                self.story_logger.log_assistant_response(&assistant_message.content);
            }

            // Check if there are tool calls to execute
            if let Some(tool_calls) = &assistant_message.tool_calls {
                if self.config.preferences.verbose || tool_calls.len() > 1 {
                    println!("🤖 LLM making {} tool calls", tool_calls.len());
                }
                
                for tool_call in tool_calls {
                    if self.config.preferences.verbose {
                        println!("  🔧 Executing: {}", tool_call.function.name);
                    } else {
                        println!("🔧 {}", tool_call.function.name);
                    }

                    // Log tool execution
                    let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                        .unwrap_or(serde_json::Value::Null);
                    self.story_logger.log_tool_execution(&tool_call.function.name, &args);
                    
                    match self.tool_executor.execute_tool_call(tool_call).await {
                        Ok(result) => {
                            if self.config.preferences.verbose {
                                println!("  ✅ Success: {}", result);
                            } else {
                                println!("  ✅");
                            }

                            // Check if the command was interrupted and log accordingly
                            let was_interrupted = if let Ok(json_result) = serde_json::from_str::<serde_json::Value>(&result) {
                                json_result["interrupted"].as_bool().unwrap_or(false)
                            } else {
                                false
                            };

                            if was_interrupted {
                                self.story_logger.log_process_interrupted();
                            }

                            // Log tool result
                            self.story_logger.log_tool_result(&tool_call.function.name, true, &result);
                            
                            // Create tool response message
                            let tool_message = Message {
                                role: "tool".to_string(),
                                content: result.clone(),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            };
                            self.messages.push(tool_message);
                            
                            // Check for completion
                            if tool_call.function.name == "complete" {
                                println!("🎉 Project completed successfully!");
                                return Ok(());
                            }
                        }
                        Err(e) => {
                            println!("  ❌ Error: {}", e);

                            // Log tool error
                            self.story_logger.log_tool_result(&tool_call.function.name, false, &e.to_string());
                            
                            // Create error tool response
                            let error_message = Message {
                                role: "tool".to_string(),
                                content: json!({"status": "error", "message": e.to_string()}).to_string(),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            };
                            self.messages.push(error_message);
                        }
                    }
                }
            } else {
                // No more tool calls, LLM provided final response
                if !assistant_message.content.is_empty() {
                    println!("🤖 {}", assistant_message.content);
                }
                break;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_session_id(&self) -> &str {
        &self.session_id
    }

    #[allow(dead_code)]
    pub fn get_working_dir(&self) -> &str {
        &self.working_dir
    }

    async fn handle_slash_command(&mut self, command: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let parts: Vec<&str> = command.trim().split_whitespace().collect();
        if parts.is_empty() {
            return Ok(false);
        }

        let cmd = parts[0].trim_start_matches('/');
        
        match cmd {
            "list-models" => {
                let search_term = if parts.len() > 1 {
                    parts[1..].join(" ")
                } else {
                    String::new()
                };
                
                // Print a newline first to ensure output starts at the beginning of a new line
                println!();
                
                match self.openrouter_client.list_models(&search_term).await {
                    Ok(models) => {
                        if models.is_empty() {
                            if search_term.is_empty() {
                                println!("📋 No models available");
                            } else {
                                println!("📋 No models found matching '{}'", search_term);
                            }
                        } else {
                            if search_term.is_empty() {
                                println!("📋 Available models ({}):", models.len());
                            } else {
                                println!("📋 Models matching '{}' ({}):", search_term, models.len());
                            }
                            
                            let max_items = std::cmp::min(models.len(), 10);
                            for model in models.iter().take(max_items) {
                                println!("  • {}", model);
                            }
                            
                            if models.len() > max_items {
                                println!("  ... and {} more", models.len() - max_items);
                            }
                        }
                        Ok(true)
                    }
                    Err(e) => {
                        println!("❌ Failed to fetch models: {}", e);
                        Ok(true) // Still handled, just with error
                    }
                }
            }
            "model" => {
                if parts.len() < 2 {
                    println!();
                    println!("❌ Usage: /model <model_name>");
                    println!("💡 Tip: Use /list-models to see available models");
                    return Ok(true);
                }
                
                let new_model = parts[1..].join(" ");
                let old_model = self.config.openrouter.model.clone();
                
                // Update the model in config
                self.config.openrouter.model = new_model.clone();
                
                // Update the OpenRouter client with new config
                match crate::openrouter::OpenRouterClient::new(self.config.clone()).await {
                    Ok(new_client) => {
                        self.openrouter_client = new_client;
                        println!();
                        println!("✅ Model changed from '{}' to '{}'", old_model, new_model);
                        Ok(true)
                    }
                    Err(e) => {
                        // Revert the model change on error
                        self.config.openrouter.model = old_model;
                        println!();
                        println!("❌ Failed to switch to model '{}': {}", new_model, e);
                        println!("💡 Tip: Use /list-models to see available models");
                        Ok(true) // Still handled, just with error
                    }
                }
            }
            "clear" => {
                // Count current messages (excluding system message)
                let message_count = self.messages.len().saturating_sub(1);
                
                // Keep only the system message (first message)
                if !self.messages.is_empty() {
                    let system_message = self.messages[0].clone();
                    self.messages.clear();
                    self.messages.push(system_message);
                }
                
                println!();
                println!("🧹 Conversation context cleared ({} messages removed)", message_count);
                println!("💡 The system prompt has been preserved");
                Ok(true)
            }
            _ => Ok(false) // Command not recognized
        }
    }
}