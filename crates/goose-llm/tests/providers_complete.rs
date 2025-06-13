use anyhow::Result;
use dotenv::dotenv;
use goose::message::{Message, MessageContent, Role};
use goose::model::{CLAUDE_TOKENIZER, GPT_4O_TOKENIZER}; // Added CLAUDE_TOKENIZER
use goose::token_counter::TokenCounter;
use goose_llm::model::ModelConfig;
use goose_llm::providers::base::Provider;
use goose_llm::providers::databricks::{
    DatabricksProvider, DatabricksProviderConfig, DATABRICKS_DEFAULT_MODEL,
}; // Added for new tests
use goose_llm::providers::errors::ProviderError;
use goose_llm::providers::openai::{OpenAiProvider, OpenAiProviderConfig, OPEN_AI_DEFAULT_MODEL};
use goose_llm::providers::utils::ImageFormat; // Added for Databricks config
use goose_llm::types::core::{Content, Tool};
use goose_llm::usage_tracker::TokenUsageTracker;
use httpmock::prelude::*;
use serde_json::json; // Added
use std::collections::HashMap;
use std::sync::Arc; // Already here
use std::sync::Mutex;
// Note: openai module itself is not directly used from goose_llm::providers::openai in existing tests,
// but the specific structs are now imported for the new tests.

#[derive(Debug, Clone, Copy)]
enum TestStatus {
    Passed,
    Skipped,
    Failed,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "✅"),
            TestStatus::Skipped => write!(f, "⏭️"),
            TestStatus::Failed => write!(f, "❌"),
        }
    }
}

struct TestReport {
    results: Mutex<HashMap<String, TestStatus>>,
}

impl TestReport {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            results: Mutex::new(HashMap::new()),
        })
    }

    fn record_status(&self, provider: &str, status: TestStatus) {
        let mut results = self.results.lock().unwrap();
        results.insert(provider.to_string(), status);
    }

    fn record_pass(&self, provider: &str) {
        self.record_status(provider, TestStatus::Passed);
    }

    fn record_skip(&self, provider: &str) {
        self.record_status(provider, TestStatus::Skipped);
    }

    fn record_fail(&self, provider: &str) {
        self.record_status(provider, TestStatus::Failed);
    }

    fn print_summary(&self) {
        println!("\n============== Providers ==============");
        let results = self.results.lock().unwrap();
        let mut providers: Vec<_> = results.iter().collect();
        providers.sort_by(|a, b| a.0.cmp(b.0));

        for (provider, status) in providers {
            println!("{} {}", status, provider);
        }
        println!("=======================================\n");
    }
}

lazy_static::lazy_static! {
    static ref TEST_REPORT: Arc<TestReport> = TestReport::new();
    static ref ENV_LOCK: Mutex<()> = Mutex::new(());
}

/// Generic test harness for any Provider implementation
struct ProviderTester {
    provider: Arc<dyn Provider>,
    name: String,
}

impl ProviderTester {
    fn new<T: Provider + Send + Sync + 'static>(provider: T, name: String) -> Self {
        Self {
            provider: Arc::new(provider),
            name,
        }
    }

    async fn test_basic_response(&self) -> Result<()> {
        let message = Message::user().with_text("Just say hello!");

        let response = self
            .provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await?;

        // For a basic response, we expect a single text response
        assert_eq!(
            response.message.content.len(),
            1,
            "Expected single content item in response"
        );

        // Verify we got a text response
        assert!(
            matches!(response.message.content[0], MessageContent::Text(_)),
            "Expected text response"
        );

        Ok(())
    }

    async fn test_tool_usage(&self) -> Result<()> {
        let weather_tool = Tool::new(
            "get_weather",
            "Get the weather for a location",
            serde_json::json!({
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                }
            }),
        );

        let message = Message::user().with_text("What's the weather like in San Francisco?");

        let response1 = self
            .provider
            .complete(
                "You are a helpful weather assistant.",
                &[message.clone()],
                &[weather_tool.clone()],
            )
            .await?;

        println!("=== {}::reponse1 ===", self.name);
        dbg!(&response1);
        println!("===================");

        // Verify we got a tool request
        assert!(
            response1
                .message
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolReq(_))),
            "Expected tool request in response"
        );

        let id = &response1
            .message
            .content
            .iter()
            .filter_map(|message| message.as_tool_request())
            .last()
            .expect("got tool request")
            .id;

        let weather = Message::user().with_tool_response(
            id,
            Ok(vec![Content::text(
                "
                  50°F°C
                  Precipitation: 0%
                  Humidity: 84%
                  Wind: 2 mph
                  Weather
                  Saturday 9:00 PM
                  Clear",
            )])
            .into(),
        );

        // Verify we construct a valid payload including the request/response pair for the next inference
        let response2 = self
            .provider
            .complete(
                "You are a helpful weather assistant.",
                &[message, response1.message, weather],
                &[weather_tool],
            )
            .await?;

        println!("=== {}::reponse2 ===", self.name);
        dbg!(&response2);
        println!("===================");

        assert!(
            response2
                .message
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::Text(_))),
            "Expected text for final response"
        );

        Ok(())
    }

    async fn test_context_length_exceeded_error(&self) -> Result<()> {
        // Google Gemini has a really long context window
        let large_message_content = if self.name.to_lowercase() == "google" {
            "hello ".repeat(1_300_000)
        } else {
            "hello ".repeat(300_000)
        };

        let messages = vec![
            Message::user().with_text("hi there. what is 2 + 2?"),
            Message::assistant().with_text("hey! I think it's 4."),
            Message::user().with_text(&large_message_content),
            Message::assistant().with_text("heyy!!"),
            // Messages before this mark should be truncated
            Message::user().with_text("what's the meaning of life?"),
            Message::assistant().with_text("the meaning of life is 42"),
            Message::user().with_text(
                "did I ask you what's 2+2 in this message history? just respond with 'yes' or 'no'",
            ),
        ];

        // Test that we get ProviderError::ContextLengthExceeded when the context window is exceeded
        let result = self
            .provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await;

        // Print some debug info
        println!("=== {}::context_length_exceeded_error ===", self.name);
        dbg!(&result);
        println!("===================");

        // Ollama truncates by default even when the context window is exceeded
        if self.name.to_lowercase() == "ollama" {
            assert!(
                result.is_ok(),
                "Expected to succeed because of default truncation"
            );
            return Ok(());
        }

        assert!(
            result.is_err(),
            "Expected error when context window is exceeded"
        );
        assert!(
            matches!(result.unwrap_err(), ProviderError::ContextLengthExceeded(_)),
            "Expected error to be ContextLengthExceeded"
        );

        Ok(())
    }

    /// Run all provider tests
    async fn run_test_suite(&self) -> Result<()> {
        self.test_basic_response().await?;
        self.test_tool_usage().await?;
        self.test_context_length_exceeded_error().await?;
        Ok(())
    }
}

fn load_env() {
    if let Ok(path) = dotenv() {
        println!("Loaded environment from {:?}", path);
    }
}

/// Helper function to run a provider test with proper error handling and reporting
async fn test_provider<F, T>(
    name: &str,
    required_vars: &[&str],
    env_modifications: Option<HashMap<&str, Option<String>>>,
    provider_fn: F,
) -> Result<()>
where
    F: FnOnce() -> T,
    T: Provider + Send + Sync + 'static,
{
    // We start off as failed, so that if the process panics it is seen as a failure
    TEST_REPORT.record_fail(name);

    // Take exclusive access to environment modifications
    let lock = ENV_LOCK.lock().unwrap();

    load_env();

    // Save current environment state for required vars and modified vars
    let mut original_env = HashMap::new();
    for &var in required_vars {
        if let Ok(val) = std::env::var(var) {
            original_env.insert(var, val);
        }
    }
    if let Some(mods) = &env_modifications {
        for &var in mods.keys() {
            if let Ok(val) = std::env::var(var) {
                original_env.insert(var, val);
            }
        }
    }

    // Apply any environment modifications
    if let Some(mods) = &env_modifications {
        for (&var, value) in mods.iter() {
            match value {
                Some(val) => std::env::set_var(var, val),
                None => std::env::remove_var(var),
            }
        }
    }

    // Setup the provider
    let missing_vars = required_vars.iter().any(|var| std::env::var(var).is_err());
    if missing_vars {
        println!("Skipping {} tests - credentials not configured", name);
        TEST_REPORT.record_skip(name);
        return Ok(());
    }

    let provider = provider_fn();

    // Restore original environment
    for (&var, value) in original_env.iter() {
        std::env::set_var(var, value);
    }
    if let Some(mods) = env_modifications {
        for &var in mods.keys() {
            if !original_env.contains_key(var) {
                std::env::remove_var(var);
            }
        }
    }

    std::mem::drop(lock);

    let tester = ProviderTester::new(provider, name.to_string());
    match tester.run_test_suite().await {
        Ok(_) => {
            TEST_REPORT.record_pass(name);
            Ok(())
        }
        Err(e) => {
            println!("{} test failed: {}", name, e);
            TEST_REPORT.record_fail(name);
            Err(e)
        }
    }
}

#[tokio::test]
async fn openai_complete() -> Result<()> {
    test_provider(
        "OpenAI",
        &["OPENAI_API_KEY"],
        None,
        openai::OpenAiProvider::default,
    )
    .await
}

#[tokio::test]
async fn databricks_complete() -> Result<()> {
    test_provider(
        "Databricks",
        &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        None,
        databricks::DatabricksProvider::default,
    )
    .await
}

// Print the final test report
#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}

#[tokio::test]
async fn test_openai_complete_records_api_token_usage() {
    let server = MockServer::start();

    let model_id = OPEN_AI_DEFAULT_MODEL.to_string();
    let prompt_tokens_from_api = 50;
    let completion_tokens_from_api = 100;

    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1677652288,
                "model": model_id,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Hello there!",
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
        base_path: "v1/chat/completions".to_string(), // Ensure this matches mock path
        project: None,
        custom_headers: None,
        timeout: 5,
    };

    let token_counter = Arc::new(TokenCounter::new(GPT_4O_TOKENIZER));
    let usage_tracker = TokenUsageTracker::new();
    let model_config = ModelConfig::new(model_id.clone());
    let provider = OpenAiProvider::from_config(
        config,
        model_config,
        Arc::clone(&token_counter),
        usage_tracker.clone(),
    )
    .unwrap();

    let system_prompt = "You are a helpful assistant.";
    let messages = vec![Message {
        role: Role::User, // Ensure Role::User is correct
        created: 0,
        content: vec![MessageContent::text("Hello")],
    }];

    let _ = provider
        .complete(system_prompt, &messages, &[])
        .await
        .unwrap();

    let usage = usage_tracker.get_usage(&model_id).unwrap();
    assert_eq!(usage.input_tokens, prompt_tokens_from_api as u64); // Cast to u64
    assert_eq!(usage.output_tokens, completion_tokens_from_api as u64); // Cast to u64
}

#[tokio::test]
async fn test_openai_complete_records_calculated_token_usage() {
    let server = MockServer::start();
    let model_id = OPEN_AI_DEFAULT_MODEL.to_string();

    server.mock(|when, then| {
        when.method(POST).path("/v1/chat/completions");
        then.status(200)
            .header("content-type", "application/json")
            .json_body(json!({
                "id": "chatcmpl-123",
                "object": "chat.completion",
                "created": 1677652288,
                "model": model_id,
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "This is a test response.", // 5 tokens with GPT-4o tokenizer
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
    let model_config = ModelConfig::new(model_id.clone());
    let provider = OpenAiProvider::from_config(
        config,
        model_config,
        Arc::clone(&token_counter),
        usage_tracker.clone(),
    )
    .unwrap();

    let system_prompt = "System prompt."; // 3 tokens with GPT-4o
    let messages = vec![Message {
        role: Role::User,
        created: 0,
        content: vec![MessageContent::text("User message.")], // 3 tokens
    }];
    // Expected input: count_chat_tokens("System prompt.", messages, [])
    // System: "System prompt." (3) + 4 (tokens_per_message) = 7
    // User: "User message." (3) + 4 (tokens_per_message) = 7
    // Reply prime: 3
    // Total expected input = 7 + 7 + 3 = 17 tokens.
    let expected_input_tokens = 17;
    let expected_output_tokens = 5; // "This is a test response."

    let _ = provider
        .complete(system_prompt, &messages, &[])
        .await
        .unwrap();

    let usage = usage_tracker.get_usage(&model_id).unwrap();
    assert_eq!(usage.input_tokens, expected_input_tokens);
    assert_eq!(usage.output_tokens, expected_output_tokens);
}

#[tokio::test]
async fn test_databricks_complete_records_api_token_usage() {
    let server = MockServer::start();
    let model_name = DATABRICKS_DEFAULT_MODEL.to_string(); // Used in URL path for Databricks
    let prompt_tokens_from_api = 70u64;
    let completion_tokens_from_api = 140u64;

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
                        "content": "Databricks response here.",
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
        image_format: ImageFormat::OpenAi, // Default or configure as needed
        timeout: 5,
    };

    let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
    let usage_tracker = TokenUsageTracker::new();
    // model_name for Databricks is the endpoint name / model identifier
    let model_config = ModelConfig::new_with_name(
        DATABRICKS_DEFAULT_MODEL.to_string(),
        DATABRICKS_DEFAULT_MODEL.to_string(),
    );
    let provider = DatabricksProvider::from_config(
        config,
        model_config.clone(),
        Arc::clone(&token_counter),
        usage_tracker.clone(),
    )
    .unwrap();

    let system_prompt = "You are a Databricks assistant.";
    let messages = vec![Message {
        role: Role::User,
        created: 0,
        content: vec![MessageContent::text("Query for Databricks.")],
    }];

    let _ = provider
        .complete(system_prompt, &messages, &[])
        .await
        .unwrap();

    // Usage is tracked by model_name in DatabricksProvider
    let usage = usage_tracker.get_usage(&model_config.model_name).unwrap();
    assert_eq!(usage.input_tokens, prompt_tokens_from_api);
    assert_eq!(usage.output_tokens, completion_tokens_from_api);
}

#[tokio::test]
async fn test_databricks_complete_records_calculated_token_usage() {
    let server = MockServer::start();
    let model_name = DATABRICKS_DEFAULT_MODEL.to_string();
    let response_text = "Calculated Databricks response."; // Approx 4 tokens with Claude (Sonnet)

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
                        "content": response_text,
                    }
                }],
                "usage": { // API provides zero tokens
                    "prompt_tokens": 0,
                    "completion_tokens": 0,
                    "total_tokens": 0
                }
            }));
    });

    let config = DatabricksProviderConfig {
        host: server.base_url(),
        token: "db_test_token".to_string(),
        image_format: ImageFormat::OpenAi,
        timeout: 5,
    };

    let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
    let usage_tracker = TokenUsageTracker::new();
    let model_config = ModelConfig::new_with_name(
        DATABRICKS_DEFAULT_MODEL.to_string(),
        DATABRICKS_DEFAULT_MODEL.to_string(),
    );
    let provider = DatabricksProvider::from_config(
        config,
        model_config.clone(),
        Arc::clone(&token_counter),
        usage_tracker.clone(),
    )
    .unwrap();

    let system_prompt = "System for Databricks."; // 4 tokens with Claude
    let messages = vec![Message {
        role: Role::User,
        created: 0,
        content: vec![MessageContent::text("User query.")], // 3 tokens
    }];
    // Expected input with CLAUDE_TOKENIZER:
    // System: "System for Databricks." (4) + 4 (tokens_per_message) = 8
    // User: "User query." (3) + 4 (tokens_per_message) = 7
    // Reply prime: 3
    // Total expected input = 8 + 7 + 3 = 18 tokens.
    let expected_input_tokens = 18u64;
    let expected_output_tokens = token_counter.count_tokens(response_text) as u64;

    let _ = provider
        .complete(system_prompt, &messages, &[])
        .await
        .unwrap();

    let usage = usage_tracker.get_usage(&model_config.model_name).unwrap();
    assert_eq!(usage.input_tokens, expected_input_tokens);
    assert_eq!(usage.output_tokens, expected_output_tokens);
}
