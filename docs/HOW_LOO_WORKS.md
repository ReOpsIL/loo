# How LOO Works - Technical Documentation

LOO is an intelligent coding CLI that uses Large Language Models (LLMs) to provide semantic conversation-based development assistance. This document explains the current architecture and how the system operates.

## Core Architecture

LOO has evolved from a simple command-based system to a sophisticated semantic conversation engine. Here's how it works:

### Main Components

```
src/
├── main.rs                  # Entry point and semantic chat orchestration
├── semantic_engine.rs       # Semantic conversation engine (core intelligence)
├── llm_intent_recognition.rs# LLM-based intent recognition
├── openrouter/             # OpenRouter API integration
├── tools/                  # Tool execution system
├── config/                 # Configuration management
├── story/                  # Session logging and storytelling
├── execution_stack.rs      # Stack-based execution system
└── commands/               # Legacy command system (partially used)
```

## How LOO Operates

### 1. Semantic Conversation Mode

Instead of traditional command parsing, LOO uses **semantic understanding**:

- **Natural Language Processing**: Users can speak naturally instead of using specific commands
- **Intent Recognition**: LLM-powered intent analysis determines what the user wants to do
- **Contextual Awareness**: The system maintains conversation state and working memory
- **Adaptive Responses**: System behavior adapts based on conversation context

### 2. The SemanticEngine

The `SemanticEngine` is the heart of LOO's intelligence:

```rust
pub struct SemanticEngine {
    pub openrouter_client: OpenRouterClient,
    pub tool_executor: ToolExecutor,
    pub story_logger: StoryLogger,
    pub config: Config,
    pub working_dir: String,
    pub session_id: String,
    pub messages: Vec<Message>,
    pub context: ConversationContext,
}
```

#### Key Features:

- **ConversationContext**: Tracks conversation state, working memory, and available tools
- **Adaptive System Messages**: Creates different system prompts based on user intent
- **Memory Management**: Maintains relevant context while pruning old information
- **Tool Awareness**: Dynamically updates available tools based on conversation semantics

### 3. Conversation States

LOO recognizes different conversation modes and adapts accordingly:

- **Conversational**: General discussion and questions
- **Planning**: User needs help planning an approach
- **Implementing**: Active development and tool usage
- **Troubleshooting**: Debugging and problem-solving
- **Exploring**: Investigation and understanding existing code

### 4. LLM-Based Intent Recognition

Instead of command parsing, LOO uses an LLM to understand user intent:

```rust
pub enum UserIntent {
    ClearContext,
    ChangeModel(String),
    ListModels(Option<String>),
    RegularConversation(String),
}
```

This allows for natural expressions like:
- "clear our conversation" → ClearContext
- "switch to claude" → ChangeModel
- "show me available models" → ListModels

### 5. Tool System Integration

LOO integrates with a comprehensive tool system:

- **Filesystem Operations**: Create, read, write, delete files and directories
- **Command Execution**: Run shell commands with output capture
- **Context-Aware Tool Selection**: Tools are suggested based on conversation semantics

### 6. Session Management

Each LOO session includes:

- **Unique Session ID**: UUID-based session tracking
- **Story Logging**: Complete session recording in `story.md`
- **Working Directory Context**: Filesystem-aware operations
- **Configuration Persistence**: User preferences and API keys

## Workflow Example

Here's how a typical LOO interaction works:

1. **User Input**: "Help me build a web server in Rust"
2. **Intent Analysis**: LLM recognizes this as an implementation task
3. **Context Update**: ConversationState → Implementing
4. **Tool Selection**: Adds relevant tools (create_file, run_command, etc.)
5. **Adaptive System Message**: Creates implementation-focused system prompt
6. **LLM Processing**: Generates response with appropriate tool calls
7. **Tool Execution**: Executes file creation, cargo commands, etc.
8. **Memory Update**: Updates working memory with completion status
9. **Response**: Provides natural language feedback to user

## Key Innovations

### 1. Semantic Understanding Over Commands

Traditional CLI tools require users to learn specific commands. LOO understands natural language:

```bash
# Traditional approach
loo create-file server.rs
loo run-command "cargo run"

# LOO semantic approach
"Create a simple HTTP server and run it"
```

### 2. Context-Aware Tool Suggestions

Tools are dynamically suggested based on conversation:

- Mention "git" → git tools become available
- Mention "test" → testing tools are suggested
- Mention "database" → database tools are added

### 3. Intelligent Memory Management

LOO maintains three types of memory:

- **Important Messages**: Key conversation turns
- **Working Memory**: Recent task completions and insights
- **Current Thread**: Main topic/focus of conversation

### 4. Adaptive System Behavior

The system prompt changes based on user intent:

- Planning mode: Asks clarifying questions, suggests steps
- Implementation mode: Proactively uses tools
- Troubleshooting mode: Investigates and debugs
- Exploration mode: Explains and investigates

## Technical Implementation

### LLM Integration

LOO uses OpenRouter for LLM access, supporting:

- **Multiple Models**: Anthropic Claude, OpenAI GPT, Meta Llama, etc.
- **Function Calling**: Native tool integration through OpenAI-compatible function calling
- **Streaming**: Real-time response processing
- **Error Handling**: Robust error recovery and fallbacks

### Tool Execution

Tools are executed through the `ToolExecutor` with:

- **Sandboxed Execution**: Operations confined to working directory
- **Result Capture**: Complete stdout/stderr capture
- **Error Handling**: Graceful failure handling
- **Logging**: Complete tool execution logging

### Configuration System

LOO uses a hierarchical configuration system:

1. **Config Files**: `~/.config/loo/config.toml`
2. **Environment Variables**: `OPENROUTER_API_KEY`, etc.
3. **CLI Arguments**: Runtime overrides

## Session Story Generation

Every LOO session generates a comprehensive story in `story.md` including:

- **User Prompts**: Original user inputs
- **Assistant Responses**: AI responses and reasoning
- **Tool Executions**: Complete tool calls and results
- **Session Metadata**: Timestamps, session ID, working directory

## Future Architecture

LOO's architecture supports several planned enhancements:

- **Plugin System**: Custom tool integration
- **Project Templates**: Intelligent project scaffolding
- **Multi-Model Orchestration**: Different models for different tasks
- **Advanced Memory**: Vector-based semantic memory
- **Collaborative Features**: Multi-user sessions

## Summary

LOO represents a paradigm shift from command-based to conversation-based development tools. By using LLM-powered semantic understanding, it provides a more natural and intelligent development experience while maintaining the power and flexibility of traditional CLI tools.

The system's modular architecture, adaptive behavior, and comprehensive tool integration make it a powerful platform for AI-assisted development workflows.