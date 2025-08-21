use std::collections::HashMap;
use std::sync::OnceLock;

/// Result type for command execution
pub type CommandResult = Result<String, Box<dyn std::error::Error + Send + Sync>>;

/// Handler function type for special commands
pub type CommandHandler = fn(&str) -> CommandResult;

/// Command metadata
#[derive(Debug, Clone)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub handler: CommandHandler,
    pub needs_engine: bool,
}

/// Registry for special commands
#[derive(Debug)]
pub struct CommandRegistry {
    commands: HashMap<String, CommandInfo>,
}

impl CommandRegistry {
    fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    /// Register a new command
    pub fn register(&mut self, name: &str, description: &str, handler: CommandHandler, needs_engine: bool) {
        let command_info = CommandInfo {
            name: name.to_string(),
            description: description.to_string(),
            handler,
            needs_engine,
        };
        self.commands.insert(name.to_string(), command_info);
    }

    /// Get all registered commands
    pub fn get_all_commands(&self) -> Vec<&CommandInfo> {
        let mut commands: Vec<&CommandInfo> = self.commands.values().collect();
        commands.sort_by(|a, b| a.name.cmp(&b.name));
        commands
    }

    /// Get commands that match a prefix
    pub fn get_matching_commands(&self, prefix: &str) -> Vec<&CommandInfo> {
        let mut matching: Vec<&CommandInfo> = self.commands
            .values()
            .filter(|cmd| cmd.name.starts_with(prefix))
            .collect();
        matching.sort_by(|a, b| a.name.cmp(&b.name));
        matching
    }

    /// Execute a command by name
    pub fn execute_command(&self, command_name: &str, args: &str) -> Option<CommandResult> {
        self.commands
            .get(command_name)
            .map(|cmd_info| (cmd_info.handler)(args))
    }

    /// Check if a command exists
    pub fn has_command(&self, command_name: &str) -> bool {
        self.commands.contains_key(command_name)
    }

    /// Check if a command needs engine context
    pub fn command_needs_engine(&self, command_name: &str) -> bool {
        self.commands.get(command_name)
            .map(|cmd| cmd.needs_engine)
            .unwrap_or(false)
    }
}

/// Global command registry instance
static COMMAND_REGISTRY: OnceLock<std::sync::Mutex<CommandRegistry>> = OnceLock::new();

/// Initialize the global command registry
pub fn init_command_registry() {
    let registry = std::sync::Mutex::new(CommandRegistry::new());
    COMMAND_REGISTRY.set(registry).expect("Command registry should only be initialized once");
    
    // Register built-in commands
    register_builtin_commands();
}

/// Get access to the global command registry
pub fn with_registry<F, R>(f: F) -> R
where
    F: FnOnce(&CommandRegistry) -> R,
{
    let registry = COMMAND_REGISTRY
        .get()
        .expect("Command registry not initialized")
        .lock()
        .unwrap();
    f(&*registry)
}

/// Get mutable access to the global command registry
pub fn with_registry_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut CommandRegistry) -> R,
{
    let mut registry = COMMAND_REGISTRY
        .get()
        .expect("Command registry not initialized")
        .lock()
        .unwrap();
    f(&mut *registry)
}

/// Register a command in the global registry
pub fn register_command(name: &str, description: &str, handler: CommandHandler) {
    with_registry_mut(|registry| {
        registry.register(name, description, handler, false);
    });
}

/// Register an engine command in the global registry
pub fn register_engine_command(name: &str, description: &str, handler: CommandHandler) {
    with_registry_mut(|registry| {
        registry.register(name, description, handler, true);
    });
}

/// Check if a command needs engine context
pub fn command_needs_engine(command_name: &str) -> bool {
    with_registry(|registry| {
        registry.command_needs_engine(command_name)
    })
}

/// Execute a command from the global registry
pub fn execute_command(command_line: &str) -> Option<CommandResult> {
    let parts: Vec<&str> = command_line.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }
    
    let command_name = parts[0];
    let args = if parts.len() > 1 {
        parts[1..].join(" ")
    } else {
        String::new()
    };
    
    with_registry(|registry| {
        registry.execute_command(command_name, &args)
    })
}

/// Get all commands for autocomplete
pub fn get_autocomplete_commands(prefix: &str) -> Vec<String> {
    with_registry(|registry| {
        registry
            .get_matching_commands(prefix)
            .into_iter()
            .map(|cmd| cmd.name.clone())
            .collect()
    })
}

/// Get command descriptions for autocomplete display
pub fn get_command_descriptions() -> HashMap<String, String> {
    with_registry(|registry| {
        registry
            .get_all_commands()
            .into_iter()
            .map(|cmd| (cmd.name.clone(), cmd.description.clone()))
            .collect()
    })
}

// Built-in command handlers

fn handle_plan_command(args: &str) -> CommandResult {
    if args.trim().is_empty() {
        Err("Plan command requires a request description".into())
    } else {
        Err(format!("ENGINE_COMMAND:plan:{}", args.trim()).into())
    }
}

// Engine commands return a special result that indicates they need engine processing
fn handle_clear_command(_args: &str) -> CommandResult {
    Err("ENGINE_COMMAND:clear".into())
}

fn handle_model_command(args: &str) -> CommandResult {
    let new_model = args.trim();
    if new_model.is_empty() {
        Err("Usage: /model <model_name>\n💡 Tip: Use /list-models to see available models".into())
    } else {
        Err(format!("ENGINE_COMMAND:model:{}", new_model).into())
    }
}

fn handle_list_models_command(args: &str) -> CommandResult {
    let search_term = args.trim();
    Err(format!("ENGINE_COMMAND:list-models:{}", search_term).into())
}

/// Register all built-in commands
fn register_builtin_commands() {
    with_registry_mut(|registry| {
        // Register engine commands that need engine context
        registry.register("clear", "Clear conversation context", handle_clear_command, true);
        registry.register("model", "Change the current LLM model", handle_model_command, true);
        registry.register("list-models", "List all available LLM models", handle_list_models_command, true);
        
        // Register plan command that needs engine context  
        registry.register("plan", "Generate detailed action plan for coding tasks", handle_plan_command, true);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_registry() {
        let mut registry = CommandRegistry::new();
        
        // Register a test command
        registry.register("test", "Test command", |args| {
            Ok(format!("Test executed with args: {}", args))
        }, false);
        
        // Test getting commands
        let commands = registry.get_all_commands();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].name, "test");
        
        // Test execution
        let result = registry.execute_command("test", "hello");
        assert!(result.is_some());
        assert!(result.unwrap().is_ok());
    }

    #[test]
    fn test_command_filtering() {
        let mut registry = CommandRegistry::new();
        registry.register("list", "List items", |_| Ok("Listed".to_string()), false);
        registry.register("list-models", "List models", |_| Ok("Models listed".to_string()), true);
        registry.register("clear", "Clear screen", |_| Ok("Cleared".to_string()), true);
        
        let matching = registry.get_matching_commands("list");
        assert_eq!(matching.len(), 2);
        
        let matching = registry.get_matching_commands("list-");
        assert_eq!(matching.len(), 1);
        assert_eq!(matching[0].name, "list-models");
        
        // Test engine command detection
        assert!(!registry.command_needs_engine("list"));
        assert!(registry.command_needs_engine("list-models"));
        assert!(registry.command_needs_engine("clear"));
    }

    #[test]
    fn test_unified_command_system() {
        // This test requires the global registry to be initialized
        init_command_registry();
        
        // Test that plan command returns ENGINE_COMMAND marker
        let result = execute_command("plan Create a test");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("ENGINE_COMMAND:plan:Create a test"));
        
        // Test that engine commands return ENGINE_COMMAND markers
        let result = execute_command("clear");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("ENGINE_COMMAND:clear"));
        
        let result = execute_command("model gpt-4");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("ENGINE_COMMAND:model:gpt-4"));
        
        let result = execute_command("list-models search");
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().starts_with("ENGINE_COMMAND:list-models:search"));
        
        // Test unknown command
        let result = execute_command("unknown-command");
        assert!(result.is_none(), "Unknown command should return None");
    }

    #[test]
    fn test_command_engine_detection() {
        init_command_registry();
        
        // Test that plan is detected as an engine command
        assert!(command_needs_engine("plan"), "plan should need engine");
        assert!(command_needs_engine("clear"), "clear should need engine");
        assert!(command_needs_engine("model"), "model should need engine");
        assert!(command_needs_engine("list-models"), "list-models should need engine");
        assert!(!command_needs_engine("unknown-command"), "unknown-command should not need engine");
    }
}