# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building the Project

```bash
# Build release version with binary copying
just release-binary

# Build debug version
cargo build

# Build for specific platforms
just release-windows    # Windows build via Docker
just release-intel      # Intel Mac build

# Build specific components
cargo build -p goose-cli       # CLI only
cargo build -p goose-server    # Server only
```

### Running Components

```bash
# Run the desktop UI (builds release first)
just run-ui

# Run the CLI
cargo run -p goose-cli -- session start

# Run the server
just run-server

# Run documentation site
just run-docs

# Run with debug build
just run-dev
```

### Testing and Quality Checks

```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p goose-llm

# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Desktop app linting
cd ui/desktop && npm run lint:check

# Run benchmarks
cargo bench
```

### Release Management

```bash
# Create a new release (from main branch)
just release 1.2.3

# Tag current version
just tag

# Push tag to trigger release CI
just tag-push
```

## High-Level Architecture

### Core Agent Framework
The `goose` crate contains the central agent logic:
- **Extension System**: Plugins provide tools, state management, and custom prompts
- **Tool Execution**: Error-aware tool calling with results surfaced to the model
- **Context Management**: Smart truncation and summarization for long conversations
- **Provider Abstraction**: Unified interface for 15+ LLM providers

### Multi-Model Support
Goose supports using different models for different purposes:
- **Lead/Worker Pattern**: Use expensive models for planning, cheaper for execution
- **Planning Model**: Specialized model for the `/plan` command
- Configure via `GOOSE_LEAD_MODEL`, `GOOSE_LEAD_PROVIDER`, `GOOSE_PLANNER_MODEL`, etc.

### MCP (Model Context Protocol)
The project implements and uses MCP for tool extensions:
- `mcp-client`, `mcp-core`, `mcp-server`: Core protocol implementation
- `goose-mcp`: Built-in MCP servers (developer tools, memory, Google Drive, etc.)
- Extensions can be MCP servers or native Rust implementations

### Frontend Architecture
- **Desktop App**: Electron + React + TypeScript application in `ui/desktop`
- **Build Process**: Rust binary (`goosed`) is embedded in the Electron app
- **API Communication**: Desktop app communicates with embedded server via HTTP

### Testing Strategy
- **Unit Tests**: Colocated with source files, run with `cargo test`
- **Integration Tests**: In `tests/` directories, test provider integrations
- **E2E Tests**: Playwright tests for desktop app
- **Benchmarking**: Comprehensive evaluation framework in `goose-bench`

### Key Development Patterns
1. **Error Handling**: All tool failures are surfaced to the model for self-correction
2. **Streaming Responses**: Event-driven architecture for real-time interaction
3. **Permission System**: Fine-grained control over tool execution
4. **Provider Flexibility**: Easy addition of new LLM providers via trait implementation