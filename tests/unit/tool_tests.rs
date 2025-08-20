use loo_cli::tools::ToolExecutor;
use loo_cli::openrouter::{ToolCall, ToolCallFunction};
use serde_json::{json, Value};
use tempfile::TempDir;
use std::fs;

fn create_test_tool_call(function_name: &str, arguments: Value) -> ToolCall {
    ToolCall {
        id: "test_call".to_string(),
        call_type: "function".to_string(),
        function: ToolCallFunction {
            name: function_name.to_string(),
            arguments: arguments.to_string(),
        },
    }
}

#[tokio::test]
async fn test_create_file_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("create_file", json!({
        "path": "test.txt",
        "content": "Hello, World!"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "test.txt");
    assert_eq!(result_json["size"], 13);
    
    // Verify file was actually created
    let file_path = temp_dir.path().join("test.txt");
    assert!(file_path.exists());
    
    let content = fs::read_to_string(file_path)?;
    assert_eq!(content, "Hello, World!");
    
    Ok(())
}

#[tokio::test]
async fn test_create_file_with_subdirectory() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("create_file", json!({
        "path": "subdir/nested/test.txt",
        "content": "Nested file content"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    
    // Verify file and directories were created
    let file_path = temp_dir.path().join("subdir/nested/test.txt");
    assert!(file_path.exists());
    
    let content = fs::read_to_string(file_path)?;
    assert_eq!(content, "Nested file content");
    
    Ok(())
}

#[tokio::test]
async fn test_read_file_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create a test file first
    let test_content = "This is test content\nWith multiple lines";
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, test_content)?;
    
    let tool_call = create_test_tool_call("read_file", json!({
        "path": "test.txt"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "test.txt");
    assert_eq!(result_json["content"], test_content);
    assert_eq!(result_json["size"], test_content.len());
    
    Ok(())
}

#[tokio::test]
async fn test_read_nonexistent_file() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("read_file", json!({
        "path": "nonexistent.txt"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let _result_json: Value = serde_json::from_str(&result)?;
    
    // Should return an error but still be a valid JSON response
    assert!(result.contains("error") || result.contains("No such file"));
    
    Ok(())
}

#[tokio::test]
async fn test_write_file_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create initial file
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Original content")?;
    
    let tool_call = create_test_tool_call("write_file", json!({
        "path": "test.txt",
        "content": "Updated content"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "test.txt");
    assert_eq!(result_json["size"], "Updated content".len());
    
    // Verify file was updated
    let content = fs::read_to_string(file_path)?;
    assert_eq!(content, "Updated content");
    
    Ok(())
}

#[tokio::test]
async fn test_delete_file_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create a test file
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "Content to be deleted")?;
    assert!(file_path.exists());
    
    let tool_call = create_test_tool_call("delete_file", json!({
        "path": "test.txt"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "test.txt");
    assert_eq!(result_json["deleted"], true);
    
    // Verify file was deleted
    assert!(!file_path.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_create_directory_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("create_directory", json!({
        "path": "new/nested/directory"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "new/nested/directory");
    assert_eq!(result_json["created"], true);
    
    // Verify directory was created
    let dir_path = temp_dir.path().join("new/nested/directory");
    assert!(dir_path.exists());
    assert!(dir_path.is_dir());
    
    Ok(())
}

#[tokio::test]
async fn test_list_directory_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create some test files and directories
    fs::write(temp_dir.path().join("file1.txt"), "content1")?;
    fs::write(temp_dir.path().join("file2.txt"), "content2")?;
    fs::create_dir(temp_dir.path().join("subdir"))?;
    
    let tool_call = create_test_tool_call("list_directory", json!({
        "path": "."
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], ".");
    
    let entries = result_json["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3);
    
    // Check that we have the expected files and directories
    let entry_names: Vec<String> = entries
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    
    assert!(entry_names.contains(&"file1.txt".to_string()));
    assert!(entry_names.contains(&"file2.txt".to_string()));
    assert!(entry_names.contains(&"subdir".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_run_command_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("run_command", json!({
        "command": "echo 'Hello from command'"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["command"], "echo 'Hello from command'");
    assert!(result_json["stdout"].as_str().unwrap().contains("Hello from command"));
    assert_eq!(result_json["success"], true);
    assert_eq!(result_json["exit_code"], 0);
    
    Ok(())
}

#[tokio::test]
async fn test_run_command_with_error() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("run_command", json!({
        "command": "ls /nonexistent/directory"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "warning"); // Non-zero exit codes are warnings
    assert_eq!(result_json["success"], false);
    assert_ne!(result_json["exit_code"], 0);
    assert!(!result_json["stderr"].as_str().unwrap().is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_query_context_full() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create some test files
    fs::write(temp_dir.path().join("test1.txt"), "content1")?;
    fs::write(temp_dir.path().join("test2.txt"), "content2")?;
    fs::create_dir(temp_dir.path().join(".git"))?; // Simulate git repo
    
    let tool_call = create_test_tool_call("query_context", json!({
        "type": "full"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    
    let context = &result_json["context"];
    assert!(context["is_git_repo"].as_bool().unwrap());
    assert_eq!(context["working_directory"], *temp_dir.path().to_string_lossy());
    
    let dir_listing = context["directory_listing"].as_array().unwrap();
    let file_names: Vec<String> = dir_listing
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    
    assert!(file_names.contains(&"test1.txt".to_string()));
    assert!(file_names.contains(&"test2.txt".to_string()));
    assert!(file_names.contains(&".git".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_query_context_directory() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Create subdirectory with files
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir)?;
    fs::write(subdir.join("file1.txt"), "content1")?;
    fs::write(subdir.join("file2.txt"), "content2")?;
    
    let tool_call = create_test_tool_call("query_context", json!({
        "type": "directory",
        "path": "subdir"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    assert_eq!(result_json["path"], "subdir");
    
    let files = result_json["files"].as_array().unwrap();
    let file_names: Vec<String> = files
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    
    assert!(file_names.contains(&"file1.txt".to_string()));
    assert!(file_names.contains(&"file2.txt".to_string()));
    
    Ok(())
}

#[tokio::test]
async fn test_complete_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("complete", json!({}));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "completed");
    assert!(result_json["message"].as_str().unwrap().contains("complete"));
    
    Ok(())
}

#[tokio::test]
async fn test_unknown_tool() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    let tool_call = create_test_tool_call("unknown_tool", json!({}));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "error");
    assert!(result_json["message"].as_str().unwrap().contains("Unknown tool"));
    
    Ok(())
}

#[tokio::test]
async fn test_tool_with_missing_parameters() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), false);
    
    // Test create_file without required path parameter
    let tool_call = create_test_tool_call("create_file", json!({
        "content": "some content"
        // missing "path"
    }));
    
    let result = executor.execute_tool_call(&tool_call).await?;
    
    // Should return error but still be valid JSON
    assert!(result.contains("Missing 'path' parameter") || result.contains("error"));
    
    Ok(())
}

#[tokio::test]
async fn test_verbose_mode() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let executor = ToolExecutor::new(temp_dir.path().to_string_lossy().to_string(), true); // verbose=true
    
    let tool_call = create_test_tool_call("create_file", json!({
        "path": "test.txt",
        "content": "Hello, World!"
    }));
    
    // In verbose mode, the executor would print debug information
    // Since we can't easily capture stdout in this test, we just verify
    // that the tool still works correctly with verbose mode enabled
    let result = executor.execute_tool_call(&tool_call).await?;
    let result_json: Value = serde_json::from_str(&result)?;
    
    assert_eq!(result_json["status"], "success");
    
    Ok(())
}