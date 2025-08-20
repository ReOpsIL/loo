use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc};
use serde_json::Value;

pub struct StoryLogger {
    working_dir: String,
    entries: Vec<StoryEntry>,
    session_id: String,
}

#[derive(Clone)]
pub struct StoryEntry {
    pub timestamp: DateTime<Utc>,
    pub entry_type: StoryEntryType,
    pub content: String,
}

#[derive(Clone)]
pub enum StoryEntryType {
    UserPrompt,
    AssistantResponse,
    ToolExecution { tool_name: String, args: Value },
    ToolResult { tool_name: String, success: bool, summary: String },
    ProcessInterrupted,
}

impl StoryLogger {
    pub fn new(working_dir: String, session_id: String) -> Self {
        Self {
            working_dir,
            entries: Vec::new(),
            session_id,
        }
    }

    pub fn log_user_prompt(&mut self, prompt: &str) {
        self.entries.push(StoryEntry {
            timestamp: Utc::now(),
            entry_type: StoryEntryType::UserPrompt,
            content: prompt.to_string(),
        });
    }

    pub fn log_assistant_response(&mut self, response: &str) {
        self.entries.push(StoryEntry {
            timestamp: Utc::now(),
            entry_type: StoryEntryType::AssistantResponse,
            content: response.to_string(),
        });
    }

    pub fn log_tool_execution(&mut self, tool_name: &str, args: &Value) {
        // Filter out file content from args for logging
        let filtered_args = self.filter_content_from_args(args.clone());
        
        self.entries.push(StoryEntry {
            timestamp: Utc::now(),
            entry_type: StoryEntryType::ToolExecution {
                tool_name: tool_name.to_string(),
                args: filtered_args,
            },
            content: String::new(),
        });
    }

    pub fn log_tool_result(&mut self, tool_name: &str, success: bool, result: &str) {
        let summary = self.create_result_summary(tool_name, result);
        
        self.entries.push(StoryEntry {
            timestamp: Utc::now(),
            entry_type: StoryEntryType::ToolResult {
                tool_name: tool_name.to_string(),
                success,
                summary,
            },
            content: String::new(),
        });
    }

    pub fn log_process_interrupted(&mut self) {
        self.entries.push(StoryEntry {
            timestamp: Utc::now(),
            entry_type: StoryEntryType::ProcessInterrupted,
            content: "Process was interrupted by user (Ctrl-C)".to_string(),
        });
    }

    fn filter_content_from_args(&self, mut args: Value) -> Value {
        if let Value::Object(ref mut map) = args {
            // Remove 'content' field if it exists
            if map.contains_key("content") {
                map.insert("content".to_string(), Value::String("[CONTENT_FILTERED]".to_string()));
            }
        }
        args
    }

    fn create_result_summary(&self, tool_name: &str, result: &str) -> String {
        match tool_name {
            "create_file" | "write_file" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let (Some(path), Some(size)) = (json["path"].as_str(), json["size"].as_u64()) {
                        return format!("File {} ({} bytes)", path, size);
                    }
                }
                "File operation completed".to_string()
            },
            "read_file" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let (Some(path), Some(size)) = (json["path"].as_str(), json["size"].as_u64()) {
                        return format!("Read file {} ({} bytes)", path, size);
                    }
                }
                "File read completed".to_string()
            },
            "delete_file" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let Some(path) = json["path"].as_str() {
                        return format!("Deleted file {}", path);
                    }
                }
                "File deleted".to_string()
            },
            "create_directory" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let Some(path) = json["path"].as_str() {
                        return format!("Created directory {}", path);
                    }
                }
                "Directory created".to_string()
            },
            "list_directory" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let (Some(path), Some(entries)) = (json["path"].as_str(), json["entries"].as_array()) {
                        return format!("Listed directory {} ({} items)", path, entries.len());
                    }
                }
                "Directory listed".to_string()
            },
            "run_command" => {
                if let Ok(json) = serde_json::from_str::<Value>(result) {
                    if let (Some(command), Some(success)) = (json["command"].as_str(), json["success"].as_bool()) {
                        let status = if success { "✓" } else { "✗" };
                        return format!("{} Command: {}", status, command);
                    }
                }
                "Command executed".to_string()
            },
            _ => format!("{} completed", tool_name)
        }
    }

    pub fn write_story_file(&self) -> Result<(), Box<dyn std::error::Error>> {
        let story_path = Path::new(&self.working_dir).join("story.md");
        let content = self.generate_markdown();
        fs::write(story_path, content)?;
        Ok(())
    }

    fn generate_markdown(&self) -> String {
        let mut markdown = String::new();
        
        // Header
        markdown.push_str(&format!("# LOO CLI Session Story\n\n"));
        markdown.push_str(&format!("**Session ID:** `{}`\n", self.session_id));
        markdown.push_str(&format!("**Working Directory:** `{}`\n", self.working_dir));
        markdown.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        markdown.push_str("---\n\n");

        // Entries
        for (i, entry) in self.entries.iter().enumerate() {
            let timestamp = entry.timestamp.format("%H:%M:%S");
            
            match &entry.entry_type {
                StoryEntryType::UserPrompt => {
                    markdown.push_str(&format!("## {} User Request\n", i + 1));
                    markdown.push_str(&format!("**Time:** {}\n\n", timestamp));
                    markdown.push_str(&format!("```\n{}\n```\n\n", entry.content));
                },
                StoryEntryType::AssistantResponse => {
                    markdown.push_str(&format!("### Assistant Response\n"));
                    markdown.push_str(&format!("**Time:** {}\n\n", timestamp));
                    markdown.push_str(&format!("{}\n\n", entry.content));
                },
                StoryEntryType::ToolExecution { tool_name, args } => {
                    markdown.push_str(&format!("### 🔧 Tool: `{}`\n", tool_name));
                    markdown.push_str(&format!("**Time:** {}\n\n", timestamp));
                    if args != &Value::Null {
                        markdown.push_str(&format!("**Arguments:**\n```json\n{}\n```\n\n", 
                            serde_json::to_string_pretty(args).unwrap_or_else(|_| "Invalid JSON".to_string())));
                    }
                },
                StoryEntryType::ToolResult { tool_name: _, success, summary } => {
                    let status_icon = if *success { "✅" } else { "❌" };
                    markdown.push_str(&format!("**Result:** {} {}\n\n", status_icon, summary));
                },
                StoryEntryType::ProcessInterrupted => {
                    markdown.push_str(&format!("### ⚠️ Process Interrupted\n"));
                    markdown.push_str(&format!("**Time:** {}\n\n", timestamp));
                    markdown.push_str(&format!("{}\n\n", entry.content));
                },
            }
        }

        markdown
    }
}