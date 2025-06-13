// tests/providers_extract.rs

use anyhow::Result;
use dotenv::dotenv;
use goose::message::{Message, MessageContent, Role};
use goose::model::{CLAUDE_TOKENIZER, GPT_4O_TOKENIZER}; // Added CLAUDE_TOKENIZER
use goose::token_counter::TokenCounter;
use goose_llm::model::ModelConfig;
use goose_llm::providers::base::Provider;
use goose_llm::providers::databricks::{
    DatabricksProvider, DatabricksProviderConfig, DATABRICKS_DEFAULT_MODEL,
}; // Full import for new tests
use goose_llm::providers::openai::{OpenAiProvider, OpenAiProviderConfig, OPEN_AI_DEFAULT_MODEL};
use goose_llm::providers::utils::ImageFormat as ProviderImageFormat; // Added for Databricks config
use goose_llm::usage_tracker::TokenUsageTracker;
use httpmock::prelude::*;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, PartialEq, Copy, Clone)]
enum ProviderType {
    OpenAi,
    Databricks,
}

impl ProviderType {
    fn required_env(&self) -> &'static [&'static str] {
        match self {
            ProviderType::OpenAi => &["OPENAI_API_KEY"],
            ProviderType::Databricks => &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        }
    }

    fn create_provider(&self, cfg: ModelConfig) -> Result<Arc<dyn Provider>> {
        Ok(match self {
            ProviderType::OpenAi => Arc::new(OpenAiProvider::from_env(cfg)),
            ProviderType::Databricks => Arc::new(DatabricksProvider::from_env(cfg)),
        })
    }
}

fn check_required_env_vars(required: &[&str]) -> bool {
    let missing: Vec<_> = required
        .iter()
        .filter(|&&v| std::env::var(v).is_err())
        .cloned()
        .collect();
    if !missing.is_empty() {
        println!("Skipping test; missing env vars: {:?}", missing);
        false
    } else {
        true
    }
}

// --- Shared inputs for "paper" task ---
const PAPER_SYSTEM: &str =
    "You are an expert at structured data extraction. Extract the metadata of a research paper into JSON.";
const PAPER_TEXT: &str =
    "Application of Quantum Algorithms in Interstellar Navigation: A New Frontier \
     by Dr. Stella Voyager, Dr. Nova Star, Dr. Lyra Hunter. Abstract: This paper \
     investigates the utilization of quantum algorithms to improve interstellar \
     navigation systems. Keywords: Quantum algorithms, interstellar navigation, \
     space-time anomalies, quantum superposition, quantum entanglement, space travel.";

fn paper_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title":    { "type": "string" },
            "authors":  { "type": "array",  "items": { "type": "string" } },
            "abstract": { "type": "string" },
            "keywords": { "type": "array",  "items": { "type": "string" } }
        },
        "required": ["title","authors","abstract","keywords"],
        "additionalProperties": false
    })
}

// --- Shared inputs for "UI" task ---
const UI_SYSTEM: &str = "You are a UI generator AI. Convert the user input into a JSON-driven UI.";
const UI_TEXT: &str = "Make a User Profile Form";

fn ui_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["div","button","header","section","field","form"]
            },
            "label":   { "type": "string" },
            "children": {
                "type": "array",
                "items": { "$ref": "#" }
            },
            "attributes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name":  { "type": "string" },
                        "value": { "type": "string" }
                    },
                    "required": ["name","value"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["type","label","children","attributes"],
        "additionalProperties": false
    })
}

/// Generic runner for any extract task
async fn run_extract_test<F>(
    provider_type: ProviderType,
    model: &str,
    system: &'static str,
    user_text: &'static str,
    schema: Value,
    validate: F,
) -> Result<()>
where
    F: Fn(&Value) -> bool,
{
    dotenv().ok();
    if !check_required_env_vars(provider_type.required_env()) {
        return Ok(());
    }

    let cfg = ModelConfig::new(model.to_string()).with_temperature(Some(0.0));
    let provider = provider_type.create_provider(cfg)?;

    let msg = Message::user().with_text(user_text);
    let resp = provider.extract(system, &[msg], &schema).await?;

    println!("[{:?}] extract => {}", provider_type, resp.data);

    assert!(
        validate(&resp.data),
        "{:?} failed validation on {}",
        provider_type,
        resp.data
    );
    Ok(())
}

/// Helper for the "paper" task
async fn run_extract_paper_test(provider: ProviderType, model: &str) -> Result<()> {
    run_extract_test(
        provider,
        model,
        PAPER_SYSTEM,
        PAPER_TEXT,
        paper_schema(),
        |v| {
            v.as_object()
                .map(|o| {
                    ["title", "authors", "abstract", "keywords"]
                        .iter()
                        .all(|k| o.contains_key(*k))
                })
                .unwrap_or(false)
        },
    )
    .await
}

/// Helper for the "UI" task
async fn run_extract_ui_test(provider: ProviderType, model: &str) -> Result<()> {
    run_extract_test(provider, model, UI_SYSTEM, UI_TEXT, ui_schema(), |v| {
        v.as_object()
            .and_then(|o| o.get("type").and_then(Value::as_str))
            == Some("form")
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_extract_paper() -> Result<()> {
        run_extract_paper_test(ProviderType::OpenAi, "gpt-4o").await
    }

    #[tokio::test]
    async fn openai_extract_ui() -> Result<()> {
        run_extract_ui_test(ProviderType::OpenAi, "gpt-4o").await
    }

    #[tokio::test]
    async fn databricks_extract_paper() -> Result<()> {
        run_extract_paper_test(ProviderType::Databricks, "goose-gpt-4-1").await
    }

    #[tokio::test]
    async fn databricks_extract_ui() -> Result<()> {
        run_extract_ui_test(ProviderType::Databricks, "goose-gpt-4-1").await
    }

    #[tokio::test]
    async fn test_openai_extract_records_api_token_usage() {
        let server = MockServer::start();
        let model_id = OPEN_AI_DEFAULT_MODEL.to_string();
        let prompt_tokens_from_api = 60u64;
        let completion_tokens_from_api = 120u64;

        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions"); // extract uses the same endpoint
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "id": "chatcmpl-ext-123",
                    "object": "chat.completion",
                    "created": 1677652288,
                    "model": model_id,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            // For extract, content is often a stringified JSON
                            "content": "{\"name\": \"Test Name\", \"value\": 123}",
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": {
                        "prompt_tokens": prompt_tokens_from_api,
                        "completion_tokens": completion_tokens_from_api,
                        "total_tokens": prompt_tokens_from_api + completion_tokens_from_api
                    }
                }));
        });

        let config = OpenAiProviderConfig {
            api_key: "test_key".to_string(),
            host: server.base_url(),
            organization: None,
            base_path: "v1/chat/completions".to_string(),
            project: None,
            custom_headers: None,
            timeout: 5,
        };

        let token_counter = Arc::new(TokenCounter::new(GPT_4O_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        let model_config = ModelConfig::new(model_id.clone()); // Already correctly using ModelConfig
        let provider = OpenAiProvider::from_config(
            config,
            model_config,
            Arc::clone(&token_counter),
            usage_tracker.clone(),
        )
        .unwrap();

        let system_prompt = "Extract information.";
        let messages = vec![Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::text("User data to extract from.")],
        }];
        let schema = json!({"type": "object", "properties": {"name": {"type": "string"}, "value": {"type": "number"}}});

        let _ = provider
            .extract(system_prompt, &messages, &schema)
            .await
            .unwrap();

        let usage = usage_tracker.get_usage(&model_id).unwrap();
        assert_eq!(usage.input_tokens, prompt_tokens_from_api);
        assert_eq!(usage.output_tokens, completion_tokens_from_api);
    }

    #[tokio::test]
    async fn test_openai_extract_records_calculated_token_usage() {
        let server = MockServer::start();
        let model_id = OPEN_AI_DEFAULT_MODEL.to_string();

        // This is the actual JSON object the model would "return" as a string
        let extracted_data_obj = json!({"name": "Calculated Test", "value": 456});
        // The model returns it as a string in the "content" field
        let content_string = extracted_data_obj.to_string();
        // Expected output tokens for `{"name": "Calculated Test", "value": 456}` with GPT-4o.
        // Manually checked with a tokenizer: `{"name":"Calculated Test","value":456}` is 13 tokens.
        let expected_output_tokens = 13u64;

        server.mock(|when, then| {
            when.method(POST).path("/v1/chat/completions");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "id": "chatcmpl-ext-456",
                    "object": "chat.completion",
                    "created": 1677652288,
                    "model": model_id,
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": content_string, // Model returns stringified JSON
                        },
                        "finish_reason": "stop"
                    }],
                    "usage": { // API provides zero tokens
                        "prompt_tokens": 0,
                        "completion_tokens": 0,
                        "total_tokens": 0
                    }
                }));
        });

        let config = OpenAiProviderConfig {
            api_key: "test_key".to_string(),
            host: server.base_url(),
            organization: None,
            base_path: "v1/chat/completions".to_string(),
            project: None,
            custom_headers: None,
            timeout: 5,
        };

        let token_counter = Arc::new(TokenCounter::new(GPT_4O_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        let model_config = ModelConfig::new(model_id.clone()); // Already correctly using ModelConfig
        let provider = OpenAiProvider::from_config(
            config,
            model_config,
            Arc::clone(&token_counter),
            usage_tracker.clone(),
        )
        .unwrap();

        let system_prompt = "Extract data."; // 3 tokens GPT-4o
        let messages = vec![Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::text("Some user input.")], // 4 tokens
        }];
        // Expected input: count_chat_tokens("Extract data.", messages, []) (tools are empty for extract)
        // System: "Extract data." (3) + 4 (tokens_per_message) = 7
        // User: "Some user input." (4) + 4 (tokens_per_message) = 8
        // Reply prime: 3
        // Total expected input = 7 + 8 + 3 = 18 tokens.
        let expected_input_tokens = 18u64;

        let schema = json!({"type": "object", "properties": {"name": {"type": "string"}, "value": {"type": "number"}}});

        let _ = provider
            .extract(system_prompt, &messages, &schema)
            .await
            .unwrap();

        let usage = usage_tracker.get_usage(&model_id).unwrap();
        assert_eq!(usage.input_tokens, expected_input_tokens);
        assert_eq!(usage.output_tokens, expected_output_tokens);
    }

    #[tokio::test]
    async fn test_databricks_extract_records_api_token_usage() {
        let server = MockServer::start();
        let model_name = DATABRICKS_DEFAULT_MODEL.to_string();
        let prompt_tokens_from_api = 75u64;
        let completion_tokens_from_api = 150u64;

        server.mock(|when, then| {
            when.method(POST)
                .path(format!("/serving-endpoints/{}/invocations", model_name));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": "{\"extracted_field\": \"Databricks API value\"}",
                        }
                    }],
                    "usage": {
                        "prompt_tokens": prompt_tokens_from_api,
                        "completion_tokens": completion_tokens_from_api,
                        "total_tokens": prompt_tokens_from_api + completion_tokens_from_api
                    }
                }));
        });

        let config = DatabricksProviderConfig {
            host: server.base_url(),
            token: "db_test_token".to_string(),
            image_format: ProviderImageFormat::OpenAi, // Or appropriate
            timeout: 5,
        };

        let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        let model_config =
            ModelConfig::new_with_name(model_name.clone(), model_name.clone());
        let provider = DatabricksProvider::from_config(
            config,
            model_config.clone(),
            Arc::clone(&token_counter),
            usage_tracker.clone(),
        )
        .unwrap();

        let system_prompt = "Extract from Databricks.";
        let messages = vec![Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::text(
                "Some text for Databricks extraction.",
            )],
        }];
        let schema =
            json!({"type": "object", "properties": {"extracted_field": {"type": "string"}}});

        let _ = provider
            .extract(system_prompt, &messages, &schema)
            .await
            .unwrap();

        let usage = usage_tracker.get_usage(&model_config.model_name).unwrap();
        assert_eq!(usage.input_tokens, prompt_tokens_from_api);
        assert_eq!(usage.output_tokens, completion_tokens_from_api);
    }

    #[tokio::test]
    async fn test_databricks_extract_records_calculated_token_usage() {
        let server = MockServer::start();
        let model_name = DATABRICKS_DEFAULT_MODEL.to_string();

        let extracted_data_obj = json!({"field": "Calculated Databricks Data"});
        let content_string = extracted_data_obj.to_string();

        server.mock(|when, then| {
            when.method(POST)
                .path(format!("/serving-endpoints/{}/invocations", model_name));
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({
                    "choices": [{
                        "index": 0,
                        "message": {
                            "role": "assistant",
                            "content": content_string,
                        }
                    }],
                    "usage": {
                        "prompt_tokens": 0,
                        "completion_tokens": 0,
                        "total_tokens": 0
                    }
                }));
        });

        let config = DatabricksProviderConfig {
            host: server.base_url(),
            token: "db_test_token".to_string(),
            image_format: ProviderImageFormat::OpenAi,
            timeout: 5,
        };

        let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        let model_config =
            ModelConfig::new_with_name(model_name.clone(), model_name.clone());
        let provider = DatabricksProvider::from_config(
            config,
            model_config.clone(),
            Arc::clone(&token_counter),
            usage_tracker.clone(),
        )
        .unwrap();

        let system_prompt = "System for Databricks extract."; // 5 tokens with Claude
        let messages = vec![Message {
            role: Role::User,
            created: 0,
            content: vec![MessageContent::text("User input to extract.")], // 5 tokens
        }];
        // Expected input with CLAUDE_TOKENIZER:
        // System: "System for Databricks extract." (5) + 4 = 9
        // User: "User input to extract." (5) + 4 = 9
        // Reply prime: 3
        // Total expected input = 9 + 9 + 3 = 21 tokens.
        let expected_input_tokens = 21u64;
        let expected_output_tokens = token_counter.count_tokens(&content_string) as u64;

        let schema = json!({"type": "object", "properties": {"field": {"type": "string"}}});
        let _ = provider
            .extract(system_prompt, &messages, &schema)
            .await
            .unwrap();

        let usage = usage_tracker.get_usage(&model_config.model_name).unwrap();
        assert_eq!(usage.input_tokens, expected_input_tokens);
        assert_eq!(usage.output_tokens, expected_output_tokens);
    }
}
