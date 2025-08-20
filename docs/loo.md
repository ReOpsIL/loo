# LLM-Based Coding CLI - LOO

## Overview
LOO is a Rust-based command-line tool that acts as a bridge between an LLM (serving as the central brain for reasoning and decision-making) and the filesystem/command-line tools ecosystem. The design eliminates hard-coded logic, ensuring the LLM dynamically designs plans, breaks down tasks into high-granularity procedures, generates content, reviews, debugs, and iterates via continuous feedback loops until project completion. The CLI provides no inherent intelligence—it's purely a facilitator for LLM instructions and tool executions.

## Design Principles
- **No Hard-Coded Logic**: All decision-making, planning, and task breakdown originate from the LLM. The CLI executes only what the LLM instructs.
- **LLM as Brain**: The LLM reasons, plans, and supervises every step, querying context as needed.
- **Filesystem and Tools as Execution End**: Use filesystem operations and command-line tools (e.g., Linux utilities) to perform actions like content generation, verification, and debugging.
- **High-Granularity Tasks**: LLM breaks down procedures into small, atomic tasks for precise control and easy feedback.
- **Continuous Feedback Loop**: After each task, provide detailed context (e.g., file contents, tool outputs) back to the LLM for review and next instructions.
- **Context Provisioning**: LLM can query filesystem state, tool results, or previous outputs at any time to maintain full perspective and avoid unknowns.
- **Simplicity and Modularity**: CLI components are minimal, focused on LLM communication, task execution, and feedback collection.

## Features
1. **Dynamic Planning**: LLM initiates by designing a high-level plan, then iteratively breaks it into granular tasks.
2. **Task Execution**: CLI runs LLM-specified tasks using filesystem (e.g., create/read/write files) and tools (e.g., run scripts, grep, diff).
3. **Content Generation**: LLM instructs the CLI to generate file content via tools or direct writes, then reviews outputs.
4. **Review and Debugging**: LLM receives feedback (e.g., file diffs, test results) and decides on fixes or iterations.
5. **Feedback and Querying**: CLI provides comprehensive snapshots of project state; LLM can request specific queries (e.g., "cat file.rs") for context.
6. **Iteration Until Completion**: Loop continues with LLM-driven instructions and feedback until the LLM deems the assignment complete.

## Architecture
The CLI consists of two primary ends connected via OpenRouter's LLM API with tool calling:
- **OpenRouter LLM Backend**: Uses OpenRouter REST API for LLM communication with native tool calling support.
- **Execution End**: Interfaces with filesystem and command-line tools via OpenRouter tool definitions.

Core components:
1. **OpenRouter Client**: Communicates with OpenRouter API using tool calling for filesystem and command operations.
2. **Tool Registry**: Exposes filesystem and command-line operations as OpenRouter-compatible tool definitions.
3. **Feedback System**: Collects and formats tool outputs for LLM consumption.
4. **Session Management**: Tracks conversation history and maintains context across interactions.

## Configuration
LOO uses TOML-based configuration with hierarchical precedence:

1. **Configuration File**: `~/.config/loo/config.toml` (Linux/macOS) or `%APPDATA%\loo\config.toml` (Windows)
2. **Environment Variables**: `OPENROUTER_API_KEY`, `OPENROUTER_MODEL` override file settings
3. **Command-line Arguments**: `--model`, `--verbose` flags override all other settings

### Sample Configuration
```toml
[openrouter]
api_key = "sk-or-v1-your-key-here"
model = "anthropic/claude-3.5-sonnet"
base_url = "https://openrouter.ai/api/v1"

[preferences]
verbose = false
auto_confirm = false
default_directory = "/home/user/projects"

[tools]
filesystem = true
commands = true
git = true
command_timeout = 300
```

## Tool System
LOO exposes these tools to the LLM via OpenRouter function calling:

### Filesystem Operations
- `create_file`: Create files with content
- `read_file`: Read file contents
- `write_file`: Update existing files
- `delete_file`: Remove files
- `create_directory`: Create directory structures
- `list_directory`: List directory contents

### Command Execution
- `run_command`: Execute shell commands with output capture
- `query_context`: Get project state and context information

### Project Management
- `complete`: Signal task completion
- Context queries for project introspection

## Usage Examples

### Basic Session
```bash
# Initialize configuration
loo config init

# Set OpenRouter API key
loo config set openrouter.api_key "sk-or-v1-your-key-here"

# Start a coding session
loo start "Create a REST API server in Rust with basic CRUD operations"
```

### Advanced Usage
```bash
# Use specific model and directory
loo start "Refactor the authentication module" \
  --model "anthropic/claude-3.5-sonnet" \
  --dir /path/to/project

# Enable verbose output for debugging
loo start "Debug failing tests" --verbose
```

### Configuration Management
```bash
loo config init                    # Initialize default configuration
loo config get                     # View current configuration
loo config validate               # Validate configuration
loo config set openrouter.model "meta-llama/llama-3.1-70b-instruct"
```

## Session Management
- **Session IDs**: Unique UUID generated for each session
- **State Persistence**: Working directory and conversation history maintained
- **Resume Support**: (Future feature) Resume interrupted sessions

## Error Handling
- **API Errors**: Graceful handling of OpenRouter API issues
- **Tool Failures**: Comprehensive error reporting and recovery
- **Configuration Issues**: Clear validation and setup guidance
- **Network Issues**: Retry logic and offline detection

## Development and Testing
LOO includes comprehensive testing infrastructure:
- **Unit Tests**: Configuration management and tool execution
- **Integration Tests**: CLI functionality and command interfaces
- **End-to-End Tests**: Mock OpenRouter server for realistic testing
- **Prompt-based Testing**: Real-world scenario validation

This architecture ensures LOO serves as a reliable bridge between LLM reasoning and practical development tasks, enabling sophisticated AI-driven coding workflows.