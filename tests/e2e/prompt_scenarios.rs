use super::framework::*;
use crate::mocks::*;

#[tokio::test]
async fn test_simple_hello_world_creation() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "hello_world".to_string(),
        MockScenario::simple_file_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command with a simple prompt
    let result = test_env
        .run_interactive_break_command(
            &["start", "create a simple hello world program"],
            30, // 30 second timeout
        )
        .await?;

    // Verify the command succeeded
    result.assert_success()?;
    result.assert_contains_stderr("🚀 Starting Break CLI")?;
    result.assert_contains_stderr("🎉 Project completed")?;

    // Verify the file was created with correct content
    test_env.assert_file_exists("hello.py")?;
    test_env.assert_file_contains("hello.py", "print('Hello, World!')")?;

    println!("✅ Simple Hello World test passed");
    Ok(())
}

#[tokio::test]
async fn test_rust_project_creation() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server with Rust project scenario
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "rust_project".to_string(),
        MockScenario::rust_project_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command to create a Rust project
    let result = test_env
        .run_interactive_break_command(
            &["start", "create a rust project with basic structure"],
            45, // 45 second timeout for more complex operation
        )
        .await?;

    // Verify the command succeeded
    result.assert_success()?;
    result.assert_contains_stderr("🚀 Starting Break CLI")?;
    result.assert_contains_stderr("🎉 Project completed")?;

    // Verify project structure was created
    test_env.assert_file_exists("Cargo.toml")?;
    test_env.assert_file_contains("Cargo.toml", "name = \"test-project\"")?;
    test_env.assert_file_contains("Cargo.toml", "edition = \"2021\"")?;
    
    test_env.assert_directory_exists("src")?;
    test_env.assert_file_exists("src/main.rs")?;
    test_env.assert_file_contains("src/main.rs", "fn main()")?;
    test_env.assert_file_contains("src/main.rs", "println!(\"Hello, world!\");")?;

    println!("✅ Rust project creation test passed");
    Ok(())
}

#[tokio::test]
async fn test_multi_file_web_server() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server with web server scenario
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "web_server".to_string(),
        MockScenario::multi_file_project(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command to create a web server
    let result = test_env
        .run_interactive_break_command(
            &["start", "create a web server with Flask"],
            60, // 60 second timeout for multi-file project
        )
        .await?;

    // Verify the command succeeded
    result.assert_success()?;
    result.assert_contains_stderr("🚀 Starting Break CLI")?;
    result.assert_contains_stderr("🎉 Project completed")?;

    // Verify all files were created
    test_env.assert_file_exists("server.py")?;
    test_env.assert_file_contains("server.py", "from flask import Flask")?;
    test_env.assert_file_contains("server.py", "@app.route('/')")?;
    
    test_env.assert_file_exists("requirements.txt")?;
    test_env.assert_file_contains("requirements.txt", "Flask==")?;
    
    test_env.assert_file_exists("README.md")?;
    test_env.assert_file_contains("README.md", "# Simple Web Server")?;

    println!("✅ Multi-file web server test passed");
    Ok(())
}

#[tokio::test]
async fn test_error_handling_and_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server with error handling scenario
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "error_test".to_string(),
        MockScenario::error_handling_scenario(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command to test error handling
    let result = test_env
        .run_interactive_break_command(
            &["start", "test error handling with file operations"],
            40, // 40 second timeout
        )
        .await?;

    // Verify the command succeeded despite encountering errors
    result.assert_success()?;
    result.assert_contains_stderr("🚀 Starting Break CLI")?;
    result.assert_contains_stderr("🎉 Project completed")?;

    // Verify the test file was created after the error
    test_env.assert_file_exists("test-file.txt")?;
    test_env.assert_file_contains("test-file.txt", "This file now exists!")?;

    println!("✅ Error handling and recovery test passed");
    Ok(())
}

#[tokio::test]
async fn test_verbose_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "verbose_test".to_string(),
        MockScenario::simple_file_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command with verbose flag
    let result = test_env
        .run_interactive_break_command(
            &["start", "--verbose", "create a simple hello world program"],
            30,
        )
        .await?;

    // Verify verbose output is present
    result.assert_success()?;
    result.assert_contains_stderr("🔧 Using model:")?;
    result.assert_contains_stderr("🔧 API endpoint:")?;

    println!("✅ Verbose mode test passed");
    Ok(())
}

#[tokio::test]
async fn test_custom_model_override() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "model_test".to_string(),
        MockScenario::simple_file_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command with custom model
    let result = test_env
        .run_interactive_break_command(
            &[
                "start",
                "--model", "anthropic/claude-3.5-sonnet",
                "--verbose",
                "create a simple hello world program"
            ],
            30,
        )
        .await?;

    // Verify the custom model was used
    result.assert_success()?;
    result.assert_contains_stderr("🔧 Using model: anthropic/claude-3.5-sonnet")?;

    println!("✅ Custom model override test passed");
    Ok(())
}

#[tokio::test]
async fn test_working_directory_option() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "workdir_test".to_string(),
        MockScenario::simple_file_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Create a subdirectory
    let subdir = test_env.working_dir.join("subproject");
    std::fs::create_dir_all(&subdir)?;

    // Run the loo command with custom working directory
    let result = test_env
        .run_interactive_break_command(
            &[
                "start",
                "--dir", subdir.to_str().unwrap(),
                "create a simple hello world program"
            ],
            30,
        )
        .await?;

    // Verify the command succeeded
    result.assert_success()?;

    // Verify the file was created in the subdirectory
    let hello_file = subdir.join("hello.py");
    assert!(hello_file.exists(), "File should be created in subdirectory");

    println!("✅ Working directory option test passed");
    Ok(())
}

#[tokio::test]
async fn test_session_id_generation() -> Result<(), Box<dyn std::error::Error>> {
    // Setup mock server
    let mut mock_server = MockOpenRouterServer::new();
    mock_server.add_scenario(
        "session_test".to_string(),
        MockScenario::simple_file_creation(),
    );
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the loo command
    let result = test_env
        .run_interactive_break_command(
            &["start", "create a simple hello world program"],
            30,
        )
        .await?;

    // Verify session ID is shown
    result.assert_success()?;
    result.assert_contains_stderr("🆔 Session ID:")?;

    println!("✅ Session ID generation test passed");
    Ok(())
}

// Integration test with realistic scenarios
#[tokio::test]
async fn test_realistic_development_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // This test simulates a realistic development workflow where an LLM
    // creates a project, adds features, runs tests, and fixes issues

    let mut mock_server = MockOpenRouterServer::new();
    
    // Create a complex scenario for a development workflow
    let workflow_scenario = MockScenario {
        prompt: "build a todo cli".to_string(),
        responses: vec![
            MockResponse {
                message: Some("I'll create a comprehensive TODO CLI application for you.".to_string()),
                tool_calls: vec![
                    MockToolCall {
                        id: "call_1".to_string(),
                        function_name: "create_file".to_string(),
                        arguments: serde_json::json!({
                            "path": "Cargo.toml",
                            "content": "[package]\nname = \"todo-cli\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nclap = { version = \"4.0\", features = [\"derive\"] }\nserde = { version = \"1.0\", features = [\"derive\"] }\nserde_json = \"1.0\"\n"
                        }),
                    }
                ],
            },
            MockResponse {
                message: Some("Now I'll create the main application structure.".to_string()),
                tool_calls: vec![
                    MockToolCall {
                        id: "call_2".to_string(),
                        function_name: "create_directory".to_string(),
                        arguments: serde_json::json!({
                            "path": "src"
                        }),
                    }
                ],
            },
            MockResponse {
                message: Some("Creating the main application file.".to_string()),
                tool_calls: vec![
                    MockToolCall {
                        id: "call_3".to_string(),
                        function_name: "create_file".to_string(),
                        arguments: serde_json::json!({
                            "path": "src/main.rs",
                            "content": "use clap::{Parser, Subcommand};\nuse serde::{Deserialize, Serialize};\nuse std::fs;\n\n#[derive(Parser)]\n#[command(name = \"todo\")]\nstruct Cli {\n    #[command(subcommand)]\n    command: Commands,\n}\n\n#[derive(Subcommand)]\nenum Commands {\n    Add { task: String },\n    List,\n    Done { id: usize },\n}\n\n#[derive(Serialize, Deserialize)]\nstruct Todo {\n    id: usize,\n    task: String,\n    done: bool,\n}\n\nfn main() {\n    println!(\"TODO CLI v0.1.0\");\n}\n"
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
                        arguments: serde_json::json!({
                            "command": "cargo check"
                        }),
                    }
                ],
            },
            MockResponse {
                message: Some("Great! Now let me create a README with usage instructions.".to_string()),
                tool_calls: vec![
                    MockToolCall {
                        id: "call_5".to_string(),
                        function_name: "create_file".to_string(),
                        arguments: serde_json::json!({
                            "path": "README.md",
                            "content": "# TODO CLI\n\nA simple command-line TODO application.\n\n## Usage\n\n```bash\n# Add a task\ntodo add \"Buy groceries\"\n\n# List all tasks\ntodo list\n\n# Mark task as done\ntodo done 1\n```\n\n## Building\n\n```bash\ncargo build --release\n```\n"
                        }),
                    }
                ],
            },
            MockResponse {
                message: Some("Perfect! I've created a complete TODO CLI application with proper structure.".to_string()),
                tool_calls: vec![
                    MockToolCall {
                        id: "call_6".to_string(),
                        function_name: "complete".to_string(),
                        arguments: serde_json::json!({}),
                    }
                ],
            },
        ],
        current_step: 0,
    };

    mock_server.add_scenario("todo_cli".to_string(), workflow_scenario);
    let server_url = mock_server.start().await?;

    // Setup test environment
    let mut test_env = BreakTestEnvironment::new().await?;
    test_env.set_mock_server_url(server_url);
    test_env.create_test_config()?;

    // Run the comprehensive workflow
    let result = test_env
        .run_interactive_break_command(
            &["start", "build a todo cli application with proper structure"],
            90, // 90 second timeout for complex workflow
        )
        .await?;

    // Verify the workflow completed successfully
    result.assert_success()?;
    result.assert_contains_stderr("🚀 Starting Break CLI")?;
    result.assert_contains_stderr("🎉 Project completed")?;

    // Verify all components were created
    test_env.assert_file_exists("Cargo.toml")?;
    test_env.assert_file_contains("Cargo.toml", "name = \"todo-cli\"")?;
    test_env.assert_file_contains("Cargo.toml", "clap")?;
    
    test_env.assert_directory_exists("src")?;
    test_env.assert_file_exists("src/main.rs")?;
    test_env.assert_file_contains("src/main.rs", "use clap::{Parser, Subcommand};")?;
    test_env.assert_file_contains("src/main.rs", "struct Todo")?;
    
    test_env.assert_file_exists("README.md")?;
    test_env.assert_file_contains("README.md", "# TODO CLI")?;
    test_env.assert_file_contains("README.md", "## Usage")?;

    println!("✅ Realistic development workflow test passed");
    Ok(())
}