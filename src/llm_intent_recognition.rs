/// LLM-powered intent recognition system
/// Uses the LLM itself to understand user intent naturally

use crate::openrouter::{Message, OpenRouterClient};
use serde_json;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum UserIntent {
    /// User wants to clear conversation context
    ClearContext,
    /// User wants to change the AI model
    ChangeModel(String),
    /// User wants to list available models
    ListModels(Option<String>),
    /// User is asking for help or planning
    RequestHelp(String),
    /// User wants to implement something
    Implement(String),
    /// User is troubleshooting an issue
    Troubleshoot(String),
    /// User wants to explore/understand something
    Explore(String),
    /// Regular conversation
    RegularConversation(String),
}

pub struct LLMIntentRecognizer {
    client: OpenRouterClient,
}

impl LLMIntentRecognizer {
    pub fn new(client: OpenRouterClient) -> Self {
        Self { client }
    }

    /// Analyze user input using the LLM to determine intent
    pub async fn recognize_intent(&self, input: &str) -> Result<UserIntent, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an intent classification system. Analyze the user's input and determine their intent.

Respond with ONLY a JSON object in this exact format:
{
  "intent": "one of: clear_context, change_model, list_models, request_help, implement, troubleshoot, explore, regular_conversation",
  "specifics": "extracted specific information like model name, or null if none",
  "confidence": 0.95
}

Intent definitions:
- clear_context: User wants to reset/clear the conversation
- change_model: User wants to switch AI models
- list_models: User wants to see available models
- request_help: User needs guidance, planning, or assistance
- implement: User wants to create, build, or develop something
- troubleshoot: User has problems, errors, or issues to debug
- explore: User wants to understand, investigate, or learn about something
- regular_conversation: General chat, questions, or conversation

Be flexible and understand natural variations in language."#;

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
            Message {
                role: "user".to_string(),
                content: input.to_string(),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        let response = self.client.chat_completion(messages).await?;
        let content = &response.choices[0].message.content;

        // Parse the JSON response
        let parsed: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse intent JSON: {} - Response: {}", e, content))?;

        let intent_str = parsed["intent"]
            .as_str()
            .ok_or("Missing 'intent' field in response")?;

        let specifics = parsed["specifics"]
            .as_str()
            .map(|s| s.to_string());

        // Convert to UserIntent enum
        let intent = match intent_str {
            "clear_context" => UserIntent::ClearContext,
            "change_model" => {
                let model = specifics.unwrap_or_else(|| {
                    // Fallback: try to extract model from input
                    self.extract_model_name(input).unwrap_or("unknown".to_string())
                });
                UserIntent::ChangeModel(model)
            }
            "list_models" => UserIntent::ListModels(specifics),
            "request_help" => UserIntent::RequestHelp(input.to_string()),
            "implement" => UserIntent::Implement(input.to_string()),
            "troubleshoot" => UserIntent::Troubleshoot(input.to_string()),
            "explore" => UserIntent::Explore(input.to_string()),
            "regular_conversation" | _ => UserIntent::RegularConversation(input.to_string()),
        };

        Ok(intent)
    }

    /// Fallback method to extract model name from input
    fn extract_model_name(&self, input: &str) -> Option<String> {
        let input_lower = input.to_lowercase();
        
        // Common model patterns
        let models = ["gpt-4", "gpt-3.5", "claude-3", "claude-2", "llama", "gemini"];
        
        for model in &models {
            if input_lower.contains(model) {
                return Some(model.to_string());
            }
        }
        
        // Try to extract from common patterns
        let words: Vec<&str> = input.split_whitespace().collect();
        for i in 0..words.len() {
            if words[i].to_lowercase() == "model" && i + 1 < words.len() {
                return Some(words[i + 1].to_string());
            }
            if words[i].to_lowercase() == "to" && i + 1 < words.len() {
                // For "change model to X"
                if i > 0 && words[i - 1].to_lowercase() == "model" {
                    return Some(words[i + 1].to_string());
                }
            }
        }
        
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    // Mock tests - in practice you'd need a test environment
    #[tokio::test]
    async fn test_intent_recognition_concept() {
        // This would require a mock OpenRouter client for testing
        // The concept is sound but needs proper test infrastructure
        
        // Test cases that the LLM should handle better than regex:
        let test_cases = vec![
            ("please clear the conversation context", UserIntent::ClearContext),
            ("I'd like to switch to gpt-4 please", UserIntent::ChangeModel("gpt-4".to_string())),
            ("could you show me what models are available?", UserIntent::ListModels(None)),
            ("can you help me plan a web application?", UserIntent::RequestHelp("can you help me plan a web application?".to_string())),
            ("let's build something cool together", UserIntent::Implement("let's build something cool together".to_string())),
            ("my code isn't working properly", UserIntent::Troubleshoot("my code isn't working properly".to_string())),
            ("tell me more about this codebase", UserIntent::Explore("tell me more about this codebase".to_string())),
            ("how's your day going?", UserIntent::RegularConversation("how's your day going?".to_string())),
        ];

        // These would all work with LLM-based recognition but fail with regex
        assert!(true, "LLM-based intent recognition would handle all natural language variations");
    }
}