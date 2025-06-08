---
Version: 1.0.24
Last Generated: 2025-06-08
---

# Database Schema

## Table of Contents
- [Overview](#overview)
- [LanceDB for Tool Vector Storage](#lancedb-for-tool-vector-storage)
  - [Purpose](#purpose)
  - [Storage Location](#storage-location)
  - [Data Structure (`ToolRecord`)](#data-structure-toolrecord)
  - [Arrow Schema](#arrow-schema)
  - [Key Operations](#key-operations)
- [Other Data Persistence](#other-data-persistence)

## Overview
`codename goose` does not use a traditional relational SQL database for its primary operations. Instead, it primarily utilizes an embedded vector database, **LanceDB**, for a specific purpose: storing and querying tool embeddings to facilitate tool routing and selection. Other data, like session history and configuration, is stored as files on the filesystem.

## LanceDB for Tool Vector Storage
The component responsible for this is `crates/goose/src/agents/tool_vectordb.rs`.

### Purpose
LanceDB is used to store vector embeddings of tool descriptions and their schemas. This allows the agent to perform semantic searches for relevant tools based on a query vector, typically derived from the user's request or the agent's current task. This is part of the "Tool Router" functionality.

### Storage Location
- The LanceDB database is stored locally on the filesystem.
- Path (derived from `Xdg::data_dir()`):
    - Linux/macOS: `~/.local/share/goose/tool_db/`
    - Windows: `%APPDATA%\goose\tool_db\` (or similar, depends on XDG strategy on Windows)
- The default table name is `tools`, but can be customized (e.g., for testing).

### Data Structure (`ToolRecord`)
When tools are indexed, they are represented by the `ToolRecord` struct:
```rust
// From crates/goose/src/agents/tool_vectordb.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecord {
    pub tool_name: String,    // Name of the tool
    pub description: String,  // Description of what the tool does
    pub schema: String,       // JSON schema string for the tool's input parameters
    pub vector: Vec<f32>,     // Embedding vector (e.g., from OpenAI, 1536 dimensions)
}
```

### Arrow Schema
Internally, LanceDB uses Apache Arrow. The schema for the "tools" table is defined as follows:
- **`tool_name`**: `DataType::Utf8` (non-nullable) - The unique name of the tool.
- **`description`**: `DataType::Utf8` (non-nullable) - A textual description of the tool's functionality.
- **`schema`**: `DataType::Utf8` (non-nullable) - A JSON string representing the input schema of the tool.
- **`vector`**: `DataType::FixedSizeList(Field::new("item", DataType::Float32, true), 1536)` (non-nullable) - The fixed-size list of 1536 float32 values representing the tool's embedding. The dimension (1536) corresponds to OpenAI's embedding model dimension.

### Key Operations
- **`init_table()`**: Initializes the "tools" table if it doesn't exist with the defined Arrow schema.
- **`index_tools(tools: Vec<ToolRecord>)`**: Adds a batch of `ToolRecord`s to the LanceDB table.
- **`search_tools(query_vector: Vec<f32>, k: usize)`**: Performs a vector similarity search against the indexed tools and returns the top `k` matching `ToolRecord`s (vector field in returned records is empty).
- **`remove_tool(tool_name: &str)`**: Deletes a tool from the database by its name.
- **`clear_tools()`**: (Used in tests) Deletes all records from the table.

## Other Data Persistence
- **Session History**: Stored as JSONL files (one message per line) in `~/.local/share/goose/sessions/<session_id>.jsonl` (path may vary slightly based on OS and XDG strategy). The first line of each session file is a JSON object containing session metadata.
- **Configuration**: Stored in `~/.config/goose/config.yaml`.
- **Secrets/API Keys**: Stored in the system keychain by default, or in `~/.config/goose/secrets.yaml` if keyring is disabled.
- **Logs**: Stored in date-based subdirectories under `~/.local/state/goose/logs/cli/` and `~/.local/state/goose/logs/server/`.

These file-based storages do not have a formal "database schema" but their structure is dictated by the serialization formats (JSON, YAML) of the respective data types in the Rust code.

---
*Note: This document was auto-generated based on static analysis of the codebase. While efforts have been made to ensure accuracy, it should be reviewed and validated by developers familiar with the project. Specific details, especially regarding internal logic or upcoming changes, might require further updates.*
