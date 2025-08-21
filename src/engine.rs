use crate::config::{Config, ConfigManager};
use crate::openrouter::{Message, OpenRouterClient};
use crate::story::StoryLogger;
use crate::terminal::{TerminalInput, InputEvent};
use crate::tools::ToolExecutor;
use crate::commands::{execute_command, engine_commands};
use crate::execution_stack::{ExecutionStack, StackRequest, StackResponse};
use crate::llm_schemas::{TaskDecompositionResponse, PlanActionDecompositionResponse, NestedPlanResponse, schema_examples, create_json_prompt};
use serde_json::json;
use uuid::Uuid;

pub struct LooEngine {
    pub openrouter_client: OpenRouterClient,
    pub tool_executor: ToolExecutor,
    pub story_logger: StoryLogger,
    pub config: Config,
    pub working_dir: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub execution_stack: ExecutionStack,
    pub auto_execute_stack: bool,
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
            execution_stack: ExecutionStack::new(),
            auto_execute_stack: true,
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
                InputEvent::CommandExecuted(result) => {
                    // Display command result and continue
                    println!("✅ Command result:\n{}\n", result);
                    continue;
                }
                InputEvent::EngineCommand(command_line) => {
                    // Handle engine-specific commands
                    let parts: Vec<&str> = command_line.split_whitespace().collect();
                    if parts.is_empty() {
                        continue;
                    }
                    
                    let command_name = parts[0];
                    
                    // Execute command through registry
                    let result = if let Some(registry_result) = execute_command(&command_line) {
                        // Check if this is an engine command marker
                        match registry_result {
                            Err(e) if e.to_string().starts_with("ENGINE_COMMAND:") => {
                                let error_msg = e.to_string();
                                let parts: Vec<&str> = error_msg.strip_prefix("ENGINE_COMMAND:").unwrap().split(':').collect();
                                match parts[0] {
                                    "clear" => engine_commands::handle_clear_command(self).await,
                                    "model" => {
                                        let model_name = if parts.len() > 1 { parts[1] } else { "" };
                                        engine_commands::handle_model_command(self, model_name).await
                                    },
                                    "list-models" => {
                                        let search_term = if parts.len() > 1 { parts[1] } else { "" };
                                        engine_commands::handle_list_models_command(self, search_term).await
                                    },
                                    "plan" => {
                                        let request = if parts.len() > 1 { parts[1] } else { "" };
                                        engine_commands::handle_plan_command(self, request).await
                                    },
                                    "stack-status" => {
                                        engine_commands::handle_stack_status_command(self, "").await
                                    },
                                    "stack-execute" => {
                                        engine_commands::handle_stack_execute_command(self, "").await
                                    },
                                    "stack-clear" => {
                                        engine_commands::handle_stack_clear_command(self, "").await
                                    },
                                    "stack-auto" => {
                                        let args = if parts.len() > 1 { parts[1] } else { "" };
                                        engine_commands::handle_stack_auto_command(self, args).await
                                    },
                                    "stack-push" => {
                                        let args = if parts.len() > 1 { parts[1] } else { "" };
                                        engine_commands::handle_stack_push_command(self, args).await
                                    },
                                    _ => Err(format!("Unknown engine command: {}", parts[0]).into())
                                }
                            },
                            other => other
                        }
                    } else {
                        Err(format!("Unknown command: {}", command_name).into())
                    };


                    match result {
                        Ok(output) => println!("✅ {}\n", output),
                        Err(e) => println!("❌ Error: {}\n", e),
                    }
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

    /// Push a user prompt to the execution stack
    pub fn push_user_prompt(&mut self, prompt: &str, priority: u8) -> String {
        self.execution_stack.push_user_prompt(prompt.to_string(), priority)
    }

    /// Push an action plan to the execution stack
    pub fn push_action_plan(&mut self, plan: crate::plan_display::ActionPlan) -> Vec<String> {
        self.execution_stack.push_action_plan(plan, None)
    }

    /// Start the recursive execution loop
    pub async fn start_stack_execution(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.auto_execute_stack {
            return Ok(());
        }

        println!("🔄 Starting recursive execution stack processing...");
        println!("{}", self.execution_stack.get_status_summary());

        while self.execution_stack.has_pending_requests() {
            if let Some(request) = self.execution_stack.pop_request() {
                println!("\n🎯 Processing request: {}", self.get_request_description(&request));
                
                let request_id = self.get_request_id_from_request(&request);
                
                // Mark request as started
                self.execution_stack.start_processing(request.clone());
                
                // Process the request
                match self.process_stack_request(request).await {
                    Ok(response) => {
                        println!("✅ Request completed successfully");
                        self.execution_stack.push_response(response);
                    }
                    Err(e) => {
                        println!("❌ Request failed: {}", e);
                        // Create error response
                        let error_response = StackResponse {
                            request_id,
                            success: false,
                            content: format!("Error: {}", e),
                            generated_requests: Vec::new(),
                            completed_actions: Vec::new(),
                        };
                        self.execution_stack.push_response(error_response);
                    }
                }

                // Small delay to prevent overwhelming the LLM
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        }

        println!("\n🎉 Stack execution completed!");
        println!("{}", self.execution_stack.get_status_summary());
        Ok(())
    }

    /// Process a single stack request
    async fn process_stack_request(&mut self, request: StackRequest) -> Result<StackResponse, Box<dyn std::error::Error>> {
        match request {
            StackRequest::UserPrompt { id, content, .. } => {
                self.process_user_prompt_request(id, content).await
            }
            StackRequest::PlanAction { id, action, context, .. } => {
                self.process_plan_action_request(id, action, context).await
            }
            StackRequest::NestedPlan { id, request: req_content, depth, .. } => {
                self.process_nested_plan_request(id, req_content, depth).await
            }
        }
    }

    /// Process a user prompt request using structured JSON
    async fn process_user_prompt_request(&mut self, id: String, content: String) -> Result<StackResponse, Box<dyn std::error::Error>> {
        // Check if this is a simple request that can be executed directly
        if self.is_executable_request(&content) {
            return self.execute_direct_request(id, content).await;
        }

        // Create JSON-structured prompt for LLM decomposition
        let instruction = format!(
            "Analyze this user request and determine if it can be executed directly or needs to be broken down into sub-tasks.\n\n\
            User Request: {}\n\n\
            If the request is simple and executable (like 'create file X', 'run command Y'), set is_executable to true and provide the executable_action.\n\
            If the request is complex and needs decomposition, set is_executable to false and provide sub_tasks.\n\n\
            Consider the available tools: bash, file operations, git, directory operations, etc.",
            content
        );

        let json_prompt = create_json_prompt(&instruction, schema_examples::TASK_DECOMPOSITION);
        let llm_response = self.send_decomposition_request(&json_prompt).await?;
        
        // Parse JSON response
        match self.parse_task_decomposition_response(&llm_response) {
            Ok(decomposition) => {
                if decomposition.is_executable {
                    // Execute directly if marked as executable
                    if let Some(action) = decomposition.executable_action {
                        let action_description = format!("{} {} on {}", action.tool, action.operation, action.target);
                        return self.execute_direct_request(id, action_description).await;
                    } else {
                        return Err("LLM marked as executable but provided no executable_action".into());
                    }
                } else {
                    // Create sub-requests from the decomposition
                    let sub_requests = self.create_sub_requests_from_decomposition(&decomposition, &id, 1)?;
                    
                    Ok(StackResponse {
                        request_id: id,
                        success: true,
                        content: format!("Analysis: {}\nReasoning: {}", decomposition.analysis, decomposition.reasoning),
                        generated_requests: sub_requests,
                        completed_actions: Vec::new(),
                    })
                }
            }
            Err(parse_error) => {
                // Fallback to original string parsing if JSON parsing fails
                println!("⚠️ JSON parsing failed, falling back to string parsing: {}", parse_error);
                self.process_user_prompt_fallback(id, content, llm_response).await
            }
        }
    }

    /// Process a plan action request using structured JSON
    async fn process_plan_action_request(&mut self, id: String, action: crate::plan_display::Action, context: String) -> Result<StackResponse, Box<dyn std::error::Error>> {
        // Check if this action is already executable
        if self.is_action_executable(&action) {
            return self.execute_plan_action(id, action).await;
        }

        // Create JSON-structured prompt for plan action decomposition
        let instruction = format!(
            "Analyze this plan action and determine if it can be executed directly or needs to be broken down into executable steps.\n\n\
            Context: {}\n\
            Action: {}\n\
            Tool: {}\n\
            Target: {}\n\
            Operation: {}\n\
            Purpose: {}\n\n\
            If the action can be executed directly with the specified tool, set is_executable to true and provide executable_steps.\n\
            If the action needs further breakdown, set is_executable to false and provide sub_actions.\n\n\
            Available tools include: bash, file operations, git, directory operations, etc.",
            context, action.title, action.tool, action.target, action.operation, action.purpose
        );

        let json_prompt = create_json_prompt(&instruction, schema_examples::PLAN_ACTION_DECOMPOSITION);
        let llm_response = self.send_decomposition_request(&json_prompt).await?;
        
        // Parse JSON response
        match self.parse_plan_action_decomposition_response(&llm_response) {
            Ok(decomposition) => {
                if decomposition.is_executable {
                    // Execute the action directly
                    return self.execute_plan_action(id, action).await;
                } else {
                    // Create sub-requests from the decomposition
                    let sub_requests = self.create_sub_requests_from_plan_action_decomposition(&decomposition, &id, 2)?;
                    
                    Ok(StackResponse {
                        request_id: id,
                        success: true,
                        content: format!("Analysis: {}", decomposition.analysis),
                        generated_requests: sub_requests,
                        completed_actions: Vec::new(),
                    })
                }
            }
            Err(parse_error) => {
                // Fallback to original string parsing if JSON parsing fails
                println!("⚠️ JSON parsing failed for plan action, falling back to string parsing: {}", parse_error);
                self.process_plan_action_fallback(id, action, llm_response).await
            }
        }
    }

    /// Process a nested plan request
    async fn process_nested_plan_request(&mut self, id: String, request: String, depth: u8) -> Result<StackResponse, Box<dyn std::error::Error>> {
        // Use the existing plan command functionality but at a nested level
        use crate::commands::PlanCommand;
        let plan_cmd = PlanCommand::new();
        
        match plan_cmd.execute(&request).await {
            Ok(plan_result) => {
                // Try to parse the plan result to extract actionable items
                let sub_requests = self.extract_requests_from_plan_output(&plan_result, &id, depth + 1)?;
                
                Ok(StackResponse {
                    request_id: id,
                    success: true,
                    content: plan_result,
                    generated_requests: sub_requests,
                    completed_actions: Vec::new(),
                })
            }
            Err(e) => Err(e.into())
        }
    }

    /// Check if a request is executable without further decomposition
    fn is_executable_request(&self, request: &str) -> bool {
        let request_lower = request.to_lowercase();
        
        // Simple heuristics for executable requests
        request_lower.starts_with("create file") ||
        request_lower.starts_with("write to file") ||
        request_lower.starts_with("run command") ||
        request_lower.starts_with("execute") ||
        request_lower.starts_with("delete") ||
        request_lower.starts_with("copy") ||
        request_lower.starts_with("move") ||
        request_lower.contains("ls ") ||
        request_lower.contains("mkdir ") ||
        request_lower.contains("touch ") ||
        request_lower.contains("echo ")
    }

    /// Check if a plan action is executable
    fn is_action_executable(&self, action: &crate::plan_display::Action) -> bool {
        // Actions with specific tools and clear targets are likely executable
        matches!(action.tool.to_lowercase().as_str(), 
            "bash" | "run_command" | "create_file" | "write_file" | 
            "read_file" | "ls" | "mkdir" | "touch" | "echo" | "git")
    }

    /// Execute a direct request using tools
    async fn execute_direct_request(&mut self, id: String, request: String) -> Result<StackResponse, Box<dyn std::error::Error>> {
        println!("⚙️ Executing direct request: {}", request);
        
        // This would integrate with the existing tool system
        // For now, simulate execution
        let execution_result = format!("Executed: {}", request);
        
        Ok(StackResponse {
            request_id: id.clone(),
            success: true,
            content: execution_result,
            generated_requests: Vec::new(),
            completed_actions: vec![id],
        })
    }

    /// Execute a plan action using tools
    async fn execute_plan_action(&mut self, id: String, action: crate::plan_display::Action) -> Result<StackResponse, Box<dyn std::error::Error>> {
        println!("⚙️ Executing plan action: {}", action.title);
        
        // This would integrate with the existing tool executor
        let execution_result = format!("Executed action: {} using {}", action.title, action.tool);
        
        Ok(StackResponse {
            request_id: id.clone(),
            success: true,
            content: execution_result,
            generated_requests: Vec::new(),
            completed_actions: vec![action.id.to_string()],
        })
    }

    /// Send a decomposition request to the LLM
    async fn send_decomposition_request(&mut self, prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Create a temporary message set for decomposition
        let system_message = Message {
            role: "system".to_string(),
            content: "You are a task decomposition expert. Break down complex requests into specific, actionable steps. Be concise and precise.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let user_message = Message {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let temp_messages = vec![system_message, user_message];
        
        let response = self.openrouter_client.chat_completion(temp_messages).await?;
        Ok(response.choices[0].message.content.clone())
    }

    /// Parse LLM decomposition response into stack requests
    fn parse_decomposition_into_requests(&mut self, response: &str, parent_id: &str, depth: u8) -> Result<Vec<StackRequest>, Box<dyn std::error::Error>> {
        let mut requests = Vec::new();
        
        // Simple parsing: look for numbered steps
        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() { continue; }
            
            // Look for patterns like "1. ", "2. ", "- ", etc.
            if line.chars().next().map(|c| c.is_numeric()).unwrap_or(false) || line.starts_with("- ") {
                let step_content = line
                    .split_once('.').map(|(_, content)| content.trim())
                    .or_else(|| line.strip_prefix("- "))
                    .unwrap_or(line);
                
                if !step_content.is_empty() {
                    let request_id = self.execution_stack.push_nested_plan(
                        parent_id.to_string(),
                        step_content.to_string(),
                        depth
                    );
                    
                    if !request_id.is_empty() {
                        requests.push(StackRequest::NestedPlan {
                            id: request_id,
                            parent_id: parent_id.to_string(),
                            request: step_content.to_string(),
                            depth,
                        });
                    }
                }
            }
        }
        
        Ok(requests)
    }

    /// Extract requests from plan command output
    fn extract_requests_from_plan_output(&mut self, plan_output: &str, parent_id: &str, depth: u8) -> Result<Vec<StackRequest>, Box<dyn std::error::Error>> {
        // This is a simplified version - in practice, you'd parse the structured plan output
        self.parse_decomposition_into_requests(plan_output, parent_id, depth)
    }

    /// Get description of a request for logging
    fn get_request_description(&self, request: &StackRequest) -> String {
        match request {
            StackRequest::UserPrompt { content, .. } => format!("User Prompt: {}", content),
            StackRequest::PlanAction { action, .. } => format!("Plan Action: {}", action.title),
            StackRequest::NestedPlan { request, depth, .. } => format!("Nested Plan (depth {}): {}", depth, request),
        }
    }

    /// Helper to get request ID from any request
    fn get_request_id_from_request(&self, request: &StackRequest) -> String {
        match request {
            StackRequest::UserPrompt { id, .. } => id.clone(),
            StackRequest::PlanAction { id, .. } => id.clone(),
            StackRequest::NestedPlan { id, .. } => id.clone(),
        }
    }

    /// Toggle automatic stack execution
    pub fn set_auto_execute(&mut self, enabled: bool) {
        self.auto_execute_stack = enabled;
        println!("🔄 Auto-execute stack: {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Get stack status
    pub fn get_stack_status(&self) -> String {
        self.execution_stack.get_status_summary()
    }

    /// Clear the execution stack
    pub fn clear_stack(&mut self) {
        self.execution_stack.clear_all();
    }

    /// Parse task decomposition response from JSON
    fn parse_task_decomposition_response(&self, json_response: &str) -> Result<TaskDecompositionResponse, Box<dyn std::error::Error>> {
        // Clean the JSON response - remove any markdown code blocks or extra text
        let cleaned_json = self.extract_clean_json(json_response)?;
        let decomposition: TaskDecompositionResponse = serde_json::from_str(&cleaned_json)?;
        Ok(decomposition)
    }

    /// Parse plan action decomposition response from JSON
    fn parse_plan_action_decomposition_response(&self, json_response: &str) -> Result<PlanActionDecompositionResponse, Box<dyn std::error::Error>> {
        let cleaned_json = self.extract_clean_json(json_response)?;
        let decomposition: PlanActionDecompositionResponse = serde_json::from_str(&cleaned_json)?;
        Ok(decomposition)
    }

    /// Parse nested plan response from JSON
    fn parse_nested_plan_response(&self, json_response: &str) -> Result<NestedPlanResponse, Box<dyn std::error::Error>> {
        let cleaned_json = self.extract_clean_json(json_response)?;
        let plan_response: NestedPlanResponse = serde_json::from_str(&cleaned_json)?;
        Ok(plan_response)
    }

    /// Extract clean JSON from LLM response (handles markdown, extra text, etc.)
    fn extract_clean_json(&self, response: &str) -> Result<String, Box<dyn std::error::Error>> {
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
        
        Err("Could not extract valid JSON from LLM response".into())
    }

    /// Create sub-requests from task decomposition
    fn create_sub_requests_from_decomposition(&mut self, decomposition: &TaskDecompositionResponse, parent_id: &str, depth: u8) -> Result<Vec<StackRequest>, Box<dyn std::error::Error>> {
        let mut requests = Vec::new();
        
        if let Some(sub_tasks) = &decomposition.sub_tasks {
            for sub_task in sub_tasks {
                let request_id = self.execution_stack.push_nested_plan(
                    parent_id.to_string(),
                    sub_task.description.clone(),
                    depth
                );
                
                if !request_id.is_empty() {
                    requests.push(StackRequest::NestedPlan {
                        id: request_id,
                        parent_id: parent_id.to_string(),
                        request: sub_task.description.clone(),
                        depth,
                    });
                }
            }
        }
        
        Ok(requests)
    }

    /// Create sub-requests from plan action decomposition
    fn create_sub_requests_from_plan_action_decomposition(&mut self, decomposition: &PlanActionDecompositionResponse, parent_id: &str, depth: u8) -> Result<Vec<StackRequest>, Box<dyn std::error::Error>> {
        let mut requests = Vec::new();
        
        if let Some(sub_actions) = &decomposition.sub_actions {
            for sub_action in sub_actions {
                let request_id = self.execution_stack.push_nested_plan(
                    parent_id.to_string(),
                    sub_action.description.clone(),
                    depth
                );
                
                if !request_id.is_empty() {
                    requests.push(StackRequest::NestedPlan {
                        id: request_id,
                        parent_id: parent_id.to_string(),
                        request: sub_action.description.clone(),
                        depth,
                    });
                }
            }
        }
        
        Ok(requests)
    }

    /// Fallback to string parsing when JSON parsing fails
    async fn process_user_prompt_fallback(&mut self, id: String, content: String, llm_response: String) -> Result<StackResponse, Box<dyn std::error::Error>> {
        println!("🔄 Using fallback string parsing for user prompt");
        
        if llm_response.starts_with("EXECUTABLE:") {
            let action = llm_response.strip_prefix("EXECUTABLE:").unwrap().trim();
            return self.execute_direct_request(id, action.to_string()).await;
        }

        // Parse the decomposition and create sub-requests using old method
        let sub_requests = self.parse_decomposition_into_requests(&llm_response, &id, 1)?;
        
        Ok(StackResponse {
            request_id: id,
            success: true,
            content: llm_response,
            generated_requests: sub_requests,
            completed_actions: Vec::new(),
        })
    }

    /// Fallback plan action processing with string parsing
    async fn process_plan_action_fallback(&mut self, id: String, action: crate::plan_display::Action, llm_response: String) -> Result<StackResponse, Box<dyn std::error::Error>> {
        println!("🔄 Using fallback string parsing for plan action");
        
        if llm_response.starts_with("EXECUTABLE:") {
            return self.execute_plan_action(id, action).await;
        }

        // Parse decomposition into sub-requests using old method
        let sub_requests = self.parse_decomposition_into_requests(&llm_response, &id, 2)?;
        
        Ok(StackResponse {
            request_id: id,
            success: true,
            content: llm_response,
            generated_requests: sub_requests,
            completed_actions: Vec::new(),
        })
    }
}