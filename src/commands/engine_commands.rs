use crate::engine::LooEngine;
use crate::commands::registry::CommandResult;

/// Clear conversation context, keeping only the system message
pub async fn handle_clear_command(engine: &mut LooEngine) -> CommandResult {
    // Count current messages (excluding system message)
    let message_count = engine.messages.len().saturating_sub(1);
    
    // Keep only the system message (first message)
    if !engine.messages.is_empty() {
        let system_message = engine.messages[0].clone();
        engine.messages.clear();
        engine.messages.push(system_message);
    }
    
    Ok(format!("🧹 Conversation context cleared ({} messages removed)\n💡 The system prompt has been preserved", message_count))
}

/// List available models with optional filtering
pub async fn handle_list_models_command(engine: &LooEngine, args: &str) -> CommandResult {
    let search_term = args.trim();
    
    match engine.openrouter_client.list_models(search_term).await {
        Ok(models) => {
            if models.is_empty() {
                if search_term.is_empty() {
                    Ok("📋 No models available".to_string())
                } else {
                    Ok(format!("📋 No models found matching '{}'", search_term))
                }
            } else {
                let mut result = if search_term.is_empty() {
                    format!("📋 Available models ({}):\n", models.len())
                } else {
                    format!("📋 Models matching '{}' ({}):\n", search_term, models.len())
                };
                
                let max_items = std::cmp::min(models.len(), 10);
                for model in models.iter().take(max_items) {
                    result.push_str(&format!("  • {}\n", model));
                }
                
                if models.len() > max_items {
                    result.push_str(&format!("  ... and {} more", models.len() - max_items));
                }
                
                Ok(result)
            }
        }
        Err(e) => Err(format!("Failed to fetch models: {}", e).into())
    }
}

/// Change the current LLM model
pub async fn handle_model_command(engine: &mut LooEngine, args: &str) -> CommandResult {
    let new_model = args.trim();
    
    if new_model.is_empty() {
        return Err("Usage: /model <model_name>\n💡 Tip: Use /list-models to see available models".into());
    }
    
    let old_model = engine.config.openrouter.model.clone();
    
    // Update the model in config
    engine.config.openrouter.model = new_model.to_string();
    
    // Update the OpenRouter client with new config
    match crate::openrouter::OpenRouterClient::new(engine.config.clone()).await {
        Ok(new_client) => {
            engine.openrouter_client = new_client;
            Ok(format!("✅ Model changed from '{}' to '{}'", old_model, new_model))
        }
        Err(e) => {
            // Revert the model change on error
            engine.config.openrouter.model = old_model;
            Err(format!("Failed to switch to model '{}': {}\n💡 Tip: Use /list-models to see available models", new_model, e).into())
        }
    }
}

/// Generate detailed action plan for coding tasks and execute via stack
pub async fn handle_plan_command(engine: &mut LooEngine, request: &str) -> CommandResult {
    if request.trim().is_empty() {
        return Err("Plan command requires a request description".into());
    }

    println!("🎯 Generating plan and adding to execution stack...");

    use crate::commands::PlanCommand;
    let plan_cmd = PlanCommand::new();
    
    match plan_cmd.execute(request.trim()).await {
        Ok(result) => {
            // Display the generated plan
            println!("{}", result);
            
            // Also try to parse and push to execution stack if possible
            match plan_cmd.parse_plan_json(&result) {
                Ok(action_plan) => {
                    println!("\n📋 Converting plan to execution stack...");
                    let request_ids = engine.push_action_plan(action_plan);
                    println!("✅ Added {} action items to execution stack", request_ids.len());
                    
                    // Start stack execution if enabled
                    if engine.auto_execute_stack {
                        println!("\n🚀 Starting recursive execution...");
                        if let Err(e) = engine.start_stack_execution().await {
                            println!("❌ Stack execution error: {}", e);
                        }
                    } else {
                        println!("💡 Stack execution disabled. Use /stack-execute to run manually.");
                    }
                    
                    Ok(format!("{}\n\n📊 {}", result, engine.get_stack_status()))
                }
                Err(parse_err) => {
                    // If parsing fails, still push as a user prompt for decomposition
                    println!("⚠️ Could not parse structured plan, pushing as user request: {}", parse_err);
                    let request_id = engine.push_user_prompt(request.trim(), 3);
                    println!("📥 Pushed user prompt to stack: {}", request_id);
                    
                    if engine.auto_execute_stack {
                        println!("\n🚀 Starting recursive execution...");
                        if let Err(e) = engine.start_stack_execution().await {
                            println!("❌ Stack execution error: {}", e);
                        }
                    }
                    
                    Ok(format!("{}\n\n📊 {}", result, engine.get_stack_status()))
                }
            }
        }
        Err(e) => {
            // If plan generation fails, push as user prompt anyway
            println!("⚠️ Plan generation failed, pushing as user request for decomposition");
            let request_id = engine.push_user_prompt(request.trim(), 3);
            println!("📥 Pushed user prompt to stack: {}", request_id);
            
            if engine.auto_execute_stack {
                println!("\n🚀 Starting recursive execution...");
                if let Err(stack_err) = engine.start_stack_execution().await {
                    println!("❌ Stack execution error: {}", stack_err);
                }
            }
            
            Err(format!("Plan execution error: {}. Request added to execution stack for decomposition.", e).into())
        }
    }
}

/// Show execution stack status
pub async fn handle_stack_status_command(engine: &LooEngine, _args: &str) -> CommandResult {
    Ok(engine.get_stack_status())
}

/// Execute pending items in the stack
pub async fn handle_stack_execute_command(engine: &mut LooEngine, _args: &str) -> CommandResult {
    if !engine.execution_stack.has_pending_requests() {
        return Ok("📋 No pending requests in execution stack".to_string());
    }

    println!("🚀 Starting manual stack execution...");
    match engine.start_stack_execution().await {
        Ok(()) => Ok("✅ Stack execution completed successfully".to_string()),
        Err(e) => Err(format!("❌ Stack execution failed: {}", e).into()),
    }
}

/// Clear the execution stack
pub async fn handle_stack_clear_command(engine: &mut LooEngine, _args: &str) -> CommandResult {
    engine.clear_stack();
    Ok("🧹 Execution stack cleared".to_string())
}

/// Toggle automatic stack execution
pub async fn handle_stack_auto_command(engine: &mut LooEngine, args: &str) -> CommandResult {
    let enabled = match args.trim().to_lowercase().as_str() {
        "on" | "true" | "1" | "enable" | "enabled" => true,
        "off" | "false" | "0" | "disable" | "disabled" => false,
        "" => !engine.auto_execute_stack, // Toggle if no argument
        _ => return Err("Usage: /stack-auto [on|off]".into()),
    };
    
    engine.set_auto_execute(enabled);
    Ok(format!("🔄 Automatic stack execution: {}", if enabled { "enabled" } else { "disabled" }))
}

/// Push a user prompt to the stack
pub async fn handle_stack_push_command(engine: &mut LooEngine, args: &str) -> CommandResult {
    if args.trim().is_empty() {
        return Err("Usage: /stack-push <prompt> [priority]".into());
    }
    
    let parts: Vec<&str> = args.trim().splitn(2, ' ').collect();
    let prompt = if parts.len() > 1 { parts[0] } else { args.trim() };
    let priority = if parts.len() > 1 { 
        parts[1].parse().unwrap_or(3) 
    } else { 
        3 
    };
    
    let request_id = engine.push_user_prompt(prompt, priority);
    Ok(format!("📥 Pushed prompt to stack: {} (priority: {})", request_id, priority))
}