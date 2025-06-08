---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Goose Agent Internals

## Table of Contents
- [Overview](#overview)
- [Core Agent Structure (`Agent` struct)](#core-agent-structure-agent-struct)
- [Agent Lifecycle and Initialization](#agent-lifecycle-and-initialization)
  - [Configuration (`Config`, `Profile`)](#configuration-config-profile)
  - [Provider Setup](#provider-setup)
  - [Extension Loading](#extension-loading)
- [The Core Agent Loop (Interaction Cycle)](#the-core-agent-loop-interaction-cycle)
  - [Receiving User Input](#receiving-user-input)
  - [Prompt Construction](#prompt-construction)
  - [Interaction with LLM Provider](#interaction-with-llm-provider)
  - [Tool Request Handling](#tool-request-handling)
- [Session Management](#session-management)
  - [Session ID and Storage](#session-id-and-storage)
  - [Message History](#message-history)
  - [Metadata](#metadata)
- [Extension Management (`ExtensionManager`)](#extension-management-extensionmanager)
  - [Loading Extensions](#loading-extensions)
  - [Tool Discovery from Extensions](#tool-discovery-from-extensions)
  - [MCP Client Interaction](#mcp-client-interaction)
  - [Resource Management](#resource-management)
- [Tool Handling](#tool-handling)
  - [Tool Definition (`Tool` struct)](#tool-definition-tool-struct)
  - [Tool Routing and Selection](#tool-routing-and-selection)
    - [Default Router](#default-router)
    - [Vector DB Tool Router (`tool_vectordb.rs`)](#vector-db-tool-router-tool_vectordbrs)
  - [Tool Execution Flow (`dispatch_tool_call`)](#tool-execution-flow-dispatch_tool_call)
    - [Platform Tools](#platform-tools)
    - [Frontend Tools](#frontend-tools)
    - [Extension (MCP) Tools](#extension-mcp-tools)
  - [Permissions (`PermissionManager`, `PermissionJudge`)](#permissions-permissionmanager-permissionjudge)
    - [Permission Levels](#permission-levels)
    - [Confirmation Flow](#confirmation-flow)
  - [Tool Output and Error Handling](#tool-output-and-error-handling)
  - [Tool Monitoring (`ToolMonitor`)](#tool-monitoring-toolmonitor)
- [Context Management (`context.rs`)](#context-management-contextrs)
  - [Building Context for LLM](#building-context-for-llm)
  - [Token Counting (`TokenCounter`)](#token-counting-tokencounter)
  - [Truncation Strategies](#truncation-strategies)
  - [Summarization Strategies](#summarization-strategies)
- [Multi-Model Usage](#multi-model-usage)
  - [Lead/Worker Pattern](#leadworker-pattern)
  - [Planner Model](#planner-model)
  - [Tool Shim Model](#tool-shim-model)
- [Prompt Management (`PromptManager`)](#prompt-management-promptmanager)
  - [System Prompts](#system-prompts)
  - [Extension-Provided Prompts](#extension-provided-prompts)
  - [Dynamic Prompt Construction](#dynamic-prompt-construction)
- [Notifier System](#notifier-system)
- [Frontend Interaction (Desktop UI / CLI)](#frontend-interaction-desktop-ui--cli)

## Overview
This document delves into the internal workings of the `codename goose` agent. The core logic resides primarily within the `goose` crate, specifically in the `crates/goose/src/agents/` directory. The `Agent` struct is central to orchestrating tasks, managing state, and interacting with various subsystems like LLM providers, extensions, and the user interface.

## Core Agent Structure (`Agent` struct)
The main `Agent` struct (in `crates/goose/src/agents/agent.rs`) holds the state and logic for an agent instance. Its key fields include:
- `provider`: A mutex-wrapped `Option<Arc<dyn Provider>>` for the current LLM provider.
- `extension_manager`: A mutex-wrapped `ExtensionManager` to manage all loaded extensions (tools, prompts, MCP clients).
- `frontend_tools`: A map of tools provided directly by a frontend (like the Desktop UI).
- `frontend_instructions`: Optional instructions related to frontend tools.
- `prompt_manager`: Manages system prompts and extension-specific prompts.
- `confirmation_tx`, `confirmation_rx`: Channels for handling permission confirmations from the user.
- `tool_result_tx`, `tool_result_rx`: Channels for receiving results from asynchronously executed frontend tools.
- `tool_monitor`: Optional `ToolMonitor` for tracking tool call repetitions.
- `router_tool_selector`: Optional component for advanced tool routing strategies (e.g., vector-based search).

## Agent Lifecycle and Initialization

### Configuration (`Config`, `Profile`)
- Agent behavior is heavily influenced by configuration loaded via the `Config` struct (from `crates/goose/src/config/base.rs`).
- `Config::global()` provides a singleton instance, typically loading from `~/.config/goose/config.yaml`.
- This configuration includes:
    - LLM provider settings (API keys, model choices).
    - Enabled extensions and their specific configurations.
    - Tool permissions.
    - Environment variables (e.g., `GOOSE_LEAD_MODEL`, `GOOSE_ROUTER_TOOL_SELECTION_STRATEGY`).
- The concept of a "Profile" (mentioned in root `ARCHITECTURE.md`) encapsulates a specific set of configurations for an agent instance, including which extensions to load and which models to use.

### Provider Setup
- The `Agent::update_provider()` method sets or changes the LLM provider.
- This also triggers an update to the `router_tool_selector` if a vector-based strategy is configured, as embeddings might be provider-specific.

### Extension Loading
- Extensions are loaded by the `ExtensionManager` based on the `config.yaml`.
- `Agent::add_extension()` handles adding new extensions dynamically. This involves:
    - For frontend extensions: Storing their tool definitions and instructions directly in the `Agent`.
    - For other extensions (builtin, stdio, sse): Delegating to `ExtensionManager::add_extension()`.
    - If vector tool routing is enabled, tools from the new extension are indexed via `ToolRouterIndexManager::update_extension_tools()`.

## The Core Agent Loop (Interaction Cycle)
The primary interaction logic is within `Agent::reply()`. This method processes a sequence of messages and yields `AgentEvent`s (either new messages or MCP notifications).

### Receiving User Input
- `Agent::reply()` takes a slice of `Message`s, representing the conversation history.
- The last message is typically the new user input.

### Prompt Construction
1. **System Prompt**: `PromptManager::build_system_prompt()` assembles the main system prompt. This incorporates:
   - Base system instructions.
   - Information from all loaded extensions (names, instructions, resource capabilities) via `ExtensionManager::get_extensions_info()`.
   - Instructions for frontend-provided tools.
   - Prompts related to suggesting disabling extensions if many are active.
   - Current LLM model name.
   - Any dynamically added "extra" system prompt lines.
2. **User Messages**: The conversation history is passed to the LLM.

### Interaction with LLM Provider
- `Agent::generate_response_from_provider()` calls the configured LLM provider's `complete()` method with the system prompt, message history, and available tools.
- The provider returns the LLM's response (which may include text and tool call requests) and token usage information.
- Handles `ProviderError::ContextLengthExceeded` by yielding a special message and terminating.

### Tool Request Handling
1.  **Categorization**: Assistant's response is parsed. Tool requests are categorized into `frontend_requests` and `remaining_requests`.
2.  **Tool Call Recording**: If a `router_tool_selector` is active, all tool names from requests are recorded.
3.  **Frontend Tools**: `Agent::handle_frontend_tool_requests()` processes tools designated as frontend-executable.
    - It yields a `Message::assistant().with_frontend_tool_request(...)`.
    - Waits for the result via the `tool_result_rx` channel (populated by `Agent::handle_tool_result()`).
4.  **Backend Tools (Chat Mode vs. Auto/Approve Modes)**:
    - If `GOOSE_MODE` is "chat", all backend tool calls are skipped, and a message explaining this is prepared.
    - Otherwise (auto, approve, smart_approve modes):
        - **Permission Check**: `check_tool_permissions()` (from `crates/goose/src/permission/permission_judge.rs`) categorizes tools into `approved`, `denied`, or `needs_approval` based on `config.yaml` permissions, `GOOSE_MODE`, and tool annotations (read-only).
        - **Approved Tools**: Dispatched immediately and concurrently.
        - **Denied Tools**: A standard "declined" response is prepared.
        - **Needs Approval Tools**: `Agent::handle_approval_tool_requests()` processes these.
            - Yields a `Message::user().with_tool_confirmation_request(...)`.
            - Waits for user confirmation via `confirmation_rx` channel (populated by `Agent::handle_confirmation()`).
            - If approved, dispatches the tool. If denied, prepares a "declined" response.
            - Updates `PermissionManager` if "AlwaysAllow" is chosen.
5.  **Tool Dispatch**: `Agent::dispatch_tool_call()` sends the tool call to the appropriate handler (platform tool, or `ExtensionManager` for extension tools).
6.  **Result Aggregation**: Results from all dispatched tools (frontend and backend) are collected into a single `Message::user()` with multiple tool responses.
7.  This tool response message is then yielded and appended to the conversation history for the next LLM turn.

## Session Management
- Session handling logic is partly in `crates/goose/src/session/`.
- **Session ID**: Typically a timestamp-based unique identifier (e.g., `generate_session_id()`).
- **Storage**: Session history (messages) and metadata are stored in files, usually under `~/.local/share/goose/sessions/` (path from XDG strategy). Each session is a `.jsonl` file.
- **Message History**: Messages are appended to the session file. `read_messages()` loads them.
- **Metadata (`SessionMetadata`)**: Stored as the first line in the session file. Includes working directory, description, message count, and token counts. `read_metadata()` and `update_metadata()` handle this. `Agent::update_session_metrics()` updates token usage after an LLM call.

## Extension Management (`ExtensionManager`)
Located in `crates/goose/src/agents/extension_manager.rs`.
- **State**: Holds a map of active MCP clients (`clients: HashMap<String, Arc<Mutex<Box<dyn McpClientTrait>>>>`), their instructions, and a set of resource-capable extensions.
- **Loading Extensions**:
    - `ExtensionManager::add_extension()` takes an `ExtensionConfig`.
    - For `Stdio` or `Sse` types, it creates the respective transport (`StdioTransport`, `SseTransport`), starts it, and connects an `McpClient`.
    - For `Builtin` types, it constructs a `StdioTransport` to call the main `goose` binary with "mcp <extension_name>" arguments.
    - It initializes the client with `client.initialize()`, storing any returned instructions and noting resource capabilities.
- **Tool Discovery**: `ExtensionManager::get_prefixed_tools()` lists tools from all (or a specific) managed clients. Tool names are prefixed with the sanitized extension name (e.g., `myextension__sometool`).
- **MCP Client Interaction**:
    - `dispatch_tool_call()`: Finds the correct client based on the tool name prefix and calls its `call_tool()` method.
    - `list_prompts()`, `get_prompt()`: Interact with clients to get extension-defined prompts.
- **Resource Management**:
    - `get_resources()`: Iterates resource-capable extensions, calls `list_resources()` and `read_resource()` on their clients to fetch active resources.
    - `read_resource_from_extension()`, `list_resources_from_extension()`: Handle resource operations for a specific extension.

## Tool Handling

### Tool Definition (`Tool` struct)
- Defined in `mcp-core`. Includes `name`, `description`, `inputSchema` (JSON schema), and `annotations` (e.g., `readOnlyHint`, `destructiveHint`).

### Tool Routing and Selection
- **Default Router**: The LLM itself acts as the primary tool router, by choosing from the list of all available tools provided in the prompt.
- **Vector DB Tool Router (`crates/goose/src/agents/tool_vectordb.rs`)**:
    - Enabled by `GOOSE_ROUTER_TOOL_SELECTION_STRATEGY="vector"`.
    - Uses LanceDB to store embeddings of tool names, descriptions, and schemas (`ToolRecord`).
    - `Agent::update_router_tool_selector()` initializes `ToolVectorDB` and `RouterToolSelector` (e.g., `VectorRouterToolSelector`).
    - `ToolRouterIndexManager` handles indexing:
        - `index_platform_tools()`: Indexes built-in platform tools.
        - `update_extension_tools()`: Indexes/removes tools when extensions are added/removed.
    - When active, a special `vector_search_tool` (from `router_tools.rs`) is made available to the LLM. The LLM can call this tool with a search query.
    - `VectorRouterToolSelector::select_tools()` then uses this query to search LanceDB and returns a list of relevant tools to the LLM, which can then choose one of those more specific tools.
    - The selector also tracks `get_recent_tool_calls()` to potentially boost recently used tools.

### Tool Execution Flow (`dispatch_tool_call` in `Agent`)
1. **Repetition Check**: If `ToolMonitor` is active, it checks if the tool call exceeds allowed repetitions.
2. **Platform Tools**: Certain tools are handled directly by the `Agent`:
   - `platform_manage_extensions`: Calls `Agent::manage_extensions()`.
   - `platform_read_resource`: Calls `ExtensionManager::read_resource()`.
   - `platform_list_resources`: Calls `ExtensionManager::list_resources()`.
   - `platform_search_available_extensions`: Calls `ExtensionManager::search_available_extensions()`.
3. **Frontend Tools**: If `Agent::is_frontend_tool()` is true, `dispatch_tool_call` isn't directly hit in the same way; instead, `Agent::reply()` yields a `FrontendToolRequest` (see "Core Agent Loop").
4. **Router Vector Search Tool**: If `tool_call.name == ROUTER_VECTOR_SEARCH_TOOL_NAME`, it's dispatched to the `router_tool_selector.select_tools()`.
5. **Extension (MCP) Tools**: For other tools, it delegates to `ExtensionManager::dispatch_tool_call()`, which:
   - Finds the client based on the sanitized name prefix.
   - Strips the prefix to get the original tool name.
   - Calls `client.call_tool()`.
   - Returns a `ToolCallResult` containing a future for the result and a stream for notifications.

### Permissions (`PermissionManager`, `PermissionJudge`)
- `PermissionManager` (in `crates/goose/src/config/permission.rs` but used by agent) loads and saves tool permissions from/to `config.yaml`.
- `permission_judge::check_tool_permissions()` (in `crates/goose/src/permission/`) determines if a list of tool requests are:
    - **Approved**: Allowed by config, or read-only in "smart_approve".
    - **Denied**: Disallowed by config.
    - **Needs Approval**: Requires explicit user confirmation (based on `GOOSE_MODE` and config).
- **Permission Levels**: `AlwaysAllow`, `AskBefore`, `NeverAllow`.
- **Confirmation Flow**:
    - Agent sends `ToolConfirmationRequest` via `AgentEvent::Message`.
    - UI sends back `PermissionConfirmation` via HTTP POST to `/confirm`, which calls `Agent::handle_confirmation()`.
    - `Agent::handle_confirmation()` puts it on the `confirmation_tx` channel.
    - `Agent::handle_approval_tool_requests()` reads from `confirmation_rx` to proceed.

### Tool Output and Error Handling
- Tool results are `ToolResult<Vec<Content>>`. Errors are typically `ToolError`.
- `large_response_handler::process_tool_response()` may truncate or summarize large tool outputs before they are added to the message history.
- All tool errors are surfaced to the LLM to enable self-correction (a key design principle).

### Tool Monitoring (`ToolMonitor`)
- `crates/goose/src/tool_monitor.rs`.
- If enabled (`Agent::configure_tool_monitor()`), tracks tool call counts and arguments.
- `ToolMonitor::check_tool_call()` prevents excessive repetition of the same tool with the same arguments.

## Context Management (`context.rs`)
Located in `crates/goose/src/agents/context.rs` and `crates/goose/src/context_mgmt/`.
- **Building Context**: The list of `Message` objects forms the history. The `SystemPrompt` provides overall instructions. Tools are also part of the context given to the LLM.
- **Token Counting (`TokenCounter`)**:
    - Uses `tokenizers` crate, specific to the LLM provider/model (e.g., `cl100k_base` for GPT).
    - `count_chat_tokens()` estimates token count for messages and tools.
- **Truncation Strategies (`truncate.rs`)**:
    - `OldestFirstTruncation`: Implemented strategy. Removes oldest messages (after system prompt and first user message) until token count is within `target_context_limit`.
    - `Agent::truncate_context()` is the public API, called when `ContextLengthExceeded` occurs.
- **Summarization Strategies (`summarize.rs`)**:
    - `summarize_messages()`: Uses an LLM to summarize older parts of the conversation if truncation alone is insufficient or as an alternative strategy.
    - `Agent::summarize_context()` is the public API.

## Multi-Model Usage
- **Lead/Worker Pattern**:
    - `GOOSE_LEAD_MODEL` and `GOOSE_LEAD_PROVIDER` environment variables configure a (typically more powerful) model for complex reasoning/planning.
    - The main provider (`provider` field in `Agent`) can be a "worker" model.
    - The `LeadWorkerProvider` (in `crates/goose/src/providers/lead_worker.rs`) likely orchestrates this, though its direct integration into `Agent::reply` isn't explicitly detailed in `agent.rs` but is a known architectural pattern of the system. The agent prepares tools and prompts, and the provider implementation handles which model to call.
- **Planner Model**:
    - `GOOSE_PLANNER_MODEL` and `GOOSE_PLANNER_PROVIDER` for a specialized planning model, often used for the `/plan` command in the CLI.
    - `Agent::get_plan_prompt()` prepares a specific prompt for planning.
- **Tool Shim Model**:
    - Some providers (like `ToolShimProvider`) can use a different model specifically for understanding tool schemas and generating tool calls, while another model handles the main chat/reasoning.

## Prompt Management (`PromptManager`)
Located in `crates/goose/src/agents/prompt_manager.rs`.
- **System Prompts**:
    - Loads base system prompts (e.g., `system.md`, `system_gpt_4.1.md`).
    - `build_system_prompt()` assembles the final system prompt by incorporating:
        - Base system instructions.
        - Extension information (instructions, resource capabilities).
        - Frontend tool instructions.
        - Optional "extra" instructions added dynamically (`Agent::extend_system_prompt()`).
        - LLM-specific formatting.
- **Extension-Provided Prompts**: `ExtensionManager::list_prompts()` and `get_prompt()` allow extensions to expose named prompts that can be retrieved and used.
- **Recipe Prompt**: `get_recipe_prompt()` for generating recipes.
- **Planning Prompt**: `ExtensionManager::get_planning_prompt()` for generating plans.
- **Dynamic Prompt Construction**: Uses `minijinja` for templating (evident from `prompt_template.rs` and dependencies).

## Notifier System
- The root `ARCHITECTURE.md` describes a `Notifier` trait:
  ```rust
  trait Notifier {
      fn log(&self, content: RichRenderable);
      fn status(&self, message: String);
  }
  ```
- This trait is implemented by the UX layer (CLI, Desktop Server) to receive logs and status updates from the agent core and extensions.
- Extensions receive a `Box<dyn Notifier>` to report their actions. This decouples core logic from UI specifics.

## Frontend Interaction (Desktop UI / CLI)
- **CLI (`goose-cli`)**:
    - Instantiates and runs the `Agent`.
    - Implements `Notifier` for console output.
    - Handles user input from stdin and sends it to `Agent::reply()`.
    - Manages its own session loop.
- **Desktop Server (`goose-server` -> `goosed` binary)**:
    - Instantiates and manages `Agent` instances (potentially one per session/user).
    - Exposes the agent's functionality via an HTTP API (see `API_REFERENCE.md`).
    - Implements `Notifier` to relay information to the connected desktop client (e.g., via WebSockets or long polling, though specific mechanism isn't detailed here).
    - Handles `FrontendToolRequest`s by forwarding them to the actual Desktop UI for execution and then sending results back.
    - Manages permission confirmations originating from the Desktop UI.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
