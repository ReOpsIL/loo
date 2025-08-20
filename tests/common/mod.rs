// Common test utilities and fixtures

use std::sync::Once;

static INIT: Once = Once::new();

/// Initialize test environment
pub fn setup() {
    INIT.call_once(|| {
        // Setup logging for tests if needed
        env_logger::init();
    });
}

/// Test data fixtures
pub mod fixtures {
    pub const SAMPLE_RUST_PROJECT: &str = r#"
[package]
name = "sample-project"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = "4.0"
serde = "1.0"
"#;

    pub const SAMPLE_MAIN_RS: &str = r#"
use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long)]
    name: String,
}

fn main() {
    let cli = Cli::parse();
    println!("Hello, {}!", cli.name);
}
"#;

    pub const SAMPLE_PYTHON_SCRIPT: &str = r#"
#!/usr/bin/env python3

def main():
    print("Hello, World!")

if __name__ == "__main__":
    main()
"#;

    pub const SAMPLE_CONFIG_TOML: &str = r#"
[openrouter]
model = "anthropic/claude-3.5-sonnet"
base_url = "https://openrouter.ai/api/v1"

[preferences]
verbose = true
auto_confirm = false

[tools]
filesystem = true
commands = true
git = true
command_timeout = 300
"#;
}

/// Mock responses for testing
pub mod mock_responses {
    use serde_json::json;

    pub fn simple_file_creation_response() -> serde_json::Value {
        json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "I'll create a simple file for you.",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "create_file",
                            "arguments": "{\"path\":\"hello.py\",\"content\":\"print('Hello, World!')\"}"
                        }
                    }]
                }
            }]
        })
    }

    pub fn project_completion_response() -> serde_json::Value {
        json!({
            "choices": [{
                "message": {
                    "role": "assistant", 
                    "content": "Project completed successfully!",
                    "tool_calls": [{
                        "id": "call_complete",
                        "type": "function",
                        "function": {
                            "name": "complete",
                            "arguments": "{}"
                        }
                    }]
                }
            }]
        })
    }

    pub fn error_response() -> serde_json::Value {
        json!({
            "error": {
                "message": "API key not valid",
                "type": "authentication_error"
            }
        })
    }
}

/// Test assertion helpers
pub mod assertions {
    use std::path::Path;
    use std::fs;

    pub fn assert_file_exists_with_content<P: AsRef<Path>>(path: P, expected_content: &str) {
        let path = path.as_ref();
        assert!(path.exists(), "File should exist: {}", path.display());
        
        let content = fs::read_to_string(path)
            .unwrap_or_else(|_| panic!("Failed to read file: {}", path.display()));
        
        assert!(
            content.contains(expected_content),
            "File content should contain '{}', but was: {}",
            expected_content,
            content
        );
    }

    pub fn assert_directory_structure(base_path: &Path, expected_files: &[&str]) {
        for file in expected_files {
            let file_path = base_path.join(file);
            assert!(
                file_path.exists(),
                "Expected file/directory should exist: {}",
                file_path.display()
            );
        }
    }

    pub fn assert_command_output_contains(output: &std::process::Output, expected: &str) {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        assert!(
            stdout.contains(expected) || stderr.contains(expected),
            "Output should contain '{}'\nStdout: {}\nStderr: {}",
            expected,
            stdout,
            stderr
        );
    }
}

/// Environment helpers for tests
pub mod test_env {
    use std::env;
    use tempfile::TempDir;

    pub struct TestEnvironment {
        pub temp_dir: TempDir,
        pub original_env: Vec<(String, Option<String>)>,
    }

    impl TestEnvironment {
        pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
            let temp_dir = TempDir::new()?;
            
            // Store original environment variables that we might modify
            let env_vars = ["XDG_CONFIG_HOME", "APPDATA", "OPENROUTER_API_KEY", "OPENROUTER_MODEL"];
            let original_env = env_vars
                .iter()
                .map(|var| (var.to_string(), env::var(var).ok()))
                .collect();
            
            Ok(Self {
                temp_dir,
                original_env,
            })
        }
        
        pub fn set_config_dir(&self) {
            env::set_var("XDG_CONFIG_HOME", self.temp_dir.path());
            env::set_var("APPDATA", self.temp_dir.path());
        }
        
        pub fn set_api_key(&self, key: &str) {
            env::set_var("OPENROUTER_API_KEY", key);
        }
        
        pub fn set_model(&self, model: &str) {
            env::set_var("OPENROUTER_MODEL", model);
        }
    }

    impl Drop for TestEnvironment {
        fn drop(&mut self) {
            // Restore original environment variables
            for (var, value) in &self.original_env {
                match value {
                    Some(val) => env::set_var(var, val),
                    None => env::remove_var(var),
                }
            }
        }
    }
}