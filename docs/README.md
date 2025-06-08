---
Version: 1.0.24
Last Generated: 2025-06-08
---

# codename goose README

## Table of Contents
- [Project Overview and Purpose](#project-overview-and-purpose)
- [Quick Start](#quick-start)
- [Prerequisites and System Requirements](#prerequisites-and-system-requirements)
- [Basic Installation Instructions](#basic-installation-instructions)
- [Other Documentation](#other-documentation)

## Project Overview and Purpose
`codename goose` is a local, extensible, open-source AI agent designed to automate engineering tasks. It can handle complex development workflows, including building projects, writing and executing code, debugging, orchestrating tasks, and interacting with external APIs autonomously.

The key features of `codename goose` include:
- **Autonomous Task Execution**: Goes beyond code suggestions to manage entire project lifecycles.
- **Extensibility**: Supports integration with any LLM and MCP (Model Context Protocol) servers.
- **Multiple Model Configuration**: Allows using different models for different purposes (e.g., powerful models for planning, faster/cheaper models for execution) to optimize performance and cost. This includes a Lead/Worker model pattern and a specialized Planning Model.
- **Cross-Platform Availability**: Available as a desktop application and a CLI.
- **Comprehensive Tooling**: Provides a rich set of built-in tools and capabilities.

The goal of `codename goose` is to enhance developer productivity by automating complex tasks, allowing developers to focus on innovation.

## Quick Start
For a comprehensive quick start guide, please refer to the [official Quickstart documentation](https://block.github.io/goose/docs/quickstart).

A basic way to get started with the CLI after installation:
1. Configure your AI provider (e.g., OpenAI):
   ```bash
   export OPENAI_API_KEY="your_api_key_here"
   # Or for other providers, e.g., Anthropic
   # export ANTHROPIC_API_KEY="your_api_key_here"
   ```
2. Start a new session:
   ```bash
   goose session start
   ```
3. Issue a command to goose, for example:
   ```
   /issue "Create a hello world python script and run it"
   ```

## Prerequisites and System Requirements
- **Rust**: The core of `codename goose` is built in Rust. You'll need the Rust toolchain installed. See [rustup.rs](https://rustup.rs/) for installation.
- **Node.js**: For the desktop UI and some development scripts.
- **Platform-specific build tools**:
    - **Windows**: MSVC build tools.
    - **Linux**: Standard build essentials (gcc, make, etc.).
    - **macOS**: Xcode command-line tools.
- **Hermit (Optional but Recommended)**: This project uses Hermit for managing development environments. See `bin/activate-hermit`.
- **Git**: For version control.

System requirements will vary based on the models used and the complexity of tasks, but generally, a modern development machine should be sufficient.

## Basic Installation Instructions
For detailed and up-to-date installation instructions, please visit the [official Installation guide](https://block.github.io/goose/docs/getting-started/installation).

A general overview of building from source:
1. **Clone the repository**:
   ```bash
   git clone https://github.com/block/goose.git
   cd goose
   ```
2. **Set up Hermit (recommended)**:
   ```bash
   ./bin/activate-hermit
   ```
3. **Build the project**:
   - For a release build of the CLI and other binaries:
     ```bash
     just release-binary
     ```
     The binaries will be in `target/release/` and potentially copied to the root `dist/` folder.
   - For a debug build:
     ```bash
     cargo build
     ```
4. **Running the CLI**:
   After building, the CLI (named `goose` or `goose-cli`) will be available in `target/release/` or `target/debug/`. If `just release-binary` was used, it might be in `dist/`.
   ```bash
   # Example if in target/release
   ./target/release/goose session start
   ```

## Other Documentation
This directory contains further detailed documentation:
- [OVERVIEW.md](./OVERVIEW.md): A high-level introduction to what Goose is and how it works.
- [AGENT.md](./AGENT.md): In-depth explanation of the agent's internal workings.
- [ARCHITECTURE.md](./ARCHITECTURE.md): High-level system architecture, patterns, and decisions.
- [TECH_STACK.md](./TECH_STACK.md): Breakdown of the technologies, languages, and frameworks used.
- [COMPONENTS.md](./COMPONENTS.md): Detailed information on major components and modules.
- [BUILD_AND_DEPLOY.md](./BUILD_AND_DEPLOY.md): Comprehensive guide to building, setting up the development environment, and deployment.
- [DEBUGGING_AND_TROUBLESHOOTING.md](./DEBUGGING_AND_TROUBLESHOOTING.md): Tips for debugging, logging, and common issues.
- [API_REFERENCE.md](./API_REFERENCE.md): Documentation for any public APIs or FFI interfaces.
- [DATABASE_SCHEMA.md](./DATABASE_SCHEMA.md): Information on database structure (if applicable).
- [IDEAS.md](./IDEAS.md): Exploration of potential future extensions and adaptations for Goose.

For the official, user-facing documentation, please visit [https://block.github.io/goose/](https://block.github.io/goose/).

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
