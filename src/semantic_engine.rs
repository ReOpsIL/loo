use crate::config::Config;
use crate::openrouter::{Message, OpenRouterClient};
use crate::prompts::PromptManager;
use crate::story::StoryLogger;
use crate::tools::ToolExecutor;
use inquire::Autocomplete;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;

/// Represents conversation context and semantic understanding
#[derive(Debug, Clone)]
pub struct ConversationContext {
    /// Recent messages with semantic importance
    pub important_messages: VecDeque<Message>,
    /// Current conversation thread (what the user is working on)
    pub current_thread: Option<String>,
    /// Tools that are contextually relevant
    pub available_tools: Vec<String>,
    /// Conversation state (planning, executing, questioning, etc.)
    pub state: ConversationState,
    /// Working memory for ongoing tasks
    pub working_memory: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversationState {
    /// User is asking questions or having a discussion
    Conversational,
    /// User needs help planning something
    Planning,
    /// User is actively working on implementation
    Implementing,
    /// User is stuck and needs guidance
    Troubleshooting,
    /// User is exploring/investigating something
    Exploring,
}

/// JSON response schema for LLM conversation state analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConversationStateAnalysis {
    /// Detected conversation state
    state: String,
    /// Confidence level (0.0 to 1.0)
    confidence: f32,
    /// Brief reasoning for the classification
    reasoning: String,
    /// Extracted main topic/thread from the input
    topic: Option<String>,
    /// Suggested tools that might be relevant
    suggested_tools: Vec<String>,
}

impl Default for ConversationContext {
    fn default() -> Self {
        Self {
            important_messages: VecDeque::with_capacity(20),
            current_thread: None,
            available_tools: vec![
                "create_file".to_string(),
                "run_command".to_string(),
                "read_file".to_string(),
                "create_directory".to_string(),
            ],
            state: ConversationState::Conversational,
            working_memory: Vec::new(),
        }
    }
}

/// Semantic conversation engine that adapts to user needs
pub struct SemanticEngine {
    pub openrouter_client: OpenRouterClient,
    pub tool_executor: ToolExecutor,
    pub story_logger: StoryLogger,
    pub config: Config,
    pub working_dir: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub context: ConversationContext,
}

impl SemanticEngine {
    pub async fn new(
        working_dir: String,
        cli_model: Option<String>,
        cli_verbose: bool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        use crate::config::ConfigManager;
        use uuid::Uuid;

        let mut config = ConfigManager::load_config()?;

        if let Some(model) = cli_model {
            config.openrouter.model = model;
        }

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
            context: ConversationContext::default(),
        })
    }

    /// Process a conversation turn with semantic understanding
    pub async fn process_conversation(&mut self, user_input: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Analyze user intent and update conversation context
        self.analyze_and_update_context(user_input).await?;

        // Add user message to conversation
        let user_message = Message {
            role: "user".to_string(),
            content: user_input.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        self.messages.push(user_message.clone());
        self.context.important_messages.push_back(user_message);
        self.story_logger.log_user_prompt(user_input);

        // Manage context size
        self.manage_context_size();

        // Process with adaptive system message
        let system_message = self.create_adaptive_system_message();

        // Create temporary message list with adaptive context
        let mut conversation_messages = vec![system_message];
        conversation_messages.extend(self.get_relevant_context());
        conversation_messages.push(self.messages.last().unwrap().clone());

        // Process conversation loop with semantic awareness
        loop {
            let response = self.openrouter_client
                .chat_completion(conversation_messages.clone())
                .await?;

            let assistant_message = &response.choices[0].message;
            let response_clone = assistant_message.clone();
            conversation_messages.push(response_clone.clone());
            self.messages.push(response_clone);

            // Update working memory with assistant insights
            if !assistant_message.content.is_empty() {
                self.story_logger.log_assistant_response(&assistant_message.content);
                self.update_working_memory(&assistant_message.content);
            }

            // Handle tool calls with semantic awareness
            if let Some(tool_calls) = &assistant_message.tool_calls {
                self.execute_tools_semantically(tool_calls, &mut conversation_messages).await?;
            } else {
                // No more tool calls, conversation complete
                if !assistant_message.content.is_empty() {
                    println!("🤖 {}", assistant_message.content);
                }
                break;
            }
        }

        Ok(())
    }

    /// Analyze user input and update conversation context using LLM
    async fn analyze_and_update_context(&mut self, user_input: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Use LLM to analyze conversation state
        match self.analyze_conversation_state_with_llm(user_input).await {
            Ok(analysis) => {
                // Parse the detected state
                self.context.state = self.parse_conversation_state(&analysis.state);

                // Update current thread/topic if provided
                if let Some(topic) = analysis.topic {
                    if !topic.trim().is_empty() {
                        self.context.current_thread = Some(topic);
                    }
                }

                // Update available tools based on LLM suggestions
                if !analysis.suggested_tools.is_empty() {
                    self.context.available_tools.extend(analysis.suggested_tools);
                    self.context.available_tools.sort();
                    self.context.available_tools.dedup();
                }

                // Add analysis reasoning to working memory for context
                if !analysis.reasoning.trim().is_empty() {
                    self.context.working_memory.push(format!("Analysis: {}", analysis.reasoning));
                }
            }
            Err(e) => {
                // Fallback to rule-based analysis if LLM fails
                eprintln!("Error: LLM analysis failed, using Conversational state: {}", e);
                self.context.state = ConversationState::Conversational;
            }
        }

        Ok(())
    }

    /// Update available tools based on conversation semantics
    fn update_available_tools(&mut self, input_lower: &str) {
        let mut tools = vec!["create_file", "run_command", "read_file", "create_directory"];

        // Add context-specific tools
        if input_lower.contains("git") || input_lower.contains("commit") || input_lower.contains("repository") {
            tools.extend_from_slice(&["git_status", "git_add", "git_commit"]);
        }

        if input_lower.contains("test") || input_lower.contains("spec") {
            tools.extend_from_slice(&["run_tests", "create_test"]);
        }

        if input_lower.contains("install") || input_lower.contains("package") || input_lower.contains("dependency") {
            tools.extend_from_slice(&["package_install", "dependency_check"]);
        }

        if input_lower.contains("database") || input_lower.contains("db") || input_lower.contains("sql") {
            tools.extend_from_slice(&["database_query", "database_migration"]);
        }

        self.context.available_tools = tools.into_iter().map(|s| s.to_string()).collect();
    }

    /// Extract the main topic/thread from user input
    fn extract_topic(&self, input: &str) -> String {
        // Simple topic extraction - could be enhanced with NLP
        let words: Vec<&str> = input.split_whitespace().collect();
        let mut topic_words = Vec::new();

        for window in words.windows(3) {
            if window[1].to_lowercase() == "for" || window[1].to_lowercase() == "with" {
                topic_words.extend_from_slice(window);
                break;
            }
        }

        if topic_words.is_empty() && words.len() > 2 {
            topic_words = words[0..std::cmp::min(5, words.len())].to_vec();
        }

        topic_words.join(" ")
    }

    /// Create an adaptive system message based on current context
    fn create_adaptive_system_message(&self) -> Message {
        // Start with the base system prompt from PromptManager
        let mut content = PromptManager::get_system_prompt();

        // Add working directory context
        content.push_str(&format!(" Working directory: {}.", self.working_dir));

        // Add conversation state-specific extensions
        match self.context.state {
            ConversationState::Planning => {
                content.push_str(" PLANNING MODE: You are helping the user plan their approach.
                    Create detailed, step-by-step action plans that break down complex requests into specific, executable actions.
                    Each action should specify exactly which tools to use and what operations to perform.
                    Ask clarifying questions if the request is ambiguous. Focus on creating comprehensive plans before implementation.
                    Use the action plan format from the prompts when creating detailed plans.");
            }
            ConversationState::Implementing => {
                content.push_str(" IMPLEMENTATION MODE: The user is ready to build and implement.
                    Use available tools proactively to execute the necessary steps.
                    Follow systematic approaches: Read files before editing, verify changes after implementation,
                    and use appropriate tools for each operation (Edit, MultiEdit, Write, Bash, etc.).
                    Be thorough in your execution and provide clear feedback about what you're doing.");
            }
            ConversationState::Troubleshooting => {
                content.push_str(" TROUBLESHOOTING MODE: The user is experiencing difficulties.
                    Help them debug issues systematically. First, gather information about the problem,
                    examine relevant files and logs, reproduce the issue if possible,
                    then provide specific solutions. Use discovery tools (Read, Grep, Glob, LS, Bash)
                    to investigate before suggesting fixes.");
            }
            ConversationState::Exploring => {
                content.push_str(" EXPLORATION MODE: The user wants to investigate and understand existing code/systems.
                    Help them explore systematically using discovery tools.
                    Examine code structure, understand implementations, explain findings clearly,
                    and guide them through the codebase. Focus on understanding before making changes.");
            }
            ConversationState::Conversational => {
                content.push_str(" CONVERSATIONAL MODE: Engage in natural conversation while being helpful and responsive.
                    Provide clear explanations, answer questions thoroughly, and be ready to switch modes
                    based on the user's needs. Use tools when necessary to provide accurate information.");
            }
        }

        // Add available tools context
        if !self.context.available_tools.is_empty() {
            content.push_str(&format!(" Available tools: {}.", self.context.available_tools.join(", ")));
        }

        // Add current thread/focus context
        if let Some(thread) = &self.context.current_thread {
            content.push_str(&format!(" Current focus: {}.", thread));
        }

        // Add working memory context
        if !self.context.working_memory.is_empty() {
            let recent_context = self.context.working_memory
                .iter()
                .rev()
                .take(3)
                .rev()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            content.push_str(&format!(" Recent context: {}.", recent_context));
        }

        // Add tool selection guidance based on conversation state
        match self.context.state {
            ConversationState::Planning => {
                content.push_str(" For planning: Focus on understanding requirements first, then create structured plans.
                     Use Read/Grep/LS tools for discovery, then provide detailed action plans.");
            }
            ConversationState::Implementing => {
                content.push_str(" For implementation: Use Edit/MultiEdit for code changes, Write for new files,
                    Bash for commands/tests. Always read files before editing.");
            }
            ConversationState::Troubleshooting => {
                content.push_str(" For troubleshooting: Start with Read/Grep to examine code, Bash to reproduce issues,
                then use Edit tools to fix problems. Verify fixes with additional commands.");
            }
            ConversationState::Exploring => {
                content.push_str(" For exploration: Use Read, Grep, Glob, and LS extensively to understand the codebase.
                Explain what you find and guide the user through the structure.");
            }
            _ => {}
        }

        Message {
            role: "system".to_string(),
            content,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Get relevant conversation context for the LLM
    fn get_relevant_context(&self) -> Vec<Message> {
        // Get the most recent important messages
        self.context.important_messages
            .iter()
            .rev()
            .take(10)
            .rev()
            .cloned()
            .collect()
    }

    /// Manage context size by pruning old messages
    fn manage_context_size(&mut self) {
        // Keep messages list reasonable
        if self.messages.len() > 50 {
            // Keep system message and recent messages
            let system_msg = self.messages[0].clone();
            let recent_messages = self.messages.split_off(self.messages.len() - 30);
            self.messages = vec![system_msg];
            self.messages.extend(recent_messages);
        }

        // Prune important messages queue
        while self.context.important_messages.len() > 20 {
            self.context.important_messages.pop_front();
        }

        // Prune working memory
        while self.context.working_memory.len() > 10 {
            self.context.working_memory.remove(0);
        }
    }

    /// Update working memory with insights from assistant responses
    fn update_working_memory(&mut self, content: &str) {
        // Extract key insights from assistant responses
        let content_lower = content.to_lowercase();

        if content_lower.contains("created") || content_lower.contains("built") {
            self.context.working_memory.push("Creation/Build completed".to_string());
        }

        if content_lower.contains("error") || content_lower.contains("failed") {
            self.context.working_memory.push("Issue encountered".to_string());
        }

        if content_lower.contains("next") || content_lower.contains("then") {
            self.context.working_memory.push("Planning next steps".to_string());
        }
    }

    /// Execute tools with semantic awareness
    async fn execute_tools_semantically(
        &mut self,
        tool_calls: &[crate::openrouter::ToolCall],
        conversation_messages: &mut Vec<Message>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.config.preferences.verbose || tool_calls.len() > 1 {
            println!("🤖 Making {} tool calls", tool_calls.len());
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

                    // Update working memory based on tool results
                    self.update_memory_from_tool_result(&tool_call.function.name, &result);

                    // Log tool result
                    self.story_logger.log_tool_result(&tool_call.function.name, true, &result);

                    // Create tool response message
                    let tool_message = Message {
                        role: "tool".to_string(),
                        content: result.clone(),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    };
                    conversation_messages.push(tool_message.clone());
                    self.messages.push(tool_message);
                }
                Err(e) => {
                    println!("  ❌ Error: {}", e);

                    // Log tool error
                    self.story_logger.log_tool_result(&tool_call.function.name, false, &e.to_string());

                    // Create error tool response
                    let error_message = Message {
                        role: "tool".to_string(),
                        content: serde_json::json!({"status": "error", "message": e.to_string()}).to_string(),
                        tool_calls: None,
                        tool_call_id: Some(tool_call.id.clone()),
                    };
                    conversation_messages.push(error_message.clone());
                    self.messages.push(error_message);
                }
            }
        }

        Ok(())
    }

    /// Analyze conversation state using LLM with structured JSON response
    async fn analyze_conversation_state_with_llm(&self, user_input: &str) -> Result<ConversationStateAnalysis, Box<dyn std::error::Error>> {
        let system_prompt = r#"You are an expert at analyzing user intent and conversation context for a coding assistant. 

Analyze the user's input and determine their conversation state. Respond with ONLY a JSON object matching this exact schema:

{
  "state": "one of: Conversational, Planning, Implementing, Troubleshooting, Exploring",
  "confidence": 0.85,
  "reasoning": "Brief explanation of why you chose this state",
  "topic": "Main topic or focus area (optional, can be null)",
  "suggested_tools": ["array of potentially useful tool names"]
}

Conversation States:
- Conversational: General discussion, questions, or casual interaction
- Planning: User wants to plan, design, or think through an approach
- Implementing: User is ready to build, create, or implement something specific
- Troubleshooting: User has problems, errors, or things not working
- Exploring: User wants to investigate, understand, or explore existing code/systems

Common tools: ["create_file", "run_command", "read_file", "create_directory", "git_status", "git_add", "git_commit", "run_tests", "create_test", "package_install", "dependency_check", "database_query", "database_migration"]

Respond with ONLY the JSON object, no other text."#;

        let analysis_message = Message {
            role: "user".to_string(),
            content: format!("Analyze this user input: \"{}\"", user_input),
            tool_calls: None,
            tool_call_id: None,
        };

        let system_message = Message {
            role: "system".to_string(),
            content: system_prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        };

        let messages = vec![system_message, analysis_message];

        let response = self.openrouter_client.chat_completion(messages).await?;
        let content = &response.choices[0].message.content;

        // Parse JSON response
        let analysis: ConversationStateAnalysis = serde_json::from_str(content)
            .map_err(|e| format!("Failed to parse LLM response as JSON: {}. Response was: {}", e, content))?;

        Ok(analysis)
    }

    /// Parse string state to ConversationState enum
    fn parse_conversation_state(&self, state_str: &str) -> ConversationState {
        match state_str.to_lowercase().as_str() {
            "planning" => {
                println!("📋 Planning mode activated - Ready to design and strategize your approach");
                ConversationState::Planning
            },
            "implementing" => {
                println!("🔨 Implementation mode activated - Time to build and create");
                ConversationState::Implementing
            },
            "troubleshooting" => {
                println!("🔧 Troubleshooting mode activated - Let's debug and solve problems");
                ConversationState::Troubleshooting
            },
            "exploring" => {
                println!("🔍 Exploration mode activated - Investigating and understanding the codebase");
                ConversationState::Exploring
            },
            _ => {
                println!("💬 Conversational mode activated - Ready to discuss and answer questions");
                ConversationState::Conversational
            }
        }
    }


    /// Update working memory based on tool results
    fn update_memory_from_tool_result(&mut self, tool_name: &str, result: &str) {
        let memory_entry = match tool_name {
            "create_file" => "File created",
            "create_directory" => "Directory created",
            "run_command" => {
                if result.contains("error") || result.contains("Error") {
                    "Command failed"
                } else {
                    "Command executed"
                }
            }
            "read_file" => "File content read",
            _ => return,
        };

        self.context.working_memory.push(memory_entry.to_string());
    }

    /// Clear conversation context (semantic equivalent of /clear)
    pub fn clear_context(&mut self) -> String {
        let message_count = self.messages.len().saturating_sub(1);

        // Keep only system message if any
        if !self.messages.is_empty() && self.messages[0].role == "system" {
            let system_message = self.messages[0].clone();
            self.messages.clear();
            self.messages.push(system_message);
        } else {
            self.messages.clear();
        }

        // Reset conversation context
        self.context = ConversationContext::default();

        format!("🧹 Conversation context cleared ({} messages removed)", message_count)
    }

    /// Change model (semantic equivalent of /model)
    pub async fn change_model(&mut self, new_model: &str) -> Result<String, Box<dyn std::error::Error>> {
        let old_model = self.config.openrouter.model.clone();

        self.config.openrouter.model = new_model.to_string();

        match crate::openrouter::OpenRouterClient::new(self.config.clone()).await {
            Ok(new_client) => {
                self.openrouter_client = new_client;
                Ok(format!("✅ Model changed from '{}' to '{}'", old_model, new_model))
            }
            Err(e) => {
                self.config.openrouter.model = old_model;
                Err(format!("Failed to switch to model '{}': {}", new_model, e).into())
            }
        }
    }

    /// List available models (semantic equivalent of /list-models)
    pub async fn list_models(&self, search_term: &str) -> Result<String, Box<dyn std::error::Error>> {
        match self.openrouter_client.list_models(search_term).await {
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
}

/// File system autocomplete for semantic engine
#[derive(Clone)]
pub struct CustomTextAutocomplete {
    working_dir: String,
}

impl CustomTextAutocomplete {
    pub fn new(working_dir: String) -> Self {
        Self {
            working_dir,
        }
    }
}

impl Autocomplete for CustomTextAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        // Handle filesystem autocomplete if '@' is present
        if input.contains('@') {
            let last_at = input.rfind('@').unwrap();
            let before_at = &input[..last_at];
            let after_at = &input[last_at + 1..];

            // Check if this is a folder path that should show contents
            if after_at.ends_with('/') && !after_at.trim_end_matches('/').is_empty() {
                let folder_contents = self.get_folder_contents(after_at);

                if !folder_contents.is_empty() {
                    let drill_suggestions: Vec<String> = folder_contents
                        .into_iter()
                        .map(|item| format!("{}@{}{}", before_at, after_at, item))
                        .collect();

                    return Ok(drill_suggestions);
                }
            }

            // Regular filesystem autocomplete
            let suggestions = self.get_file_suggestions(after_at);

            let full_suggestions: Vec<String> = suggestions
                .into_iter()
                .map(|suggestion| format!("{}@{}", before_at, suggestion))
                .collect();

            return Ok(full_suggestions);
        }

        // No suggestions for regular text
        Ok(vec![])
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(match highlighted_suggestion {
            Some(suggestion) => inquire::autocompletion::Replacement::Some(suggestion),
            None => inquire::autocompletion::Replacement::None,
        })
    }
}

impl CustomTextAutocomplete {
    fn get_folder_contents(&self, folder_path: &str) -> Vec<String> {
        let clean_path = folder_path.trim_end_matches('/');
        let full_path = Path::new(&self.working_dir).join(clean_path);
        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(&full_path) {
            for entry in dir_entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.starts_with('.') && !folder_path.contains("/.") {
                        continue;
                    }

                    if metadata.is_dir() {
                        entries.push(format!("{}/", name));
                    } else {
                        entries.push(name);
                    }
                }
            }
        }

        entries.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        entries
    }

    fn get_file_suggestions(&self, partial_path: &str) -> Vec<String> {
        if partial_path.is_empty() {
            return self.list_directory(".");
        }

        let path = Path::new(partial_path);
        let (dir_path, file_prefix) = if partial_path.ends_with('/') {
            (partial_path.trim_end_matches('/').to_string(), String::new())
        } else {
            match path.parent() {
                Some(parent) => {
                    let parent_str = parent.to_string_lossy().to_string();
                    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    (parent_str, file_name)
                }
                None => (".".to_string(), partial_path.to_string()),
            }
        };

        let dir_path_str = if dir_path.is_empty() { "." } else { &dir_path };
        let entries = self.list_directory(dir_path_str);

        entries
            .into_iter()
            .filter(|entry| entry.starts_with(&file_prefix))
            .collect()
    }

    fn list_directory(&self, relative_path: &str) -> Vec<String> {
        let full_path = Path::new(&self.working_dir).join(relative_path);
        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(&full_path) {
            for entry in dir_entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.starts_with('.') && !relative_path.contains("/.") {
                        continue;
                    }

                    let entry_path = if relative_path == "." {
                        if metadata.is_dir() {
                            format!("{}/", name)
                        } else {
                            name
                        }
                    } else {
                        let clean_relative_path = relative_path.trim_end_matches('/');
                        if metadata.is_dir() {
                            format!("{}/{}/", clean_relative_path, name)
                        } else {
                            format!("{}/{}", clean_relative_path, name)
                        }
                    };

                    entries.push(entry_path);
                }
            }
        }

        entries.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.cmp(b),
            }
        });

        entries
    }
}