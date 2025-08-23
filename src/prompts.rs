/// Centralized prompt management to ensure consistency across all LLM interactions
pub struct PromptManager;

impl PromptManager {
    /// Unified system prompt that aligns with action_plan_prompt.md philosophy
    /// This should be used consistently across all LLM interactions
    pub fn get_system_prompt() -> String {
        "You are an expert coding assistant that creates detailed, step-by-step action plans for coding tasks. \
        You work with filesystem and command execution tools to complete user requests. \
        You always respond with valid JSON when JSON format is requested, and you follow the exact format specified in user prompts. \
        You create comprehensive plans that break down complex requests into specific, executable actions.".to_string()
    }

    /// Get the main action plan prompt template (embedded)
    pub fn get_action_plan_template() -> &'static str {
        r#"# LLM Action Plan Generation Prompt

## Core Instruction

You are a coding assistant cli application that needs to create a highly granulated plan for handling user request. When given a user request, you must create a comprehensive, step-by-step action plan that breaks down the user request into specific, executable actions. Each action should specify exactly which tools to use and what operations to perform.

## Plan Structure Requirements

Your plan must include:

### 1. **Analysis Phase**
- Understanding the request scope and requirements
- Identifying the project structure and technology stack
- Locating relevant files and dependencies
- Assessing current implementation state

### 2. **Discovery Actions**
For each discovery step, specify:
- **Tool**: Which tool to use (Read, Grep, Glob, LS, Bash, etc.)
- **Target**: Specific files, directories, or patterns to examine
- **Purpose**: What information you're seeking
- **Expected Output**: What you expect to find

### 3. **Implementation Actions**
For each implementation step, specify:
- **Tool**: Which tool to use (Edit, MultiEdit, Write, Bash, etc.)
- **Operation**: Exact changes to make
- **Files**: Specific file paths involved
- **Dependencies**: Prerequisites from previous steps
- **Validation**: How to verify the step succeeded

### 4. **Verification Phase**
- Testing procedures
- Quality checks (lint, typecheck, build)
- Integration verification
- Documentation updates

## Action Item Format

Each action item must follow this structure:

```
### Action [N]: [Brief Description]
**Tool**: [Tool Name]
**Target**: [Specific file/directory/pattern]
**Operation**: [Detailed description of what to do]
**Purpose**: [Why this step is necessary]
**Success Criteria**: [How to know it worked]
**Dependencies**: [Previous actions this depends on]
```

## Required Output Format

**IMPORTANT**: Your response MUST be in valid JSON format that can be parsed and processed programmatically. Use the following structure:

```json
{
  "title": "Brief task summary",
  "overview": "Detailed description of the overall task and approach",
  "phases": [
    {
      "name": "Analysis Phase",
      "emoji": "🔍",
      "actions": [
        {
          "id": 1,
          "title": "Action title",
          "tool": "Tool name",
          "target": "Specific file/directory/pattern",
          "operation": "Detailed description of what to do",
          "purpose": "Why this step is necessary",
          "success_criteria": "How to know it worked",
          "dependencies": [2, 3],
          "status": "pending"
        }
      ]
    },
    {
      "name": "Implementation Phase", 
      "emoji": "🛠️",
      "actions": [...]
    },
    {
      "name": "Verification Phase",
      "emoji": "✅", 
      "actions": [...]
    }
  ],
  "expected_outcome": "Description of the final state after all actions complete"
}
```

**Critical Requirements**:
- Response must be valid JSON only - no markdown, no explanations, no additional text
- All string values must be properly escaped for JSON
- Action IDs must be unique integers starting from 1
- Dependencies array contains action IDs that must complete first
- Status must be "pending" for all initial actions
- Include all phases: Analysis, Implementation, Verification (minimum)

## Quality Guidelines

1. **Specificity**: Each action must be specific enough to execute without ambiguity
2. **Atomicity**: Each action should accomplish one clear objective
3. **Dependency Management**: Clearly specify which actions depend on others
4. **Error Handling**: Consider what could go wrong and how to handle it
5. **Validation**: Include verification steps for critical changes

## Tool Selection Guide

- **Read**: For examining specific files
- **Grep**: For searching content across files
- **Glob**: For finding files by pattern
- **LS**: For directory exploration
- **Edit/MultiEdit**: For modifying existing files
- **Write**: For creating new files (use sparingly)
- **Bash**: For running commands, tests, builds
- **Task**: For complex multi-step operations

## Response Requirements

1. Always start with a clear overview
2. Number actions sequentially
3. Group related actions logically
4. Include emoji headers for visual organization
5. End with expected outcome summary
6. Ensure the plan is comprehensive but not overly verbose

Remember: The goal is to create a plan so detailed that any LLM could execute it step-by-step without additional decision-making."#
    }

    /// Create a user message for plan generation requests
    pub fn create_plan_user_message(user_request: &str) -> String {
        let base_prompt = Self::get_action_plan_template();
        
        format!(
            "{}\n\n---\n\n## User Request\n\n{}\n\nPlease create a detailed action plan for this request following the format specified above. Remember to respond with valid JSON only.",
            base_prompt,
            user_request
        )
    }

    /// Create a user message for task decomposition (aligned with plan approach)
    pub fn create_decomposition_user_message(
        task_description: &str, 
        working_dir: &str,
        context: Option<&str>
    ) -> String {
        let context_section = if let Some(ctx) = context {
            format!("\n\n## Context\n\n{}", ctx)
        } else {
            String::new()
        };

        format!(
            "## Task Analysis Request\n\n\
            Working directory: {}\n\n\
            ## Task Description\n\n\
            {}{}\n\n\
            ## Instructions\n\n\
            Please analyze this task and determine if it can be executed directly with available tools, \
            or if it needs to be broken down into smaller sub-tasks.\n\n\
            IMPORTANT: Respond with valid JSON only, following this exact schema:\n\
            {{\n\
              \"analysis\": \"Brief analysis of the task complexity and requirements\",\n\
              \"is_executable\": false,\n\
              \"executable_action\": null,\n\
              \"sub_tasks\": [\n\
                {{\n\
                  \"id\": \"unique-task-id\",\n\
                  \"description\": \"Specific sub-task description\",\n\
                  \"priority\": 1\n\
                }}\n\
              ],\n\
              \"reasoning\": \"Explanation of why this approach was chosen\"\n\
            }}\n\n\
            If the task IS directly executable, set is_executable to true and provide executable_action instead of sub_tasks.",
            working_dir,
            task_description,
            context_section
        )
    }

    /// Create a user message for direct execution requests (aligned with plan approach)
    pub fn create_execution_user_message(
        request: &str,
        working_dir: &str,
        context: Option<&str>
    ) -> String {
        let context_section = if let Some(ctx) = context {
            format!("\n\n## Context\n\n{}", ctx)
        } else {
            String::new()
        };

        format!(
            "## Direct Execution Request\n\n\
            Working directory: {}\n\n\
            ## Task to Execute\n\n\
            {}{}\n\n\
            ## Instructions\n\n\
            Please execute this task using the available tools. \
            You have access to filesystem operations, command execution, and code editing tools. \
            Be systematic and thorough in your approach.\n\n\
            Important:\n\
            - Use the Read tool to examine files before editing\n\
            - Use appropriate tools for each operation (Edit, MultiEdit, Write, Bash, etc.)\n\
            - Verify your changes after implementation\n\
            - Provide clear feedback about what you're doing",
            working_dir,
            request,
            context_section
        )
    }

    /// Create a user message for nested plan requests
    pub fn create_nested_plan_user_message(
        request: &str,
        working_dir: &str,
        depth: u8,
        parent_context: Option<&str>
    ) -> String {
        let context_section = if let Some(ctx) = parent_context {
            format!("\n\n## Parent Context\n\n{}", ctx)
        } else {
            String::new()
        };

        format!(
            "## Nested Planning Request (Depth: {})\n\n\
            Working directory: {}\n\n\
            ## Request to Plan\n\n\
            {}{}\n\n\
            ## Instructions\n\n\
            This is a nested planning request at depth {}. Please create a focused plan that:\n\
            - Is more specific than higher-level plans\n\
            - Contains concrete, actionable steps\n\
            - Can be executed directly or with minimal further decomposition\n\n\
            Since this is at depth {}, prefer direct execution over further decomposition when possible.\n\n\
            Respond with a clear plan or indicate if this should be executed directly.",
            depth,
            working_dir,
            request,
            context_section,
            depth,
            depth
        )
    }
}