---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Debugging and Troubleshooting

## Table of Contents
- [Overview](#overview)
- [General Debugging Techniques](#general-debugging-techniques)
  - [Enable Verbose Logging](#enable-verbose-logging)
  - [Run in Debug Mode](#run-in-debug-mode)
  - [Examine Tool Interactions](#examine-tool-interactions)
  - [Isolate Components](#isolate-components)
- [Logging](#logging)
  - [Log Locations](#log-locations)
    - [CLI Logs (`goose-cli`)](#cli-logs-goose-cli)
    - [Server Logs (`goose-server`/`goosed`)](#server-logs-goose-servergoosed)
  - [Log Format](#log-format)
  - [Configuring Log Levels](#configuring-log-levels)
  - [Langfuse Integration](#langfuse-integration)
- [Common Issues and Solutions](#common-issues-and-solutions)
  - [Permission Issues (macOS)](#permission-issues-macos)
  - [Hermit Errors](#hermit-errors)
  - [API Errors (Token/Credit Issues)](#api-errors-tokencredit-issues)
  - [Keychain/Keyring Errors](#keychainkeyring-errors)
  - [Package Runner Issues for Extensions](#package-runner-issues-for-extensions)
  - [Connection Error with Ollama Provider on WSL](#connection-error-with-ollama-provider-on-wsl)
  - [Context Length Exceeded](#context-length-exceeded)
  - [Rate Limit Errors (429)](#rate-limit-errors-429)
  - [Agent Stuck or Unresponsive](#agent-stuck-or-unresponsive)
  - [DeepSeek Models with Ollama](#deepseek-models-with-ollama)
  - [Goose Edits Files Unexpectedly](#goose-edits-files-unexpectedly)
- [Testing Strategies](#testing-strategies)
  - [Running Unit and Integration Tests](#running-unit-and-integration-tests)
  - [Running Desktop App E2E Tests](#running-desktop-app-e2e-tests)
  - [Running Benchmarks](#running-benchmarks)
- [Performance Monitoring](#performance-monitoring)
  - [Benchmarking Framework (`goose-bench`)](#benchmarking-framework-goose-bench)
  - [Tips from `ARCHITECTURE.md`](#tips-from-architecturemd)

## Overview
This document provides guidance on debugging `codename goose`, understanding its logging mechanisms, solutions to common problems, and testing strategies.

## General Debugging Techniques

### Enable Verbose Logging
Increase log verbosity using environment variables (see [Configuring Log Levels](#configuring-log-levels)) to get more detailed output from specific modules. For example, `RUST_LOG="goose=trace,goose_cli=trace"`

### Run in Debug Mode
- For Rust components: Build with `cargo build` (instead of `--release`) and run the debug binaries from `target/debug/`.
- For the Desktop UI with a debug backend: Use `just run-dev`. This builds the Rust backend in debug mode, copies the binary, and then starts the UI.

### Examine Tool Interactions
A core aspect of `goose` is its interaction with tools (both internal and MCP-based). Logs will show:
- Which tool is being called.
- The parameters passed to the tool.
- The output or error from the tool.
This is crucial for understanding the agent's decision-making process and where things might be going wrong. The design principle "All tool failures are surfaced to the model for self-correction" means errors are part of the expected flow and can be observed in logs.

### Isolate Components
- **Test CLI separately**: `cargo run -p goose-cli -- session start`
- **Test Server separately**: `just run-server` (runs `cargo run -p goose-server`)
- **Test specific MCP servers**: e.g., `npx @modelcontextprotocol/inspector cargo run -p goose-mcp --example mcp` (from `crates/goose-mcp/README.md`)

## Logging
`codename goose` uses the `tracing` ecosystem for logging in its Rust components.

### Log Locations

#### CLI Logs (`goose-cli`)
- **Directory**:
    - macOS/Linux: `~/.local/state/goose/logs/cli/<YYYY-MM-DD>/`
    - Windows: `~\AppData\Roaming\Block\goose\data\logs\cli\<YYYY-MM-DD>/`
    (Path derived from `etcetera::choose_app_strategy()` and `crates/goose-cli/src/logging.rs`)
- **Filename**: `<YYYYMMDD_HHMMSS>.log` or `<YYYYMMDD_HHMMSS>-<session_name>.log`. A new log file is created for each session, prefixed with a timestamp.

#### Server Logs (`goose-server`/`goosed`)
- **Directory**:
    - macOS/Linux: `~/.local/state/goose/logs/server/<YYYY-MM-DD>/`
    - Windows: `~\AppData\Roaming\Block\goose\data\logs\server\<YYYY-MM-DD>/`
    (Path derived from `etcetera::choose_app_strategy()` and `crates/goose-server/src/logging.rs`)
- **Filename**: Similar to CLI logs, `<YYYYMMDD_HHMMSS>.log` or with a name if provided during setup.

### Log Format
- **File Logs**: JSON formatted, including target, level, and other fields.
- **Console Logs (Development/CLI)**: Pretty-formatted, with level, target, and optionally file/line numbers.

### Configuring Log Levels
Log levels are primarily controlled by the `RUST_LOG` environment variable.
- **Default CLI Filter Example (`crates/goose-cli/src/logging.rs`)**:
  `RUST_LOG="mcp_server=debug,mcp_client=debug,goose=debug,goose_cli=info,warn"`
  (Sets `mcp_server`, `mcp_client`, `goose` to DEBUG, `goose_cli` to INFO, and everything else to WARN).
- **Default Server Filter Example (`crates/goose-server/src/logging.rs`)**:
  `RUST_LOG="mcp_server=debug,mcp_client=debug,goose=debug,goose_server=info,tower_http=info,warn"`

To get more detailed logs for a specific component, you can override these. For example, for verbose `goose` core logs:
`export RUST_LOG=goose=trace`

### Langfuse Integration
- The system can integrate with [Langfuse](https://langfuse.com/) for observability if configured.
- `langfuse_layer::create_langfuse_observer()` in `logging.rs` files attempts to set this up.
- Requires environment variables like `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY`, and `LANGFUSE_URL` (or `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` / `_SECRET_KEY`).
- The `Justfile` contains a `langfuse-server` recipe using `./scripts/setup_langfuse.sh` for local setup.

## Common Issues and Solutions
(Many of these are sourced from `documentation/docs/troubleshooting.md`)

### Permission Issues (macOS)
- **Symptom**: Desktop app shows no window on launch, or tools fail to create files.
- **Cause**: Goose needs read/write access to `~/.config` (for logs, config) and potentially other directories for its operations.
- **Fix**:
    1. Check permissions: `ls -ld ~/.config`
    2. Grant permissions: `chmod u+rw ~/.config` (create with `mkdir -p ~/.config` if it doesn't exist).
    3. Check System Settings: `System Settings > Privacy & Security > Files & Folders` and grant Goose access.
    4. As a last resort for diagnosis: `sudo /Applications/Goose.app/Contents/MacOS/Goose` (not a permanent solution).

### Hermit Errors
- **Symptom**: Errors like "hermit:fatal" when installing extensions in the app.
- **Cause**: Cache issues with Hermit if an older version was used.
- **Fix**: Clear Hermit cache. On macOS: `sudo rm -rf ~/Library/Caches/hermit`.

### API Errors (Token/Credit Issues)
- **Symptom**: Errors like `httpx.HTTPStatusError: Client error '404 Not Found'` for provider URLs, or messages indicating token issues.
- **Cause**: Exhausted API credits, invalid API key, or incorrect provider configuration.
- **Fix**:
    1. Check API credit balance with your LLM provider.
    2. Reconfigure API key: `goose configure` (CLI) or via Desktop UI settings.

### Keychain/Keyring Errors
- **Symptom**: Errors like "Failed to access secure storage (keyring): Platform secure storage failure: DBus error..."
- **Cause**: System environment lacks keyring support (common in some Linux setups or headless environments).
- **Fix**:
    1. Set API keys via environment variables (e.g., `export OPENAI_API_KEY=your_key_here`).
    2. When `goose configure` prompts to save to keyring, select `No`.
    3. Alternatively, disable keyring use entirely: `export GOOSE_DISABLE_KEYRING=1`. Secrets will then be stored in `~/.config/goose/secrets.yaml` (macOS/Linux) or `%APPDATA%\Block\goose\config\secrets.yaml` (Windows).

### Package Runner Issues for Extensions
- **Symptom**: "Failed to start extension: {extension name}, 'No such file or directory (os error 2)'".
- **Cause**: The system might be missing a required package runner (e.g., `npx` for Node.js based extensions).
- **Fix**: Install the necessary runner (e.g., install Node.js for `npx`).

### Connection Error with Ollama Provider on WSL
- **Symptom**: "Execution error: error sending request for url (http://localhost:11434/v1/chat/completions)" when using Ollama on WSL.
- **Cause**: WSL might be using a different IP for localhost, or network mirroring isn't set up.
- **Fix**:
    1. Check if Ollama is running: `curl http://localhost:11434/api/tags` from within WSL.
    2. If it fails, find WSL's gateway IP: `ip route show | grep -i default | awk '{ print $3 }'`. Use this IP in Goose configuration for Ollama.
    3. For Windows 11 22H2+, consider enabling WSL's Mirrored Networking mode.

### Context Length Exceeded
- **Symptom**: Errors indicating the input to the LLM is too long.
- **Fix**: Break down input into smaller parts. Use `.goosehints` for detailed context.

### Rate Limit Errors (429)
- **Symptom**: HTTP 429 errors from LLM providers.
- **Fix**: The existing documentation suggests using OpenRouter. Refer to `documentation/docs/guides/handling-llm-rate-limits-with-goose.md`.

### Agent Stuck or Unresponsive
- **Symptom**: Goose seems to be in a loop or not responding.
- **Fix**:
    1. Interrupt: `CTRL+C`.
    2. Start a new session: `goose session start`.
    3. For complex tasks, break them into smaller sessions.

### DeepSeek Models with Ollama
- **Symptom**: Issues when using DeepSeek models via Ollama.
- **Cause**: DeepSeek models may not support tool calling.
- **Fix**: Disable all Goose extensions when using these models. This limits autonomous capabilities. Ollama's other models like `qwen2.5` might offer better tool support.

### Goose Edits Files Unexpectedly
- **Symptom**: Goose modifies files you didn't intend.
- **Mitigation**: Use version control (Git). Commit your changes before running Goose. Review Goose's changes before committing them.

## Testing Strategies

### Running Unit and Integration Tests
- **Run all tests for the workspace**:
  ```bash
  cargo test
  # From crates/ directory as per ci.yml
  # (cd crates && source ../bin/activate-hermit && cargo test)
  ```
- **Run tests for a specific crate**:
  ```bash
  cargo test -p <crate_name>
  # e.g., cargo test -p goose-llm
  ```
- **CI Setup**: The `.github/workflows/ci.yml` file shows setup for Linux tests, including installing `libdbus-1-dev gnome-keyring libxcb1-dev` and unlocking the keyring. This might be relevant for local test environments if encountering keyring issues.

### Running Desktop App E2E Tests
- The `ui/desktop/package.json` includes scripts for Playwright E2E tests:
  ```bash
  cd ui/desktop
  npm run test-e2e
  # For UI mode: npm run test-e2e:ui
  # For debugging: npm run test-e2e:debug
  ```

### Running Benchmarks
- The `goose-bench` crate is dedicated to benchmarking.
- **Commands** (from `crates/goose-bench/README.md`):
  ```bash
  # Run benchmarks defined in a config file
  goose bench run --config /path/to/your-config.json
  # Generate leaderboard from results
  goose bench generate-leaderboard --benchmark-dir /path/to/benchmark-output-directory
  ```
- **Error Handling in Benchmarks**: The `goose-bench` README notes that it doesn't have robust error handling for issues like rate limiting or network errors during evaluation runs. It advises checking `aggregate_metrics.csv` for `server_error_mean` and reviewing session logs (`.jsonl` files).

## Performance Monitoring

### Benchmarking Framework (`goose-bench`)
- This is the primary tool for performance monitoring of different LLMs and configurations.
- It measures execution time, token usage, tool calls, and scores for various evaluation suites.
- See `crates/goose-bench/README.md` for detailed output structure and metrics.

### Tips from `ARCHITECTURE.md`
The original `ARCHITECTURE.md` mentions some performance-driving implementation choices:
- Encouraging `ripgrep` usage via shell for file navigation.
- Using a "replace" operation for file edits (fewer tokens) but allowing whole file overwrites for major refactors.
These are not direct monitoring approaches but design choices impacting performance that one could keep in mind when debugging performance issues.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
