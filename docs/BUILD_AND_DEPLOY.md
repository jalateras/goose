---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Build and Deployment

## Table of Contents
- [Overview](#overview)
- [Development Environment Setup](#development-environment-setup)
  - [Prerequisites](#prerequisites)
  - [Hermit Environment](#hermit-environment)
  - [Manual Setup](#manual-setup)
- [Build Tools and Scripts](#build-tools-and-scripts)
  - [Just (`Justfile`)](#just-justfile)
  - [Cargo (Rust)](#cargo-rust)
  - [npm/yarn (Node.js)](#npmyarn-nodejs)
  - [Docker](#docker)
- [Building the Project](#building-the-project)
  - [Building Core Binaries (CLI & Server - `goosed`)](#building-core-binaries-cli--server---goosed)
  - [Cross-Compilation](#cross-compilation)
    - [Windows (via Docker)](#windows-via-docker)
    - [Intel Macs (from Apple Silicon or vice-versa)](#intel-macs-from-apple-silicon-or-vice-versa)
    - [Linux (using `cross`)](#linux-using-cross)
  - [Building the Desktop Application (`ui/desktop/`)](#building-the-desktop-application-uidesktop)
  - [Building the Documentation Site (`documentation/`)](#building-the-documentation-site-documentation)
  - [Building FFI Libraries (`goose-llm`, `goose-ffi`)](#building-ffi-libraries-goose-llm-goose-ffi)
- [Running Components Locally](#running-components-locally)
- [Environment Configuration](#environment-configuration)
  - [LLM Providers & API Keys](#llm-providers--api-keys)
  - [Goose Configuration Files](#goose-configuration-files)
  - [Lead/Worker Model Configuration](#leadworker-model-configuration)
- [CI/CD Pipeline (GitHub Actions)](#cicd-pipeline-github-actions)
  - [Main CI (`ci.yml`)](#main-ci-ciyml)
  - [Release Workflow (`release.yml`)](#release-workflow-releaseyml)
  - [Desktop Bundling Workflows](#desktop-bundling-workflows)
- [Deployment Procedures (Release Process)](#deployment-procedures-release-process)
  - [Tagging](#tagging)
  - [GitHub Release](#github-release)
  - [Distribution Assets](#distribution-assets)

## Overview
This document outlines the procedures for setting up the development environment, building the various components of `codename goose`, and the deployment process. The project uses `Justfile` as a primary task runner, `cargo` for Rust components, `npm`/`yarn` for Node.js components, and GitHub Actions for CI/CD.

## Development Environment Setup

### Prerequisites
- **Git**: For cloning the repository.
- **Rust Toolchain**: Install via [rustup.rs](https://rustup.rs/). The specific version is managed by `rust-toolchain.toml` (stable channel).
- **Node.js**: Required for UI development (`ui/desktop`, `ui-v2`) and the documentation site (`documentation/`). Version is managed by Hermit (e.g., 22.9.0).
- **Docker**: Required for cross-compiling Windows builds locally and potentially for other containerized development or testing (see `documentation/docs/guides/goose-in-docker.md`).
- **Hermit (Recommended)**: For managing project-specific tool versions.
- **Platform-Specific Build Tools**:
    - **Linux**: `build-essential` (gcc, make), `libdbus-1-dev`, `gnome-keyring`, `libxcb1-dev` (for CI environment, good indicators for local setup). `pkg-config`, `libssl-dev`.
    - **macOS**: Xcode Command Line Tools.
    - **Windows**: MSVC build tools (if building natively) or WSL for Linux-based build approach.

### Hermit Environment
The project uses Hermit to manage versions of tools like Node.js, protoc, and Rust itself.
- **Activation**:
  ```bash
  ./bin/activate-hermit
  # For fish shell:
  # ./bin/activate-hermit.fish
  ```
- This will install and activate the tool versions specified in `bin/hermit.hcl` and other Hermit configuration files.

### Manual Setup
If not using Hermit:
- Ensure Rust stable is installed.
- Install Node.js (e.g., v22.x).
- Install `protoc` (protobuf compiler, e.g., v31.1).
- Ensure `just` is installed (`cargo install just`).

## Build Tools and Scripts

### Just (`Justfile`)
`Justfile` is the main command runner. It provides recipes for common tasks.
- **List all tasks**: `just`
- **Key `just` commands are covered in relevant sections below.**

### Cargo (Rust)
Rust's package manager and build tool.
- **Build debug**: `cargo build`
- **Build release**: `cargo build --release`
- **Run tests**: `cargo test`
- **Build specific package**: `cargo build -p <crate_name>`

### npm/yarn (Node.js)
- **`ui/desktop` and `ui-v2` use `npm`**:
  - Install dependencies: `cd ui/desktop && npm install`
  - Build: `cd ui/desktop && npm run package` (or specific `bundle:*` scripts)
- **`documentation/` uses `yarn`**:
  - Install dependencies: `cd documentation && yarn`
  - Build: `cd documentation && yarn build`

### Docker
Used for:
- Cross-compiling Windows executables (see `just release-windows`).
- Potentially for isolated development environments as described in `documentation/docs/guides/goose-in-docker.md`.
- `Cross.toml` defines configurations for `cross`, a tool that uses Docker for cross-compilation, targeting various Linux architectures.

## Building the Project

### Building Core Binaries (CLI & Server - `goosed`)
The primary backend binary is `goosed` (from `goose-server`), and the CLI is `goose` (from `goose-cli`).
- **Standard Release Build (native platform)**:
  ```bash
  just release-binary
  ```
  This command:
    1. Runs `cargo build --release` (builds all Rust crates).
    2. Copies the `goosed` binary to `ui/desktop/src/bin/`.
    3. Generates OpenAPI schema using `cargo run -p goose-server --bin generate_schema`.
  The `goose` CLI binary will be in `target/release/goose`.
  The `goosed` server binary will be in `target/release/goosed`.

- **Debug Build**:
  ```bash
  cargo build
  # To copy debug goosed for UI dev:
  # just copy-binary debug
  ```

### Cross-Compilation

#### Windows (via Docker)
- **Command**:
  ```bash
  just release-windows
  ```
- **Process**: Uses a Docker container with `rust:latest` image, installs `mingw-w64`, and builds the `x86_64-pc-windows-gnu` target. Copies necessary DLLs.
- **Output**: `target/x86_64-pc-windows-gnu/release/goosed.exe` and DLLs.

#### Intel Macs (from Apple Silicon or vice-versa)
- **Command**:
  ```bash
  just release-intel
  ```
- **Process**: Runs `cargo build --release --target x86_64-apple-darwin`.
- **Output**: `target/x86_64-apple-darwin/release/goosed`.

#### Linux (using `cross`)
- The `Cross.toml` file is configured for `cross build` targeting `aarch64-unknown-linux-gnu` and `x86_64-unknown-linux-gnu`.
- `run_cross_local.md` provides instructions:
  ```bash
  # Example for aarch64-unknown-linux-gnu
  CROSS_BUILD_OPTS="--platform linux/amd64 --no-cache" CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --release --target aarch64-unknown-linux-gnu
  ```
- The `pre-build` steps in `Cross.toml` handle installing dependencies like `libssl-dev`, `libdbus-1-dev`, `libxcb1-dev`, and `protoc` within the Docker build environment.

### Building the Desktop Application (`ui/desktop/`)
The desktop application bundles the `goosed` binary.
- **Prerequisites**: Build `goosed` first (e.g., `just release-binary`).
- **General Build/Packaging (macOS Arm64 default)**:
  ```bash
  cd ui/desktop
  npm install
  npm run make # Uses electron-forge to make the application
  # For a distributable .zip:
  # npm run bundle:default
  ```
- **Windows Package**:
  Requires `goosed.exe` built via `just release-windows` and copied.
  ```bash
  just make-ui-windows # This Justfile recipe handles building goosed, copying, and then running `npm run bundle:windows` in `ui/desktop`.
  ```
- **Intel Mac Package**:
  Requires `goosed` (Intel) built via `just release-intel` and copied.
  ```bash
  just make-ui-intel # This Justfile recipe handles building, copying, and then `npm run bundle:intel`.
  ```
- **Signing**: Signing is handled by GitHub Actions workflows for official releases. For local builds, they will generally be unsigned unless macOS keychain is configured similarly.

### Building the Documentation Site (`documentation/`)
- **Command**:
  ```bash
  cd documentation
  yarn install
  yarn build
  ```
- **Output**: Static files in `documentation/build/`.

### Building FFI Libraries (`goose-llm`, `goose-ffi`)
- **`goose-llm` (for Kotlin/Python bindings)**:
  ```bash
  # Build the Rust library (e.g., debug for dylib)
  cargo build -p goose-llm
  # Generate bindings (example for Kotlin)
  cargo run --features=uniffi/cli --bin uniffi-bindgen generate --library ./target/debug/libgoose_llm.dylib --language kotlin --out-dir bindings/kotlin
  ```
  (See `just kotlin-example` for a full flow).
- **`goose-ffi` (C-bindings)**:
  ```bash
  cargo build --package goose-ffi # Debug
  cargo build --release --package goose-ffi # Release
  ```
  This generates the dynamic library and `include/goose_ffi.h`.

## Running Components Locally
- **CLI**:
  ```bash
  # After `cargo build`
  ./target/debug/goose session start
  # Or release
  # ./target/release/goose session start
  ```
- **Desktop UI (with release `goosed`)**:
  ```bash
  just run-ui
  ```
  This first runs `just release-binary`, then `cd ui/desktop && npm install && npm run start-gui`.
- **Desktop UI (with debug `goosed`)**:
  ```bash
  just run-dev
  ```
- **`goosed` Server Standalone**:
  ```bash
  just run-server # Runs `cargo run -p goose-server`
  ```
- **Documentation Site**:
  ```bash
  just run-docs # Runs `cd documentation && yarn && yarn start`
  ```

## Environment Configuration

### LLM Providers & API Keys
- Usually set via environment variables (e.g., `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`).
- The `goose configure` command (CLI) or Desktop UI settings can be used to set these.
- For Dockerized environments (`documentation/docs/guides/goose-in-docker.md`), these are passed as environment variables in `docker-compose.yml`.

### Goose Configuration Files
- Main configuration: `~/.config/goose/config.yaml`. Shared between CLI and Desktop UI.
- Contains provider settings, model selection, extension configurations.
- `.goosehints` file in project directories can provide context/instructions to Goose.
- `.env` files can be used for local environment variable overrides (loaded by `dotenv` crate).

### Lead/Worker Model Configuration
- `GOOSE_LEAD_MODEL`, `GOOSE_LEAD_PROVIDER`: For the lead model pattern.
- `GOOSE_PLANNER_MODEL`, `GOOSE_PLANNER_PROVIDER`: For the planning model.

## CI/CD Pipeline (GitHub Actions)

### Main CI (`ci.yml`)
- **Triggers**: Push/PR to `main`, merge groups, manual dispatch.
- **Jobs**:
    - `rust-format`: Checks `cargo fmt --check`.
    - `rust-build-and-test`: Builds all Rust crates and runs `cargo test`. Uses caching for Cargo registry, index, and build artifacts. Installs Linux dependencies like `libdbus-1-dev`, `gnome-keyring`.
    - `desktop-lint`: Lints the `ui/desktop` Electron app (`npm run lint:check`).
    - `bundle-desktop-unsigned`: (On PRs) Calls reusable workflow `bundle-desktop.yml` without signing.

### Release Workflow (`release.yml`)
- **Triggers**: Push of tags matching `v1.*`.
- **Jobs**:
    - `build-cli`: Calls reusable workflow `build-cli.yml` (not provided, but implied to build CLI for multiple OS/arch).
    - `install-script`: Uploads `download_cli.sh` artifact.
    - `bundle-desktop`: Calls `bundle-desktop.yml` for macOS Arm64, with signing.
    - `bundle-desktop-intel`: Calls `bundle-desktop-intel.yml` for macOS Intel, with signing.
    - `bundle-desktop-windows`: (Commented out in provided file) Would call `bundle-desktop-windows.yml`.
    - `release`: Downloads all artifacts and uses `ncipollo/release-action` to create/update a versioned GitHub release and a `stable` tag release.

### Desktop Bundling Workflows
- **`bundle-desktop.yml` (macOS Arm64)**:
    - Reusable workflow.
    - Checks out code, can update version based on input.
    - Caches Cargo artifacts.
    - Builds `goosed` (`cargo build --release -p goose-server`).
    - Copies `goosed` to `ui/desktop/src/bin/`.
    - Optionally adds macOS certs for signing (if `inputs.signing` is true and secrets are present).
    - Installs npm dependencies for `ui/desktop`.
    - Runs `npm run bundle:default` (which calls `electron-forge make` and zips).
    - Uploads `Goose-darwin-arm64/Goose.zip`.
    - Optionally runs a quick launch test.
- **`bundle-desktop-intel.yml` (macOS Intel)**:
    - Similar to `bundle-desktop.yml` but targets `x86_64-apple-darwin` for `goosed`.
    - Modifies `ui/desktop/package.json` to set build architecture to `x64`.
    - Runs `npm run bundle:intel`.
    - Uploads `Goose-darwin-x64/Goose_intel_mac.zip`.
- **`bundle-desktop-windows.yml` (Windows x64)**:
    - Reusable workflow.
    - Runs on `ubuntu-latest` for cross-compilation.
    - Builds `goosed.exe` for `x86_64-pc-windows-gnu` using Docker (similar to `just release-windows`).
    - Copies `goosed.exe` and DLLs to `ui/desktop/src/bin/`.
    - Installs npm dependencies and runs `npm run bundle:windows` in `ui/desktop`.
    - Uploads `desktop-windows-dist` artifact containing `ui/desktop/out/Goose-win32-x64/`.

## Deployment Procedures (Release Process)

The release process is primarily automated via GitHub Actions, triggered by tagging.

### Tagging
1. **Ensure `main` branch is up-to-date and stable.** (`just ensure-main`)
2. **Determine new version number** (semver).
3. **Create and checkout a release branch**: `git switch -c "release/{{version}}"`.
4. **Update versions**:
   ```bash
   just release {{version_number}}
   ```
   This Justfile recipe:
     - Validates the version.
     - Updates `Cargo.toml` workspace version.
     - Updates `ui/desktop/package.json` version.
     - Runs `cargo update --workspace` to update `Cargo.lock`.
     - Commits these changes.
5. **Merge the release branch to `main`.**
6. **Tag the `main` branch**:
   ```bash
   # On main branch after merge
   just tag # Creates tag like vX.Y.Z based on Cargo.toml version
   ```
7. **Push the tag to origin**:
   ```bash
   just tag-push # Pushes the tag, e.g., git push origin vX.Y.Z
   ```
   This push triggers the `.github/workflows/release.yml` workflow.

### GitHub Release
The `release.yml` workflow handles creating the GitHub release:
- It builds CLI binaries for various platforms, desktop apps for macOS (Arm64, Intel) and Windows (Windows bundling currently commented out in release workflow).
- It uploads these as artifacts to the GitHub Release.
- Two releases are managed:
    - A versioned release (e.g., `v1.2.3`).
    - A `stable` tag that points to the latest versioned release.

### Distribution Assets
The typical assets attached to a GitHub release include:
- CLI binaries (e.g., `goose-*.tar.bz2`).
- macOS Desktop app: `Goose-darwin-arm64.zip`, `Goose-darwin-x64.zip` (Intel).
- Windows Desktop app: (e.g., `desktop-windows-dist` containing the `.exe` and associated files, once enabled).
- `download_cli.sh`: Script for users to download and install the CLI.
- Potentially `.AppImage` or `.deb`/`.rpm` for Linux desktop if that were added.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
