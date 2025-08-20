use std::process::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_cli_help_output() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LLM-based coding CLI"));
    assert!(stdout.contains("start"));
    assert!(stdout.contains("resume"));
    assert!(stdout.contains("config"));
}

#[test]
fn test_cli_version_output() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0"));
}

#[test]
fn test_config_init_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "init"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Should succeed or indicate config already exists
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Check for success message or already exists message
    assert!(
        stdout.contains("Configuration initialized") || 
        stdout.contains("already exists") ||
        stderr.contains("Configuration initialized") ||
        stderr.contains("already exists")
    );
}

#[test]
fn test_config_get_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // First initialize config
    let _ = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "init"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output();

    // Then get config
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "get"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[openrouter]"));
    assert!(stdout.contains("[preferences]"));
    assert!(stdout.contains("[tools]"));
}

#[test]
fn test_config_set_command() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // First initialize config
    let _ = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "init"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output();

    // Set a config value
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "set", "openrouter.model", "test-model"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("Updated") || stderr.contains("Updated"));
}

#[test]
fn test_config_set_invalid_key() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "set", "invalid.key", "value"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .output()
        .expect("Failed to execute command");

    // Should fail with invalid key
    assert!(!output.status.success());
}

#[test]
fn test_config_validate_without_api_key() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    // Initialize config without API key
    let _ = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "init"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env_remove("OPENROUTER_API_KEY")
        .output();

    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "validate"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("Failed to execute command");

    // Should fail validation without API key
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("API key not found") || 
        stderr.contains("API key not found") ||
        stdout.contains("Missing API key") ||
        stderr.contains("Missing API key")
    );
}

#[test]
fn test_config_validate_with_env_api_key() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "validate"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env("OPENROUTER_API_KEY", "test-key")
        .output()
        .expect("Failed to execute command");

    // Should pass validation with env API key
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("Configuration is valid") || 
        stderr.contains("Configuration is valid")
    );
}

#[test]
fn test_resume_command_not_implemented() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "resume", "test-session"])
        .output()
        .expect("Failed to execute command");

    // Should fail with not implemented message
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stdout.contains("not yet implemented") || 
        stderr.contains("not yet implemented")
    );
}

#[test]
fn test_start_command_without_api_key() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "start", "test project"])
        .env("XDG_CONFIG_HOME", temp_dir.path())
        .env("APPDATA", temp_dir.path())
        .env_remove("OPENROUTER_API_KEY")
        .output()
        .expect("Failed to execute command");

    // Should fail without API key
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("API key not found") || stderr.contains("not found"));
}

#[test]
fn test_loo_binary_works() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LLM-based coding CLI"));
}

#[test]
fn test_global_flags() {
    // Test --verbose flag
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "--verbose", "config", "get"])
        .env("OPENROUTER_API_KEY", "test-key")
        .output()
        .expect("Failed to execute command");

    // Should not crash with verbose flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Verbose flag should work (may not show different output in this context)
    assert!(!stderr.contains("error: ") || !stderr.contains("unrecognized"));

    // Test --model flag
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "--model", "test-model", "config", "validate"])
        .env("OPENROUTER_API_KEY", "test-key")
        .output()
        .expect("Failed to execute command");

    // Should not crash with model flag
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("error: ") || !stderr.contains("unrecognized"));
}

#[test]
fn test_invalid_command() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "invalid-command"])
        .output()
        .expect("Failed to execute command");

    // Should fail with invalid command
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unrecognized subcommand") || 
        stderr.contains("invalid") ||
        stderr.contains("help")
    );
}

#[test]
fn test_config_subcommand_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "config", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Manage LOO CLI configuration"));
    assert!(stdout.contains("init"));
    assert!(stdout.contains("get"));
    assert!(stdout.contains("set"));
    assert!(stdout.contains("validate"));
}

#[test]
fn test_start_subcommand_help() {
    let output = Command::new("cargo")
        .args(&["run", "--bin", "loo", "--", "start", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start a new coding project"));
    assert!(stdout.contains("--dir"));
}

#[test]
fn test_working_directory_flag() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let work_dir = temp_dir.path().join("workdir");
    fs::create_dir_all(&work_dir).expect("Failed to create work dir");
    
    let output = Command::new("cargo")
        .args(&[
            "run", "--bin", "loo", "--", 
            "start", 
            "--dir", work_dir.to_str().unwrap(),
            "test project"
        ])
        .env("OPENROUTER_API_KEY", "test-key")
        .output()
        .expect("Failed to execute command");

    // Should attempt to start with specified directory
    // Will fail due to network, but should not fail on directory parsing
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains("No such file or directory") && !stderr.contains("cannot find"));
}