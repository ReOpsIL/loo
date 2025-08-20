use std::cmp::min;
use crate::config::Config;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

#[derive(Serialize)]
pub struct OpenRouterRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub tool_choice: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ToolFunction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ToolCallFunction,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Deserialize)]
pub struct OpenRouterResponse {
    pub choices: Vec<Choice>,
}

#[derive(Deserialize)]
pub struct ApiError {
    pub message: String,
    pub code: i32,
}

#[derive(Deserialize)]
pub struct ErrorResponse {
    pub error: ApiError,
}

#[derive(Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

#[derive(Deserialize)]
pub struct Model {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
}

pub struct OpenRouterClient {
    client: reqwest::Client,
    config: Config,
}

impl OpenRouterClient {
    pub async fn new(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Get API key from config or environment
        let api_key = config.openrouter.api_key
            .clone()
            .or_else(|| env::var("OPENROUTER_API_KEY").ok())
            .ok_or("OpenRouter API key not found. Set it in config file or OPENROUTER_API_KEY environment variable")?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", api_key).parse()?,
        );
        // headers.insert("HTTP-Referer", "https://github.com/loo".parse()?);
        headers.insert("X-Title", "Break CLI".parse()?);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(config.tools.command_timeout))
            .build()?;

        if config.preferences.verbose {
            println!("🔧 Using model: {}", config.openrouter.model);
            println!("🔧 API endpoint: {}/chat/completions", config.openrouter.base_url);
        }

        Ok(Self { client, config })
    }

    pub fn get_tools(&self) -> Vec<Tool> {
        let mut tools = Vec::new();

        if self.config.tools.filesystem {
            tools.extend(vec![
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "create_file".to_string(),
                        description: "Create a new file with specified content".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "File path to create"},
                                "content": {"type": "string", "description": "File content"}
                            },
                            "required": ["path", "content"]
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "read_file".to_string(),
                        description: "Read the contents of a file".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "File path to read"}
                            },
                            "required": ["path"]
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "write_file".to_string(),
                        description: "Write content to an existing file".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "File path to write to"},
                                "content": {"type": "string", "description": "Content to write"}
                            },
                            "required": ["path", "content"]
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "delete_file".to_string(),
                        description: "Delete a file".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "File path to delete"}
                            },
                            "required": ["path"]
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "create_directory".to_string(),
                        description: "Create a directory and any necessary parent directories".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "Directory path to create"}
                            },
                            "required": ["path"]
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "list_directory".to_string(),
                        description: "List contents of a directory".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "path": {"type": "string", "description": "Directory path to list (defaults to current directory)"}
                            }
                        }),
                    },
                },
                Tool {
                    tool_type: "function".to_string(),
                    function: ToolFunction {
                        name: "query_context".to_string(),
                        description: "Query project context and current state".to_string(),
                        parameters: json!({
                            "type": "object",
                            "properties": {
                                "type": {"type": "string", "enum": ["full", "directory"], "description": "Type of context query"}
                            }
                        }),
                    },
                },
            ]);
        }

        if self.config.tools.commands {
            tools.push(Tool {
                tool_type: "function".to_string(),
                function: ToolFunction {
                    name: "run_command".to_string(),
                    description: "Execute a shell command".to_string(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "command": {"type": "string", "description": "Command to execute"}
                        },
                        "required": ["command"]
                    }),
                },
            });
        }

        // Always include completion tool
        tools.push(Tool {
            tool_type: "function".to_string(),
            function: ToolFunction {
                name: "complete".to_string(),
                description: "Mark the project as completed".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        });

        tools
    }

    pub async fn chat_completion(
        &self,
        messages: Vec<Message>,
    ) -> Result<OpenRouterResponse, Box<dyn std::error::Error>> {
        let request = OpenRouterRequest {
            model: self.config.openrouter.model.clone(),
            messages,
            tools: self.get_tools(),
            tool_choice: "auto".to_string(),
        };

        let endpoint = format!("{}/chat/completions", self.config.openrouter.base_url);
        
        if self.config.preferences.verbose {
            println!("🔗 Sending request to: {}", endpoint);
            println!("📊 Request: {} messages, {} tools", request.messages.len(), request.tools.len());
        }

        let raw_response = self
            .client
            .post(&endpoint)
            .json(&request)
            .send()
            .await?;

        // Log the raw response for debugging
        let response_text = raw_response.text().await?;
        if self.config.preferences.verbose {
            let max_len = min(80, response_text.len());
            println!("🐛 Raw API response: {}", response_text.get(..max_len).unwrap());
        }

        // Try to parse as error response first
        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&response_text) {
            return Err(format!("OpenRouter API Error: {} (code: {})", 
                error_response.error.message, error_response.error.code).into());
        }

        let response: OpenRouterResponse = serde_json::from_str(&response_text)?;

        Ok(response)
    }

    pub async fn list_models(&self, search_term: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let endpoint = format!("{}/models", self.config.openrouter.base_url);
        
        if self.config.preferences.verbose {
            println!("🔗 Fetching models from: {}", endpoint);
        }

        let raw_response = self
            .client
            .get(&endpoint)
            .send()
            .await?;

        let response_text = raw_response.text().await?;
        if self.config.preferences.verbose {
            let max_len = min(80, response_text.len());
            println!("🐛 Raw models response: {}", response_text.get(..max_len).unwrap());
        }

        let models_response: ModelsResponse = serde_json::from_str(&response_text)?;
        
        let mut model_names: Vec<String> = models_response.data
            .into_iter()
            .map(|model| model.id)
            .collect();

        // Filter models if search term is provided
        if !search_term.is_empty() {
            let search_lower = search_term.to_lowercase();
            model_names.retain(|name| name.to_lowercase().contains(&search_lower));
        }

        // Sort models alphabetically
        model_names.sort();

        Ok(model_names)
    }
}