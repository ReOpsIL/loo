use std::fs;
use serde::{Deserialize, Serialize};
use crate::plan_display::{ActionPlan, Phase, Action, ActionStatus};
use crate::engine::LooEngine;

#[derive(Debug, Deserialize, Serialize)]
struct JsonActionPlan {
    title: String,
    overview: String,
    phases: Vec<JsonPhase>,
    expected_outcome: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonPhase {
    name: String,
    emoji: String,
    actions: Vec<JsonAction>,
}

#[derive(Debug, Deserialize, Serialize)]
struct JsonAction {
    id: usize,
    title: String,
    tool: String,
    target: String,
    operation: String,
    purpose: String,
    success_criteria: String,
    dependencies: Vec<usize>,
    status: String,
}

pub struct PlanCommand {
    prompt_path: String,
}

impl PlanCommand {
    pub fn new() -> Self {
        Self {
            prompt_path: "prompts/action_plan_prompt.md".to_string(),
        }
    }

    pub fn load_prompt(&self) -> Result<String, Box<dyn std::error::Error>> {
        let prompt_content = fs::read_to_string(&self.prompt_path)?;
        Ok(prompt_content)
    }

    pub fn create_full_prompt(&self, user_request: &str) -> Result<String, Box<dyn std::error::Error>> {
        let base_prompt = self.load_prompt()?;
        
        let full_prompt = format!(
            "{}\n\n---\n\n## User Request\n\n{}\n\nPlease create a detailed action plan for this request following the format specified above. Remember to respond with valid JSON only.",
            base_prompt,
            user_request
        );

        Ok(full_prompt)
    }

    pub fn parse_plan_json(&self, json_response: &str) -> Result<ActionPlan, Box<dyn std::error::Error>> {
        // Clean the JSON response - remove any markdown code blocks or extra text
        let cleaned_json = self.extract_json(json_response)?;
        
        let json_plan: JsonActionPlan = serde_json::from_str(&cleaned_json)?;
        
        // Convert JSON structure to display structure
        let mut phases = Vec::new();
        
        for json_phase in json_plan.phases {
            let mut actions = Vec::new();
            
            for json_action in json_phase.actions {
                let status = match json_action.status.to_lowercase().as_str() {
                    "pending" => ActionStatus::Pending,
                    "in_progress" | "in-progress" => ActionStatus::InProgress,
                    "completed" => ActionStatus::Completed,
                    "failed" => ActionStatus::Failed,
                    _ => ActionStatus::Pending,
                };
                
                actions.push(Action {
                    id: json_action.id,
                    title: json_action.title,
                    tool: json_action.tool,
                    target: json_action.target,
                    operation: json_action.operation,
                    purpose: json_action.purpose,
                    success_criteria: json_action.success_criteria,
                    dependencies: json_action.dependencies,
                    status,
                });
            }
            
            phases.push(Phase {
                name: json_phase.name,
                emoji: json_phase.emoji,
                actions,
            });
        }
        
        Ok(ActionPlan {
            title: json_plan.title,
            overview: json_plan.overview,
            phases,
            expected_outcome: json_plan.expected_outcome,
        })
    }

    fn extract_json(&self, response: &str) -> Result<String, Box<dyn std::error::Error>> {
        let response = response.trim();
        
        // If response starts with {, assume it's clean JSON
        if response.starts_with('{') && response.ends_with('}') {
            return Ok(response.to_string());
        }
        
        // Look for JSON within markdown code blocks
        if let Some(start) = response.find("```json") {
            let after_start = &response[start + 7..]; // Skip "```json"
            if let Some(end) = after_start.find("```") {
                return Ok(after_start[..end].trim().to_string());
            }
        }
        
        // Look for JSON within generic code blocks
        if let Some(start) = response.find("```") {
            let after_start = &response[start + 3..];
            if let Some(end) = after_start.find("```") {
                let potential_json = after_start[..end].trim();
                if potential_json.starts_with('{') && potential_json.ends_with('}') {
                    return Ok(potential_json.to_string());
                }
            }
        }
        
        // Look for first { to last } in the entire response
        if let (Some(start), Some(end)) = (response.find('{'), response.rfind('}')) {
            if start < end {
                return Ok(response[start..=end].to_string());
            }
        }
        
        Err("Could not extract valid JSON from response".into())
    }

    pub async fn execute(&self, user_request: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Create engine instance for LLM processing
        let working_dir = std::env::current_dir()?.to_string_lossy().to_string();
        let mut engine = LooEngine::new(working_dir, None, false).await?;
        
        // Create the full prompt for plan generation
        let full_prompt = self.create_full_prompt(user_request)?;
        
        // Process the prompt through the engine to get LLM response
        let llm_response = self.process_plan_request(&mut engine, &full_prompt).await?;
        
        // Parse the JSON response and format the plan
        match self.parse_plan_json(&llm_response) {
            Ok(plan) => {
                // Return formatted plan display
                Ok(format!("🎯 Generated Action Plan:\n\n{}", plan))
            }
            Err(parse_error) => {
                // If JSON parsing fails, return the raw response with error info
                Ok(format!(
                    "⚠️  Plan generated but JSON parsing failed:\n{}\n\n\
                    Raw LLM Response:\n{}\n\n\
                    💡 The LLM may have included extra text. Try using /parse-plan with clean JSON.",
                    parse_error, llm_response
                ))
            }
        }
    }


    async fn process_plan_request(&self, engine: &mut LooEngine, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        use crate::openrouter::Message;
        
        // Create system message for plan generation
        let system_message = Message {
            role: "system".to_string(),
            content: "You are an expert code planning assistant. You create detailed, step-by-step action plans for coding tasks. Always respond with valid JSON only, following the exact format specified in the user's prompt.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        
        // Create user message with the plan request
        let user_message = Message {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };
        
        // Set up messages for this plan request
        engine.messages.clear();
        engine.messages.push(system_message);
        engine.messages.push(user_message);
        
        // Process through engine to get LLM response
        let response = engine.openrouter_client
            .chat_completion(engine.messages.clone())
            .await?;
        
        let assistant_message = &response.choices[0].message;
        
        // Return the content from the LLM response
        Ok(assistant_message.content.clone())
    }

    pub fn display_plan(&self, json_response: &str) -> Result<String, Box<dyn std::error::Error>> {
        let plan = self.parse_plan_json(json_response)?;
        Ok(format!("{}", plan))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_clean() {
        let cmd = PlanCommand::new();
        let response = r#"{"title": "test", "overview": "test overview"}"#;
        let result = cmd.extract_json(response).unwrap();
        assert_eq!(result, response);
    }

    #[test]
    fn test_extract_json_with_markdown() {
        let cmd = PlanCommand::new();
        let response = r#"Here's the plan:

```json
{"title": "test", "overview": "test overview"}
```

That's the plan!"#;
        let result = cmd.extract_json(response).unwrap();
        assert_eq!(result, r#"{"title": "test", "overview": "test overview"}"#);
    }

    #[test]
    fn test_extract_json_with_extra_text() {
        let cmd = PlanCommand::new();
        let response = r#"Sure, here's your plan: {"title": "test", "overview": "test overview"} Hope this helps!"#;
        let result = cmd.extract_json(response).unwrap();
        assert_eq!(result, r#"{"title": "test", "overview": "test overview"}"#);
    }
}