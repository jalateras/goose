---
Version: 1.0.24
Last Generated: 2025-06-08
---

# System Architecture

## Table of Contents
- [Overview](#overview)
- [Core Agent Framework (`goose` crate)](#core-agent-framework-goose-crate)
  - [Extension System](#extension-system)
  - [Tool Execution](#tool-execution)
  - [Context Management](#context-management)
  - [Provider Abstraction](#provider-abstraction)
- [Multi-Model Support](#multi-model-support)
  - [Lead/Worker Pattern](#leadworker-pattern)
  - [Planning Model](#planning-model)
- [MCP (Model Context Protocol)](#mcp-model-context-protocol)
- [Frontend Architecture](#frontend-architecture)
  - [Desktop Application](#desktop-application)
  - [CLI (Command Line Interface)](#cli-command-line-interface)
- [Key Architectural Patterns](#key-architectural-patterns)
  - [Plugin Architecture (Extensions)](#plugin-architecture-extensions)
  - [Service Abstraction (Providers, Notifier)](#service-abstraction-providers-notifier)
  - [State Management in Extensions](#state-management-in-extensions)
- [System Boundaries and External Dependencies](#system-boundaries-and-external-dependencies)
- [Data Flow (Textual Description)](#data-flow-textual-description)
  - [CLI Interaction Flow](#cli-interaction-flow)
  - [Desktop UI Interaction Flow](#desktop-ui-interaction-flow)
  - [Tool Execution Flow](#tool-execution-flow)
- [Key Architectural Decisions and Trade-offs](#key-architectural-decisions-and-trade-offs)
  - [Rust as Core Language](#rust-as-core-language)
  - [Extensibility via Extensions](#extensibility-via-extensions)
  - [Reflection and Error Surfacing](#reflection-and-error-surfacing)
  - [MCP for Tool Integration](#mcp-for-tool-integration)
  - [Multiple Frontends (Desktop and CLI)](#multiple-frontends-desktop-and-cli)

## Overview
`codename goose` is an AI agent designed to automate engineering tasks. Its architecture is centered around a core Rust-based agent framework that is extensible, supports multiple AI models, and can be interacted with via a desktop application or a CLI. The system emphasizes flexibility, allowing integration with various LLM providers and custom tools through an extension system.

## Core Agent Framework (`goose` crate)
The `goose` crate is the heart of the system, containing the central agent logic.

### Extension System
- **Purpose**: Allows adding new capabilities (tools), state management, and custom prompts to the agent.
- **Mechanism**: Extensions are Rust structs implementing the `Extension` trait. They can define tools (methods annotated with `#[tool]`), manage their own state, and provide custom prompts.
- **Configuration**: Extensions are specified in a `Profile` (e.g., YAML configuration), which also defines model choices.
- **Dependencies**: Extensions can depend on each other, enabling modularity and reuse of capabilities.

### Tool Execution
- **Mechanism**: The core logic (referred to as `exchange` in the existing `ARCHITECTURE.md`) handles the loop of LLM generation and tool calling.
- **Error Handling**: A key design principle is to surface all tool execution errors back to the LLM, allowing it to attempt self-correction.
- **Reflection**: Tool outputs and execution status are clearly reported to the LLM to inform its next steps.

### Context Management
- **Challenge**: LLMs have limited context windows.
- **Approach**: The system employs smart truncation and summarization techniques for long conversations to manage context effectively. (Details likely within the `goose` crate's context management modules).

### Provider Abstraction
- **Purpose**: To support a wide range of LLM providers through a unified interface.
- **Mechanism**: A trait-based system allows for different LLM providers to be implemented and used interchangeably. The system supports over 15 LLM providers.

## Multi-Model Support
`goose` can use different LLMs for different purposes to optimize for performance, cost, and capability.

### Lead/Worker Pattern
- **Concept**: A more powerful (and potentially expensive) model (the "lead" model) is used for initial planning and complex reasoning. A faster, cheaper model (the "worker" model) is used for executing the steps of the plan.
- **Configuration**: Managed via environment variables like `GOOSE_LEAD_MODEL` and `GOOSE_LEAD_PROVIDER`.

### Planning Model
- **Concept**: A specialized model can be configured for the `/plan` command in the CLI, allowing for dedicated planning capabilities.
- **Configuration**: Managed via environment variables like `GOOSE_PLANNER_PROVIDER` and `GOOSE_PLANNER_MODEL`.

## MCP (Model Context Protocol)
MCP is a protocol used for tool extensions, enabling tools to be run as separate processes or services.
- **Core Components**:
    - `mcp-core`: Defines the protocol (messages, resources, tools).
    - `mcp-client`: Client library for interacting with MCP servers.
    - `mcp-server`: Framework for building MCP servers.
- **Built-in MCP Servers (`goose-mcp` crate)**: `goose` includes several built-in MCP servers providing tools for:
    - Developer tasks (filesystem, shell)
    - Memory (long-term storage for the agent)
    - Google Drive integration, etc.
- **Flexibility**: Extensions can be native Rust implementations within the agent or external MCP servers.

## Frontend Architecture
Users interact with `goose` primarily through a desktop application or a CLI.

### Desktop Application
- **Technology**: Built using Electron, React, and TypeScript. Source code is in `ui/desktop/`.
- **Backend Communication**: The desktop app embeds a Rust binary (`goosed`, likely a packaged version of `goose-server`) and communicates with it via HTTP.
- **Build Process**: The Rust backend is compiled and then bundled into the Electron application.

### CLI (Command Line Interface)
- **Technology**: A Rust application (`goose-cli` crate).
- **Interaction Model**: Users issue commands (e.g., `/issue`, `/plan`) to interact with the agent.
- **Output**: Provides textual feedback, logs, and status updates to the console.

## Key Architectural Patterns

### Plugin Architecture (Extensions)
The extension system is a classic plugin architecture, allowing the core agent's functionality to be dynamically extended. Each extension encapsulates specific tools, state, and prompting logic.

### Service Abstraction (Providers, Notifier)
- **LLM Providers**: Abstracting LLM interactions behind a common trait allows for easy swapping and addition of new models/APIs.
- **Notifier Trait**: Decouples the core agent logic from the specific UX implementation. The CLI and Desktop UI provide their own `Notifier` implementations for logging and status updates.

### State Management in Extensions
Extensions are responsible for managing their own state (e.g., `appointments_state` in the `ScheduleExtension` example from `ARCHITECTURE.md`). This keeps state localized to the relevant functionality.

## System Boundaries and External Dependencies
- **LLM Providers**: External dependencies, accessed via APIs (e.g., OpenAI, Anthropic).
- **MCP Servers**: Can be external processes or services that the agent communicates with.
- **Operating System**: For file system access, shell command execution (via the "developer" extension or MCP tools).
- **External APIs**: Some tools or extensions might interact with other external APIs (e.g., Google Drive).
- **Development Environment**: Rust toolchain, Node.js (for UI development), platform-specific build tools.

## Data Flow (Textual Description)

### CLI Interaction Flow
1. User launches `goose-cli` and starts a session.
2. User inputs a command/prompt.
3. CLI interacts with the `goose` core (Exchange).
4. The `goose` core, using the configured profile (extensions, models):
    a. Processes the input, potentially using a "lead" model for planning.
    b. Determines necessary tool calls.
    c. Executes tools (either native Rust tools or via MCP to external servers). Tool execution involves the `Notifier` to log actions.
    d. Collects tool results.
    e. Sends results back to an LLM (potentially a "worker" model) for further processing or response generation.
    f. Streams the response back to the CLI.
5. CLI displays the output to the user.
6. The loop continues with further user input or autonomous agent actions.

### Desktop UI Interaction Flow
1. User launches the desktop application.
2. The Electron app starts the embedded `goosed` server.
3. User interacts with the React-based UI.
4. UI components send requests to the local `goosed` server via HTTP.
5. The `goosed` server (which wraps the `goose` core) processes requests similarly to the CLI flow (see steps 4a-f above).
6. Responses are sent back to the UI via HTTP.
7. The UI updates dynamically to display information, logs, and agent responses.

### Tool Execution Flow
1. LLM decides to use a tool and generates tool call parameters.
2. The `goose` core (Exchange) identifies the responsible extension for the tool.
3. **For Native Tools**: The tool's Rust function within the extension is called directly.
    a. The tool function executes its logic (e.g., interacts with the filesystem, calls an OS command).
    b. It uses the `Notifier` to log its actions and status.
    c. It returns a result (or error) to the Exchange.
4. **For MCP Tools**:
    a. The `goose` core (via an MCP client component) sends a request to the appropriate MCP server.
    b. The MCP server executes the tool and returns a response.
    c. The MCP client receives the response and passes it to the Exchange.
5. The Exchange surfaces the tool's output (or error) to the LLM for the next step.

## Key Architectural Decisions and Trade-offs

### Rust as Core Language
- **Pros**: Performance, memory safety, strong type system, good for CLI tools and systems programming. Enables compilation to native binaries.
- **Cons**: Steeper learning curve for some developers, potentially slower initial development compared to higher-level languages for some tasks.

### Extensibility via Extensions
- **Pros**: Modular design, allows for a rich ecosystem of tools, enables customization for specific use cases.
- **Cons**: Can increase complexity in managing dependencies between extensions and ensuring overall system stability if extensions are poorly written.

### Reflection and Error Surfacing
- **Decision**: Tool outputs and errors are always shown to the LLM.
- **Pros**: Enables the LLM to self-correct, learn from mistakes, and handle unexpected situations more robustly.
- **Cons**: Can lead to longer conversations or more tokens if the LLM struggles with repeated errors. Requires careful prompt engineering to guide the LLM in error recovery.

### MCP for Tool Integration
- **Pros**: Allows tools to be developed and run independently of the main agent process, supports tools in different languages, enhances modularity.
- **Cons**: Introduces inter-process communication overhead, adds complexity of managing MCP servers.

### Multiple Frontends (Desktop and CLI)
- **Pros**: Caters to different user preferences (GUI vs. command line), broader accessibility.
- **Cons**: Requires maintaining separate UI codebases and ensuring consistent core functionality across frontends. The `Notifier` trait and `goose-server` help mitigate this by centralizing core logic.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
