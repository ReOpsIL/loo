use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

pub struct BreakTestEnvironment {
    pub temp_dir: TempDir,
    pub working_dir: PathBuf,
    pub config_dir: TempDir,
    pub mock_server_url: Option<String>,
}

impl BreakTestEnvironment {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let working_dir = temp_dir.path().to_path_buf();
        
        let config_dir = TempDir::new()?;
        
        Ok(Self {
            temp_dir,
            working_dir,
            config_dir,
            mock_server_url: None,
        })
    }

    pub fn set_mock_server_url(&mut self, url: String) {
        self.mock_server_url = Some(url);
    }

    pub fn create_test_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_content = format!(
            r#"
[openrouter]
api_key = "test-api-key"
model = "test-model"
base_url = "{}"

[preferences]
verbose = true
auto_confirm = false

[tools]
filesystem = true
commands = true
git = true
command_timeout = 30
"#,
            self.mock_server_url.as_ref().unwrap_or(&"http://localhost:3000".to_string())
        );

        let config_path = self.config_dir.path().join("loo").join("config.toml");
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(config_path, config_content)?;
        
        Ok(())
    }

    pub async fn run_break_command(
        &self,
        args: &[&str],
    ) -> Result<BreakCommandResult, Box<dyn std::error::Error>> {
        let output = Command::new("cargo")
            .args(&["run", "--bin", "loo", "--"])
            .args(args)
            .current_dir(&self.working_dir)
            .env("XDG_CONFIG_HOME", self.config_dir.path())
            .env("APPDATA", self.config_dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        Ok(BreakCommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        })
    }

    pub async fn run_interactive_break_command(
        &self,
        args: &[&str],
        timeout_seconds: u64,
    ) -> Result<BreakCommandResult, Box<dyn std::error::Error>> {
        let mut child = Command::new("cargo")
            .args(&["run", "--bin", "loo", "--"])
            .args(args)
            .current_dir(&self.working_dir)
            .env("XDG_CONFIG_HOME", self.config_dir.path())
            .env("APPDATA", self.config_dir.path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for the command to complete or timeout
        let mut elapsed = 0;
        let check_interval = 100; // milliseconds
        
        while elapsed < timeout_seconds * 1000 {
            if let Ok(Some(status)) = child.try_wait() {
                let output = child.wait_with_output()?;
                return Ok(BreakCommandResult {
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    exit_code: status.code(),
                    success: status.success(),
                });
            }
            
            sleep(Duration::from_millis(check_interval)).await;
            elapsed += check_interval;
        }

        // Kill the process if it's still running
        let _ = child.kill();
        let output = child.wait_with_output()?;
        
        Ok(BreakCommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: Some(124), // Timeout exit code
            success: false,
        })
    }

    pub fn assert_file_exists(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let full_path = self.working_dir.join(path);
        if !full_path.exists() {
            return Err(format!("File does not exist: {}", path).into());
        }
        Ok(())
    }

    pub fn assert_file_contains(&self, path: &str, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let full_path = self.working_dir.join(path);
        let file_content = fs::read_to_string(full_path)?;
        if !file_content.contains(content) {
            return Err(format!("File '{}' does not contain '{}'", path, content).into());
        }
        Ok(())
    }

    pub fn assert_directory_exists(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let full_path = self.working_dir.join(path);
        if !full_path.is_dir() {
            return Err(format!("Directory does not exist: {}", path).into());
        }
        Ok(())
    }

    pub fn get_file_content(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        let full_path = self.working_dir.join(path);
        Ok(fs::read_to_string(full_path)?)
    }

    pub fn list_files(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.working_dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                files.push(name.to_string());
            }
        }
        Ok(files)
    }
}

#[derive(Debug)]
pub struct BreakCommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub success: bool,
}

impl BreakCommandResult {
    pub fn assert_success(&self) -> Result<(), Box<dyn std::error::Error>> {
        if !self.success {
            return Err(format!(
                "Command failed with exit code {:?}\nStdout: {}\nStderr: {}", 
                self.exit_code, self.stdout, self.stderr
            ).into());
        }
        Ok(())
    }

    pub fn assert_contains_stdout(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.stdout.contains(text) {
            return Err(format!(
                "Stdout does not contain '{}'\nActual stdout: {}", 
                text, self.stdout
            ).into());
        }
        Ok(())
    }

    pub fn assert_contains_stderr(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        if !self.stderr.contains(text) {
            return Err(format!(
                "Stderr does not contain '{}'\nActual stderr: {}", 
                text, self.stderr
            ).into());
        }
        Ok(())
    }

    pub fn assert_exit_code(&self, expected: i32) -> Result<(), Box<dyn std::error::Error>> {
        if self.exit_code != Some(expected) {
            return Err(format!(
                "Expected exit code {}, got {:?}", 
                expected, self.exit_code
            ).into());
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! e2e_test {
    ($name:ident, $test_fn:expr) => {
        #[tokio::test]
        async fn $name() -> Result<(), Box<dyn std::error::Error>> {
            let test_env = BreakTestEnvironment::new().await?;
            $test_fn(test_env).await
        }
    };
}

// Test assertion helpers
pub struct TestAssertions;

impl TestAssertions {
    pub fn files_created<'a>(expected: &'a [&'a str]) -> Box<dyn Fn(&BreakTestEnvironment) -> Result<(), Box<dyn std::error::Error>> + 'a> {
        let expected = expected.to_vec();
        Box::new(move |env| {
            for file in &expected {
                env.assert_file_exists(file)?;
            }
            Ok(())
        })
    }

    pub fn file_contains(file: &str, content: &str) -> Box<dyn Fn(&BreakTestEnvironment) -> Result<(), Box<dyn std::error::Error>>> {
        let file = file.to_string();
        let content = content.to_string();
        Box::new(move |env| {
            env.assert_file_contains(&file, &content)
        })
    }

    pub fn directories_created<'a>(expected: &'a [&'a str]) -> Box<dyn Fn(&BreakTestEnvironment) -> Result<(), Box<dyn std::error::Error>> + 'a> {
        let expected = expected.to_vec();
        Box::new(move |env| {
            for dir in &expected {
                env.assert_directory_exists(dir)?;
            }
            Ok(())
        })
    }

    pub fn command_succeeded() -> Box<dyn Fn(&BreakCommandResult) -> Result<(), Box<dyn std::error::Error>>> {
        Box::new(|result| result.assert_success())
    }

    pub fn output_contains(text: &str) -> Box<dyn Fn(&BreakCommandResult) -> Result<(), Box<dyn std::error::Error>>> {
        let text = text.to_string();
        Box::new(move |result| result.assert_contains_stdout(&text))
    }
}