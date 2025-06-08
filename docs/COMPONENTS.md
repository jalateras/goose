---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Major Components and Modules

## Table of Contents
- [Overview](#overview)
- [Backend Components (Rust Crates)](#backend-components-rust-crates)
  - [1. `goose`](#1-goose)
  - [2. `goose-cli`](#2-goose-cli)
  - [3. `goose-server`](#3-goose-server)
  - [4. `goose-llm`](#4-goose-llm)
  - [5. `goose-mcp`](#5-goose-mcp)
  - [6. `mcp-core`](#6-mcp-core)
  - [7. `mcp-client`](#7-mcp-client)
  - [8. `mcp-server`](#8-mcp-server)
  - [9. `mcp-macros`](#9-mcp-macros)
  - [10. `goose-ffi`](#10-goose-ffi)
  - [11. `goose-bench`](#11-goose-bench)
- [Frontend Components](#frontend-components)
  - [1. Desktop UI (`ui/desktop/`)](#1-desktop-ui-uidesktop)
  - [2. Desktop UI V2 (`ui-v2/`)](#2-desktop-ui-v2-ui-v2)
  - [3. Documentation Site (`documentation/`)](#3-documentation-site-documentation)
- [Bindings Components](#bindings-components)
  - [1. Kotlin Bindings (`bindings/kotlin/`)](#1-kotlin-bindings-bindingskotlin)
  - [2. Python Bindings (`bindings/python/`)](#2-python-bindings-bindingspython)
- [Inter-Component Relationships (Simplified)](#inter-component-relationships-simplified)

## Overview
The `codename goose` project is structured into several key components, primarily as Rust crates for the backend logic and TypeScript/JavaScript projects for the frontends. The Model Context Protocol (MCP) also has its own set of core, client, and server crates.

## Backend Components (Rust Crates)
These are located in the `crates/` directory.

### 1. `goose`
- **Purpose**: This is the core agent framework. It contains the central logic for the AI agent, including the extension system, tool execution, context management, and LLM provider abstraction.
- **Key Responsibilities**:
    - Managing agent sessions and state.
    - Orchestrating interactions between LLMs, tools, and users.
    - Implementing the "Exchange" loop for LLM generation and tool calls.
    - Handling profiles for configuring agent capabilities and models.
    - Providing the `Extension` trait and managing extensions.
    - Abstracting LLM provider interactions (supports 15+ providers).
    - Implementing multi-model logic (Lead/Worker, Planning Model).
    - Smart context management (truncation, summarization).
- **Key Modules/Structs (Illustrative based on `ARCHITECTURE.md` and common patterns)**:
    - `Exchange`: Core interaction loop.
    - `Profile`: Configuration for an agent instance.
    - `Extension` trait and implementations: Defines how new tools and capabilities are added.
    - `Notifier` trait: Interface for sending logs and status updates to the UX.
    - Various provider implementations.
    - Context management modules.
    - Tool handling and permission systems.
- **Interfaces**: Exposes functionalities to `goose-cli` and `goose-server` for user interaction. Interacts with LLM provider APIs and potentially MCP servers.

### 2. `goose-cli`
- **Purpose**: Provides a Command Line Interface (CLI) for interacting with the `goose` agent.
- **Key Responsibilities**:
    - Parsing command-line arguments (using `clap`).
    - Managing CLI sessions.
    - Providing an interactive prompt (`rustyline`) and displaying output (`cliclack`, `console`, `bat`).
    - Implementing the `Notifier` trait for console output.
    - Translating user commands into actions for the `goose` core.
    - Hosting a simple web interface for some functionalities (`WEB_INTERFACE.md`).
- **Key Modules/Structs**:
    - `main.rs`: Entry point for the CLI.
    - Command handling modules (e.g., for `/issue`, `/plan`).
    - Session management specific to CLI.
    - CLI-specific `Notifier` implementation.
- **Interfaces**: Uses the `goose` crate for agent logic. Interacts with the user via the terminal.

### 3. `goose-server`
- **Purpose**: Provides an HTTP server (`goosed`) that exposes the `goose` agent's capabilities over an API. This is primarily used by the desktop UI.
- **Key Responsibilities**:
    - Hosting an Axum-based HTTP server.
    - Defining API endpoints for agent interaction (e.g., starting sessions, sending messages).
    - Managing agent state for multiple clients/sessions.
    - Handling WebSocket connections for real-time communication.
    - Implementing the `Notifier` trait to potentially relay logs/status via API responses or WebSockets.
    - Generating OpenAPI schema for its API.
- **Key Modules/Structs**:
    - `main.rs` (for `goosed` binary): Server entry point.
    - Axum route handlers.
    - API request/response types (likely using `serde`).
    - Server-specific `Notifier` implementation.
    - State management for active sessions.
- **Interfaces**: Uses the `goose` crate. Exposes an HTTP/WebSocket API (defined by OpenAPI schema) consumed by clients like `ui/desktop`.

### 4. `goose-llm`
- **Purpose**: A stateless library focused on LLM provider interactions and prompt-related logic, designed for FFI (Foreign Function Interface) use.
- **Key Responsibilities**:
    - Providing functions for chat completion with various model providers.
    - Methods for text summarization and truncation.
    - Detecting read-only tools for smart approval logic.
    - Generating FFI bindings (e.g., for Kotlin, Python) via UniFFI.
- **Key Modules/Structs**:
    - Provider interaction logic.
    - Prompt templating/formatting utilities.
    - FFI binding generation setup (`uniffi-bindgen.rs`).
- **Interfaces**: Used by the main `goose` crate for some LLM operations. Exposes an FFI interface for use by other languages.
- **README**: `crates/goose-llm/README.md` provides details on FFI usage and examples.

### 5. `goose-mcp`
- **Purpose**: Implements a collection of built-in MCP (Model Context Protocol) servers. These servers provide tools that the `goose` agent can use.
- **Key Responsibilities**:
    - Hosting MCP servers for various functionalities:
        - `developer`: Filesystem operations, shell command execution.
        - `memory`: Long-term memory for the agent.
        - `google_drive`: Integration with Google Drive.
        - Computer controller: Screen capture, etc.
        - Other specific tools (JetBrains, tutorial).
    - Implementing the tool logic defined by MCP.
- **Key Modules/Structs**:
    - Routers and handlers for different MCP toolsets (e.g., `DeveloperRouter`).
    - Implementations for specific tools (e.g., file read/write, shell execution).
- **Interfaces**: Exposes MCP endpoints that can be called by an MCP client (like the one within the `goose` agent or `mcp-client`).
- **README**: `crates/goose-mcp/README.md` mentions testing with MCP Inspector.

### 6. `mcp-core`
- **Purpose**: Defines the core data structures, traits, and protocol messages for the Model Context Protocol (MCP).
- **Key Responsibilities**:
    - Defining types for tools, resources, prompts, content, roles, etc.
    - Specifying the request/response formats for MCP communication.
    - Providing foundational elements for `mcp-client` and `mcp-server`.
- **Key Modules/Structs**:
    - `protocol.rs`: MCP message definitions.
    - `tool.rs`, `resource.rs`, etc.: Core MCP type definitions.
- **Interfaces**: Used as a dependency by `mcp-client`, `mcp-server`, `goose`, and `goose-mcp`.

### 7. `mcp-client`
- **Purpose**: Provides a client library for interacting with MCP servers.
- **Key Responsibilities**:
    - Sending requests to MCP servers.
    - Handling responses from MCP servers.
    - Supporting different transport mechanisms (e.g., stdio, SSE).
- **Key Modules/Structs**:
    - `client.rs`: Main client logic.
    - Transport specific modules.
- **Interfaces**: Used by the `goose` agent when an extension is implemented as an external MCP tool. Connects to MCP servers built with `mcp-server`.
- **README**: `crates/mcp-client/README.md` provides examples for stdio and SSE transport.

### 8. `mcp-server`
- **Purpose**: A framework for building MCP servers.
- **Key Responsibilities**:
    - Providing abstractions to simplify the creation of MCP-compliant tool servers.
    - Handling the MCP protocol details for incoming requests.
    - Routing requests to appropriate tool handlers.
- **Key Modules/Structs**:
    - Server framework components.
    - Request routing and dispatching logic.
- **Interfaces**: Used by `goose-mcp` to build its MCP tool servers. Listens for requests from MCP clients.
- **README**: `crates/mcp-server/README.md` mentions testing with MCP Inspector.

### 9. `mcp-macros`
- **Purpose**: Contains procedural macros to simplify working with MCP, likely for auto-generating boilerplate code for tool definitions or server setup.
- **Key Responsibilities**:
    - Providing macros like `#[tool]` (as seen in `ARCHITECTURE.md`, though that example might be illustrative for the `Extension` trait within `goose` itself, the concept is similar for MCP tools).
- **Interfaces**: Used by developers when creating MCP tools or servers.

### 10. `goose-ffi`
- **Purpose**: Provides a C-compatible Foreign Function Interface (FFI) for the Goose AI agent framework.
- **Key Responsibilities**:
    - Exposing core `goose` functionalities (like agent creation, message sending) via C bindings.
    - Enabling integration with other programming languages (Python, Java/Kotlin, etc.).
    - Generating a C header file (`goose_ffi.h`) via `cbindgen`.
- **Key Modules/Structs**:
    - FFI wrapper functions for `goose` core objects and methods.
    - Data structures compatible with C.
- **Interfaces**: Exposes a C ABI. Used by external language wrappers (e.g., Python `ctypes`).
- **README**: `crates/goose-ffi/README.md` details build instructions, usage examples (Python), and supported provider configuration.

### 11. `goose-bench`
- **Purpose**: A framework for benchmarking and evaluating LLM models with the Goose framework.
- **Key Responsibilities**:
    - Running benchmark suites across multiple LLMs.
    - Generating structured reports (JSON, CSV).
    - Processing evaluation results, potentially with custom scripts and LLM-as-judge.
    - Calculating aggregate metrics and leaderboards.
- **Key Modules/Structs**:
    - Benchmark configuration parsing.
    - Evaluation suite runners.
    - Reporting and aggregation logic.
- **Interfaces**: Used via the `goose bench` CLI commands. Interacts with the `goose` core to run evaluations.
- **README**: `crates/goose-bench/README.md` provides detailed instructions on workflow, configuration, and available commands.

## Frontend Components

### 1. Desktop UI (`ui/desktop/`)
- **Purpose**: Provides a graphical user interface for interacting with the `goose` agent on the desktop.
- **Technology**: Electron, React, TypeScript, Vite.
- **Key Responsibilities**:
    - Rendering the user interface.
    - Managing UI state.
    - Communicating with the `goose-server` (embedded `goosed` binary) via HTTP API.
    - Displaying agent responses, logs, and tool interactions.
    - Handling user input and settings.
- **Key Files/Modules**:
    - `src/main.ts`: Electron main process.
    - `src/renderer.tsx`: React application entry point.
    - `src/App.tsx`: Main application component.
    - `src/goosed.ts`: Logic for managing the embedded Rust backend.
    - API client generated by `@hey-api/openapi-ts`.
    - Various React components in `src/components/`.
- **Interfaces**: Communicates with `goose-server`'s HTTP API. Interacts with the user graphically.

### 2. Desktop UI V2 (`ui-v2/`)
- **Purpose**: Appears to be a newer/alternative version of the desktop UI.
- **Technology**: Electron, React (or potentially SolidJS, though `package.json` lists React), TypeScript, Vite.
- **Key Responsibilities**: Similar to the current Desktop UI, but potentially with a different design, features, or underlying UI framework.
- **Key Files/Modules**:
    - `electron/main.ts`: Electron main process.
    - `src/main.tsx`: React application entry point.
    - Routing setup with `@tanstack/react-router`.
- **Interfaces**: Likely communicates with `goose-server`'s HTTP API.

### 3. Documentation Site (`documentation/`)
- **Purpose**: Provides user-facing documentation for the `codename goose` project.
- **Technology**: Docusaurus, React, TypeScript, MDX.
- **Key Responsibilities**:
    - Rendering documentation pages, guides, tutorials, and blog posts.
    - Providing navigation and search functionality.
- **Key Files/Modules**:
    - `docusaurus.config.ts`: Main Docusaurus configuration.
    - Content files in `docs/` and `blog/`.
    - Custom React components in `src/components/`.
- **Interfaces**: Serves web pages to users.

## Bindings Components

### 1. Kotlin Bindings (`bindings/kotlin/`)
- **Purpose**: Allows using parts of the `goose` system (specifically `goose-llm`) from Kotlin.
- **Technology**: Kotlin, JNA (Java Native Access).
- **Key Responsibilities**:
    - Providing Kotlin wrapper code (auto-generated by UniFFI) for `goose-llm`'s FFI.
    - Demonstrating usage via an example (`Usage.kt`).
- **Interfaces**: Interacts with the `libgoose_llm` dynamic library.

### 2. Python Bindings (`bindings/python/`)
- **Purpose**: Allows using parts of the `goose` system (specifically `goose-llm` and `goose-ffi`) from Python.
- **Technology**: Python, `ctypes`.
- **Key Responsibilities**:
    - Providing Python wrapper code (auto-generated by UniFFI for `goose-llm`, manual or `ctypes`-based for `goose-ffi`).
    - Demonstrating usage via examples (`usage.py` for `goose-llm`, `goose_agent.py` for `goose-ffi`).
- **Interfaces**: Interacts with the `libgoose_llm` and `libgoose_ffi` dynamic libraries.

## Inter-Component Relationships (Simplified)
- **User** interacts with **`goose-cli`** or **Desktop UI (`ui/desktop` or `ui-v2`)**.
- **Desktop UI** communicates with **`goose-server`** (which runs `goosed`).
- Both **`goose-cli`** and **`goose-server`** use the core **`goose`** crate for agent logic.
- **`goose`** crate uses:
    - **`goose-llm`** for some LLM utility functions and FFI capabilities.
    - LLM Provider APIs (external).
    - **`mcp-client`** to interact with tools provided by **`goose-mcp`** servers or other MCP servers.
- **`goose-mcp`** uses **`mcp-server`** framework to build its tool servers.
- **`mcp-client`** and **`mcp-server`** rely on definitions from **`mcp-core`**.
- **`goose-ffi`** provides C bindings to the **`goose`** core, usable by Python/Kotlin examples.
- **`goose-bench`** uses **`goose-cli`** infrastructure and **`goose`** core to run benchmarks.
- **Documentation Site** describes the entire system.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
