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

/// Generate detailed action plan for coding tasks
pub async fn handle_plan_command(_engine: &LooEngine, request: &str) -> CommandResult {
    if request.trim().is_empty() {
        return Err("Plan command requires a request description".into());
    }

    use crate::commands::PlanCommand;
    let plan_cmd = PlanCommand::new();
    
    match plan_cmd.execute(request.trim()).await {
        Ok(result) => Ok(result),
        Err(e) => Err(format!("Plan execution error: {}", e).into()),
    }
}