---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Technology Stack

## Table of Contents
- [Overview](#overview)
- [Programming Languages](#programming-languages)
- [Backend (Core Agent & Servers)](#backend-core-agent--servers)
  - [Key Rust Crates/Libraries](#key-rust-crateslibraries)
  - [Async Runtime](#async-runtime)
  - [Web Framework (Server Components)](#web-framework-server-components)
  - [Serialization/Deserialization](#serializationdeserialization)
  - [Configuration](#configuration)
  - [CLI Framework](#cli-framework)
  - [Database/Storage](#databasestorage)
- [Frontend (Desktop UI & Documentation)](#frontend-desktop-ui--documentation)
  - [Desktop UI (Current - `ui/desktop`)](#desktop-ui-current---uidesktop)
    - [Framework & Build Tools](#framework--build-tools)
    - [Key Libraries](#key-libraries)
  - [Desktop UI (V2 - `ui-v2`)](#desktop-ui-v2---ui-v2)
    - [Framework & Build Tools](#framework--build-tools-1)
    - [Key Libraries](#key-libraries-1)
  - [Documentation Site (`documentation/`)](#documentation-site-documentation)
    - [Framework](#framework)
    - [Key Libraries](#key-libraries-2)
- [Bindings](#bindings)
- [Build System & Task Runners](#build-system--task-runners)
- [Development Environment & Tooling](#development-environment--tooling)
- [CI/CD](#cicd)
- [External Services & APIs (Integrations)](#external-services--apis-integrations)

## Overview
This project utilizes a diverse technology stack, with Rust forming the backbone of the core agent, CLI, and server components. JavaScript/TypeScript is used for the desktop application UIs and the documentation website. Various build tools, libraries, and frameworks support development across these different parts of the project.

## Programming Languages
- **Rust**: (Stable channel, profile: default - as per `rust-toolchain.toml`)
    - Edition: 2021 (as per root `Cargo.toml`)
    - Used for: Core agent logic, CLI, HTTP servers, MCP servers, and FFI bindings.
- **TypeScript**:
    - Used for: Desktop applications (`ui/desktop`, `ui-v2`), documentation site (`documentation/`).
    - Versions vary across `package.json` files (e.g., `~5.5.0` in `ui/desktop`, `^5.8.3` in `ui-v2`, `~5.6.2` in `documentation`).
- **JavaScript**: Used alongside TypeScript in UI and documentation projects.
- **Kotlin**: Used for example bindings (`bindings/kotlin/`).
- **Python**: Used for example bindings (`bindings/python/`).
- **Shell Script (sh/bash)**: Used in `Justfile` and CI workflow scripts.
- **PowerShell**: Used in `Justfile` for Windows-specific tasks.

## Backend (Core Agent & Servers)

### Key Rust Crates/Libraries
- **Core Logic & Utilities**:
    - `anyhow`, `thiserror`: Error handling.
    - `serde`, `serde_json`, `serde_yaml`: Data serialization and deserialization.
    - `regex`: Regular expression matching.
    - `chrono`: Date and time handling.
    - `uuid`, `nanoid`: Unique ID generation.
    - `url`: URL parsing and manipulation.
    - `base64`: Base64 encoding/decoding.
    - `tracing`, `tracing-subscriber`: Logging and application tracing.
    - `include_dir`: Embedding directories into the binary.
    - `indoc`: Formatting multiline strings.
    - `once_cell`, `lazy_static`: Global static initialization.
    - `etcetera`: Platform-specific directory locations.
- **LLM & AI Interaction**:
    - `tokenizers`: For processing text for LLMs.
    - `minijinja`: Templating engine (likely for prompts).
    - `aws-sdk-bedrockruntime`, `aws-config`: For AWS Bedrock LLM provider.
    - `jsonwebtoken`: For GCP Vertex AI provider authentication.
- **Networking & HTTP**:
    - `reqwest`: HTTP client.
- **FFI & Bindings**:
    - `uniffi`: Used in `goose-llm` for generating bindings (seen in `Justfile` for Kotlin example).
- **Security & Credentials**:
    - `keyring`: Secure credential storage.
- **Filesystem & OS**:
    - `fs2`: Filesystem utilities.
    - `shellexpand`: Shell-like variable expansion.
    - `nix` (CLI): Unix-specific operations (signals, processes).
- **Specialized MCP Libraries**:
    - `lopdf`, `docx-rs`, `image`, `umya-spreadsheet`: For file processing in `goose-mcp`.
    - `google-apis-common`, `google-drive3`, `google-sheets4`, `google-docs1`: Google Workspace integration.
    - `oauth2`: OAuth authentication.
    - `xcap`: Screen capture.
    - `kill_tree`: Process management.

### Async Runtime
- **Tokio**: (`1.43`) - Asynchronous runtime for Rust.

### Web Framework (Server Components)
- **Axum**: (`0.8.1`) - Web application framework for `goose-server` and web features in `goose-cli`.
    - `tower-http`: HTTP utility middleware.
    - `utoipa`: OpenAPI documentation generation.

### Serialization/Deserialization
- **Serde**: (`1.0`) - Framework for serializing and deserializing Rust data structures.
    - `serde_json`: JSON support.
    - `serde_yaml`: YAML support.
    - `serde_urlencoded`: URL-encoded data.

### Configuration
- `config` crate (`0.14.1`): For managing configuration in `goose-server`.
- `dotenv`: Loading environment variables from `.env` files.

### CLI Framework
- **Clap**: (`4.4`) - Command-line argument parser for `goose-cli`.
- **Cliclack**: (`0.3.5`) - CLI interactivity tools.
- **Console**: (`0.15.8`) - Terminal manipulation.
- **Bat**: (`0.24.0`) - Cat clone with syntax highlighting (for displaying files in CLI).
- **Rustyline**: (`15.0.0`) - Readline implementation for interactive CLI input.
- **Indicatif**: (`0.17.11`) - Progress bars.

### Database/Storage
- **LanceDB**: (`0.13`) - Vector database used in `goose` crate (likely for tool selection/semantic search).
    - `arrow`: (`52.2`) - Dependency for LanceDB.
- No traditional SQL/NoSQL database is explicitly listed for primary application data storage, but extensions could integrate with them.

## Frontend (Desktop UI & Documentation)

### Desktop UI (Current - `ui/desktop`)
- **Framework & Build Tools**:
    - **Electron**: (`33.1.0`) - Framework for building cross-platform desktop apps with web technologies.
        - `@electron-forge/cli`: (`^7.5.0`) - Build and packaging tool for Electron.
        - Makers: `maker-deb`, `maker-rpm`, `maker-squirrel`, `maker-zip`.
        - Plugins: `plugin-auto-unpack-natives`, `plugin-fuses`, `plugin-vite`.
    - **React**: (`^18.3.1`) - JavaScript library for building user interfaces.
    - **Vite**: (`^6.3.4`) - Build tool for modern web projects.
        - `@vitejs/plugin-react`: (`^4.3.3`)
    - **TypeScript**: (`~5.5.0`)
    - **Node.js**: (`^23.0.0` engine specified, Hermit likely manages a specific version e.g. `22.9.0` from `bin/.node-22.9.0.pkg`)
    - **Express**: (`^4.21.1`) - Used internally by the Electron app, likely for communication with the Rust backend.
- **Key Libraries**:
    - **Styling**:
        - `tailwindcss`: (`^3.4.14`) - Utility-first CSS framework.
        - `postcss`: (`^8.4.47`), `autoprefixer`: (`^10.4.20`)
    - **UI Components & Utilities**:
        - `@radix-ui/*`: Various headless UI components.
        - `@radix-ui/themes`: Theming.
        - `lucide-react`: Icons.
        - `framer-motion`: Animations.
        - `react-markdown`, `remark-gfm`: Markdown rendering.
        - `react-syntax-highlighter`: Code syntax highlighting.
        - `clsx`, `tailwind-merge`: Class name utilities.
    - **API Client & State**:
        - `@hey-api/openapi-ts`, `@hey-api/client-fetch`: OpenAPI client generation.
        - `ai` (Vercel AI SDK), `@ai-sdk/openai`, `@ai-sdk/ui-utils`: AI-related UI components and utilities.
    - **Testing**:
        - `@playwright/test`: (`^1.51.1`) - End-to-end testing.
    - **Linting & Formatting**:
        - `eslint`, `@typescript-eslint/eslint-plugin`, `eslint-plugin-react`: Linting.
        - `prettier`: Code formatting.

### Desktop UI (V2 - `ui-v2`)
- **Framework & Build Tools**:
    - **Electron**: (`^36.2.1`)
        - `@electron-forge/cli`: (`^7.8.1`)
        - `@electron-forge/plugin-vite`: (`^7.8.1`)
    - **React**: (`^19.1.0`)
    - **Vite**: (`^6.3.5`)
        - `@vitejs/plugin-react`: (`^4.4.1`)
    - **TypeScript**: (`^5.8.3`)
    - **SolidJS**: (Not explicitly in `package.json` but `solid-js` is often associated with `ui-v2` mentions, needs confirmation if used) - If SolidJS is used, it's not listed in this `package.json`.
- **Key Libraries**:
    - **Routing**: `@tanstack/react-router`
    - **Styling**: `tailwindcss` (`^4.1.7`), `postcss`, `autoprefixer`.
    - **UI Components**: `lucide-react`, `recharts`.
    - **Testing**: `@playwright/test`, `vitest` (unit/component testing).
    - **Linting & Formatting**: `eslint`, `stylelint`, `prettier`.

### Documentation Site (`documentation/`)
- **Framework**:
    - **Docusaurus**: (`3.7.0`) - Static site generator.
- **Key Libraries**:
    - **React**: (`^19.0.0`)
    - **MDX**: (`@mdx-js/react: ^3.0.0`) - Markdown with JSX.
    - **Styling**: `tailwindcss` (`^3.4.1`), `postcss`.
    - **Search**: `@inkeep/docusaurus` (Inkeep search integration).
    - **UI/UX**: `framer-motion`, `lucide-react`, `swiper`.
    - **TypeScript**: (`~5.6.2`)

## Bindings
- **UniFFI**: Used for generating language bindings from Rust (e.g., for Kotlin, Python).
    - `uniffi-bindgen` is run via `cargo run --features=uniffi/cli --bin uniffi-bindgen generate ...` (from `Justfile`).

## Build System & Task Runners
- **Cargo**: Rust's build system and package manager.
- **Just (`Justfile`)**: Command runner for various development and build tasks.
- **npm/yarn**: Package managers for Node.js projects (UI and documentation).
    - `npm` is used in `ui/desktop` and `ui-v2`.
    - `yarn` is used in `documentation/`.
- **Docker**: Used for cross-compiling Windows builds (`release-windows` task in `Justfile`).
- **`Cross.toml`**: Suggests use of `cross` for cross-compilation, though `Justfile` shows direct Docker usage for Windows.

## Development Environment & Tooling
- **Hermit (`bin/hermit.hcl`, `bin/activate-hermit`)**: Manages development environment and tool versions (e.g., Node.js, protoc, Rust).
    - Specific package versions managed by Hermit (from `bin/`):
        - Node.js `22.9.0`
        - Protobuf Compiler `31.1`
        - Rustup `1.25.2`
- **Git**: Version control.
- **Husky (`.husky/`)**: Git hooks for pre-commit checks (e.g., linting, formatting).
- **Lint-staged**: Runs linters on staged files.
- **Protobuf Compiler (`protoc`)**: Used for Protocol Buffers, likely for gRPC or other IPC mechanisms if not directly for LLM provider APIs.

## CI/CD
- **GitHub Actions (`.github/workflows/`)**:
    - `ci.yml`: Main CI workflow.
        - Checks Rust code format (`cargo fmt --check`).
        - Builds and tests Rust project (`cargo test`).
        - Lints Rust code (`cargo clippy`).
        - Lints Electron Desktop App (`npm run lint:check` in `ui/desktop`).
        - Caches Cargo registry, index, and build artifacts.
        - Uses `ubuntu-latest` and `macos-latest` runners.
        - Installs dependencies like `libdbus-1-dev`, `gnome-keyring`.
    - Other workflows for releases, bundling, PR comments, etc.

## External Services & APIs (Integrations)
- **LLM Providers**: (Configured by user, supported by `goose` crate)
    - OpenAI, Anthropic, AWS Bedrock, GCP Vertex AI, etc. (over 15 supported).
- **Google Workspace**:
    - Google Drive, Sheets, Docs (via `google-drive3`, `google-sheets4`, `google-docs1` crates in `goose-mcp`).
- **Langfuse**: (Optional, for tracing/observability)
    - `scripts/langfuse-docker-compose.yaml` and `scripts/setup_langfuse.sh` suggest integration.
- **Other potential integrations via MCP extensions**: The MCP system is designed to allow connections to various other tools and services.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
