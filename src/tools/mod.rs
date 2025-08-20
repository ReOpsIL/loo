use crate::openrouter::ToolCall;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use tokio::signal;
use tokio::process::Command as TokioCommand;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct ToolExecutor {
    working_dir: String,
    verbose: bool,
}

impl ToolExecutor {
    pub fn new(working_dir: String, verbose: bool) -> Self {
        Self { working_dir, verbose }
    }

    pub async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let args: Value = serde_json::from_str(&tool_call.function.arguments)?;
        
        if self.verbose {
            println!("  🔧 Executing: {}", tool_call.function.name);
            println!("     Args: {}", tool_call.function.arguments);
        }

        match tool_call.function.name.as_str() {
            "create_file" => self.handle_create_file(&args),
            "read_file" => self.handle_read_file(&args),
            "write_file" => self.handle_write_file(&args),
            "delete_file" => self.handle_delete_file(&args),
            "create_directory" => self.handle_create_directory(&args),
            "list_directory" => self.handle_list_directory(&args),
            "run_command" => self.handle_run_command(&args).await,
            "query_context" => self.handle_query_context(&args),
            "complete" => self.handle_complete(),
            _ => Ok(json!({"status": "error", "message": format!("Unknown tool: {}", tool_call.function.name)}).to_string()),
        }
    }

    fn handle_create_file(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().ok_or("Missing 'path' parameter")?;
        let content = args["content"].as_str().unwrap_or("");
        let full_path = Path::new(&self.working_dir).join(path);

        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&full_path, content)?;
        Ok(json!({
            "status": "success",
            "path": path,
            "size": content.len(),
            "absolute_path": full_path.to_string_lossy()
        }).to_string())
    }

    fn handle_read_file(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().ok_or("Missing 'path' parameter")?;
        let full_path = Path::new(&self.working_dir).join(path);
        let content = fs::read_to_string(&full_path)?;
        
        Ok(json!({
            "status": "success",
            "path": path,
            "content": content,
            "size": content.len(),
            "absolute_path": full_path.to_string_lossy()
        }).to_string())
    }

    fn handle_write_file(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().ok_or("Missing 'path' parameter")?;
        let content = args["content"].as_str().unwrap_or("");
        let full_path = Path::new(&self.working_dir).join(path);

        fs::write(&full_path, content)?;
        Ok(json!({
            "status": "success",
            "path": path,
            "size": content.len(),
            "absolute_path": full_path.to_string_lossy()
        }).to_string())
    }

    fn handle_delete_file(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().ok_or("Missing 'path' parameter")?;
        let full_path = Path::new(&self.working_dir).join(path);

        fs::remove_file(&full_path)?;
        Ok(json!({
            "status": "success",
            "path": path,
            "absolute_path": full_path.to_string_lossy(),
            "deleted": true
        }).to_string())
    }

    fn handle_create_directory(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().ok_or("Missing 'path' parameter")?;
        let full_path = Path::new(&self.working_dir).join(path);

        fs::create_dir_all(&full_path)?;
        Ok(json!({
            "status": "success",
            "path": path,
            "absolute_path": full_path.to_string_lossy(),
            "created": true
        }).to_string())
    }

    fn handle_list_directory(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let path = args["path"].as_str().unwrap_or(".");
        let full_path = Path::new(&self.working_dir).join(path);

        let entries = fs::read_dir(&full_path)?;
        let files: Result<Vec<_>, _> = entries
            .map(|entry| {
                entry.map(|e| {
                    let metadata = e.metadata().ok();
                    json!({
                        "name": e.file_name().to_string_lossy(),
                        "is_dir": metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                        "size": metadata.as_ref().and_then(|m| if m.is_file() { Some(m.len()) } else { None }),
                    })
                })
            })
            .collect();

        let file_list = files?;
        Ok(json!({
            "status": "success",
            "path": path,
            "absolute_path": full_path.to_string_lossy(),
            "entries": file_list
        }).to_string())
    }

    async fn handle_run_command(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let command = args["command"].as_str().ok_or("Missing 'command' parameter")?;
        
        println!("  🚀 Running: {} (Press Ctrl+C to interrupt)", command);
        
        let mut child = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()?;

        let mut stdout_output = String::new();
        let mut stderr_output = String::new();
        let mut interrupted = false;

        // Set up Ctrl+C handling
        tokio::select! {
            result = child.wait() => {
                match result {
                    Ok(status) => {
                        // Read any remaining output
                        if let Some(stdout) = child.stdout.take() {
                            let mut reader = BufReader::new(stdout);
                            let mut line = String::new();
                            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                                stdout_output.push_str(&line);
                                if self.verbose {
                                    print!("    {}", line);
                                }
                                line.clear();
                            }
                        }
                        
                        if let Some(stderr) = child.stderr.take() {
                            let mut reader = BufReader::new(stderr);
                            let mut line = String::new();
                            while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                                stderr_output.push_str(&line);
                                if self.verbose {
                                    eprint!("    {}", line);
                                }
                                line.clear();
                            }
                        }

                        let success = status.success();
                        let result = json!({
                            "status": if success { "success" } else { "warning" },
                            "command": command,
                            "stdout": stdout_output,
                            "stderr": stderr_output,
                            "exit_code": status.code(),
                            "success": success,
                            "interrupted": false
                        });

                        Ok(result.to_string())
                    }
                    Err(e) => Err(format!("Failed to wait for command: {}", e).into())
                }
            }
            _ = signal::ctrl_c() => {
                println!("  ⚠️  Ctrl+C detected, terminating process...");
                interrupted = true;
                
                // Kill the child process
                let _ = child.kill().await;
                
                let result = json!({
                    "status": "interrupted",
                    "command": command,
                    "stdout": stdout_output,
                    "stderr": stderr_output,
                    "exit_code": null,
                    "success": false,
                    "interrupted": true,
                    "message": "Process was interrupted by user (Ctrl+C)"
                });

                Ok(result.to_string())
            }
        }
    }

    fn handle_query_context(&self, args: &Value) -> Result<String, Box<dyn std::error::Error>> {
        let query_type = args["type"].as_str().unwrap_or("full");

        match query_type {
            "full" => {
                let mut context = serde_json::Map::new();
                
                // Get directory listing
                if let Ok(entries) = fs::read_dir(&self.working_dir) {
                    let files: Vec<String> = entries
                        .filter_map(|entry| {
                            entry.ok().and_then(|e| {
                                e.file_name().to_str().map(|s| s.to_string())
                            })
                        })
                        .collect();
                    context.insert("directory_listing".to_string(), json!(files));
                }

                // Check if it's a git repo
                let is_git = Path::new(&self.working_dir).join(".git").exists();
                context.insert("is_git_repo".to_string(), json!(is_git));

                // Get current working directory absolute path
                context.insert("working_directory".to_string(), json!(self.working_dir));

                Ok(json!({
                    "status": "success",
                    "context": serde_json::Value::Object(context)
                }).to_string())
            }
            "directory" => {
                let path = args["path"].as_str().unwrap_or(".");
                let full_path = Path::new(&self.working_dir).join(path);
                
                if let Ok(entries) = fs::read_dir(&full_path) {
                    let files: Vec<String> = entries
                        .filter_map(|entry| {
                            entry.ok().and_then(|e| {
                                e.file_name().to_str().map(|s| s.to_string())
                            })
                        })
                        .collect();
                    
                    Ok(json!({
                        "status": "success",
                        "path": path,
                        "files": files
                    }).to_string())
                } else {
                    Ok(json!({
                        "status": "error",
                        "message": format!("Could not read directory: {}", path)
                    }).to_string())
                }
            }
            _ => Ok(json!({
                "status": "error",
                "message": format!("Unknown query type: {}", query_type)
            }).to_string()),
        }
    }

    fn handle_complete(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(json!({
            "status": "completed",
            "message": "Project marked as complete"
        }).to_string())
    }
}