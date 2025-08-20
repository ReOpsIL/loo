use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use warp::Filter;

pub struct MockOpenRouterServer {
    port: u16,
    scenarios: Arc<Mutex<HashMap<String, MockScenario>>>,
}

#[derive(Clone, Debug)]
pub struct MockScenario {
    pub prompt: String,
    pub responses: Vec<MockResponse>,
    pub current_step: usize,
}

#[derive(Clone, Debug)]
pub struct MockResponse {
    pub message: Option<String>,
    pub tool_calls: Vec<MockToolCall>,
}

#[derive(Clone, Debug)]
pub struct MockToolCall {
    pub id: String,
    pub function_name: String,
    pub arguments: Value,
}

impl MockOpenRouterServer {
    pub fn new() -> Self {
        Self {
            port: 0, // Will be assigned when started
            scenarios: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_scenario(&self, name: String, scenario: MockScenario) {
        self.scenarios.lock().unwrap().insert(name, scenario);
    }

    pub async fn start(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let scenarios = self.scenarios.clone();
        
        let chat_completions = warp::path!("v1" / "chat" / "completions")
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("authorization"))
            .and_then(move |request: Value, auth: Option<String>| {
                let scenarios = scenarios.clone();
                async move {
                    handle_chat_completion(request, auth, scenarios).await
                }
            });

        let routes = chat_completions.with(warp::cors().allow_any_origin());

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        self.port = addr.port();
        
        tokio::spawn(async move {
            warp::serve(routes).run_incoming(
                tokio_stream::wrappers::TcpListenerStream::new(listener)
            ).await;
        });

        Ok(format!("http://127.0.0.1:{}", self.port))
    }

    pub fn get_base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

async fn handle_chat_completion(
    request: Value,
    _auth: Option<String>,
    scenarios: Arc<Mutex<HashMap<String, MockScenario>>>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let messages = request["messages"].as_array().unwrap();
    let user_message = messages
        .iter()
        .find(|msg| msg["role"] == "user")
        .and_then(|msg| msg["content"].as_str())
        .unwrap_or("");

    let mut scenarios_guard = scenarios.lock().unwrap();
    
    // Find matching scenario based on user message
    let scenario_name = find_matching_scenario(&scenarios_guard, user_message);
    
    if let Some(name) = scenario_name {
        if let Some(scenario) = scenarios_guard.get_mut(&name) {
            if scenario.current_step < scenario.responses.len() {
                let response = &scenario.responses[scenario.current_step].clone();
                scenario.current_step += 1;
                
                let mut choice = json!({
                    "message": {
                        "role": "assistant",
                        "content": response.message.as_deref().unwrap_or("")
                    }
                });

                if !response.tool_calls.is_empty() {
                    let tool_calls: Vec<Value> = response.tool_calls
                        .iter()
                        .map(|tc| json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.function_name,
                                "arguments": serde_json::to_string(&tc.arguments).unwrap()
                            }
                        }))
                        .collect();
                    
                    choice["message"]["tool_calls"] = json!(tool_calls);
                }

                return Ok(warp::reply::json(&json!({
                    "choices": [choice]
                })));
            }
        }
    }

    // Default response if no scenario matches
    Ok(warp::reply::json(&json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": "I'll help you with that task.",
                "tool_calls": [{
                    "id": "call_default",
                    "type": "function",
                    "function": {
                        "name": "complete",
                        "arguments": "{}"
                    }
                }]
            }
        }]
    })))
}

fn find_matching_scenario(
    scenarios: &HashMap<String, MockScenario>,
    user_message: &str,
) -> Option<String> {
    for (name, scenario) in scenarios.iter() {
        if user_message.to_lowercase().contains(&scenario.prompt.to_lowercase()) {
            return Some(name.clone());
        }
    }
    None
}

// Predefined scenarios for common test cases
impl MockScenario {
    pub fn simple_file_creation() -> Self {
        Self {
            prompt: "create a simple hello world".to_string(),
            responses: vec![
                MockResponse {
                    message: Some("I'll create a simple Hello World program for you.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_1".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "hello.py",
                                "content": "print('Hello, World!')\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Perfect! I've created the Hello World program.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_2".to_string(),
                            function_name: "complete".to_string(),
                            arguments: json!({}),
                        }
                    ],
                },
            ],
            current_step: 0,
        }
    }

    pub fn rust_project_creation() -> Self {
        Self {
            prompt: "create a rust project".to_string(),
            responses: vec![
                MockResponse {
                    message: Some("I'll create a basic Rust project structure for you.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_1".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "Cargo.toml",
                                "content": "[package]\nname = \"test-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Now I'll create the main source file.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_2".to_string(),
                            function_name: "create_directory".to_string(),
                            arguments: json!({
                                "path": "src"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Creating the main.rs file.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_3".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "src/main.rs",
                                "content": "fn main() {\n    println!(\"Hello, world!\");\n}\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Let me test if the project builds correctly.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_4".to_string(),
                            function_name: "run_command".to_string(),
                            arguments: json!({
                                "command": "cargo check"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Great! The Rust project has been created and builds successfully.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_5".to_string(),
                            function_name: "complete".to_string(),
                            arguments: json!({}),
                        }
                    ],
                },
            ],
            current_step: 0,
        }
    }

    pub fn multi_file_project() -> Self {
        Self {
            prompt: "create a web server".to_string(),
            responses: vec![
                MockResponse {
                    message: Some("I'll create a simple web server project with multiple files.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_1".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "server.py",
                                "content": "from flask import Flask\n\napp = Flask(__name__)\n\n@app.route('/')\ndef hello():\n    return 'Hello, World!'\n\nif __name__ == '__main__':\n    app.run(debug=True)\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Now I'll create a requirements file.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_2".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "requirements.txt",
                                "content": "Flask==2.3.3\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Let me also create a README file.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_3".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "README.md",
                                "content": "# Simple Web Server\n\nA basic Flask web server.\n\n## Setup\n\n```bash\npip install -r requirements.txt\npython server.py\n```\n"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Perfect! I've created a complete web server project.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_4".to_string(),
                            function_name: "complete".to_string(),
                            arguments: json!({}),
                        }
                    ],
                },
            ],
            current_step: 0,
        }
    }

    pub fn error_handling_scenario() -> Self {
        Self {
            prompt: "test error handling".to_string(),
            responses: vec![
                MockResponse {
                    message: Some("I'll test error handling by trying to read a non-existent file.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_1".to_string(),
                            function_name: "read_file".to_string(),
                            arguments: json!({
                                "path": "non-existent-file.txt"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("As expected, that file doesn't exist. Let me create it and try again.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_2".to_string(),
                            function_name: "create_file".to_string(),
                            arguments: json!({
                                "path": "test-file.txt",
                                "content": "This file now exists!"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Now let me read the file successfully.".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_3".to_string(),
                            function_name: "read_file".to_string(),
                            arguments: json!({
                                "path": "test-file.txt"
                            }),
                        }
                    ],
                },
                MockResponse {
                    message: Some("Error handling test completed successfully!".to_string()),
                    tool_calls: vec![
                        MockToolCall {
                            id: "call_4".to_string(),
                            function_name: "complete".to_string(),
                            arguments: json!({}),
                        }
                    ],
                },
            ],
            current_step: 0,
        }
    }
}