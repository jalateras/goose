use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    errors::ProviderError,
    formats::openai::{create_request, get_usage, response_to_message},
    utils::{emit_debug_trace, get_env, get_model, handle_response_openai_compat, ImageFormat},
};
use crate::{
    message::Message,
    model::ModelConfig,
    providers::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage},
    types::core::Tool,
    usage_tracker::TokenUsageTracker,
};
use goose::{model::GPT_4O_TOKENIZER, token_counter::TokenCounter};

pub const OPEN_AI_DEFAULT_MODEL: &str = "gpt-4o";
pub const _OPEN_AI_KNOWN_MODELS: &[&str] = &["gpt-4o", "gpt-4.1", "o1", "o3", "o4-mini"];

fn default_timeout() -> u64 {
    60
}

fn default_base_path() -> String {
    "v1/chat/completions".to_string()
}

fn default_host() -> String {
    "https://api.openai.com".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderConfig {
    pub api_key: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default = "default_base_path")]
    pub base_path: String,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub custom_headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    pub timeout: u64, // timeout in seconds
}

impl OpenAiProviderConfig {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            host: default_host(),
            organization: None,
            base_path: default_base_path(),
            project: None,
            custom_headers: None,
            timeout: 600,
        }
    }

    pub fn from_env() -> Self {
        let api_key = get_env("OPENAI_API_KEY").expect("Missing OPENAI_API_KEY");
        Self::new(api_key)
    }
}

#[derive(Debug)]
pub struct OpenAiProvider {
    config: OpenAiProviderConfig,
    model: ModelConfig,
    client: Client,
    token_counter: Arc<TokenCounter>,
    usage_tracker: TokenUsageTracker,
}

impl OpenAiProvider {
    pub fn from_env(model: ModelConfig) -> Self {
        let config = OpenAiProviderConfig::from_env();
        let token_counter = Arc::new(TokenCounter::new(GPT_4O_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        OpenAiProvider::from_config(config, model, token_counter, usage_tracker)
            .expect("Failed to initialize OpenAiProvider")
    }
}

impl Default for OpenAiProvider {
    fn default() -> Self {
        let config = OpenAiProviderConfig::from_env();
        let model = ModelConfig::new(OPEN_AI_DEFAULT_MODEL.to_string());
        let token_counter = Arc::new(TokenCounter::new(GPT_4O_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        OpenAiProvider::from_config(config, model, token_counter, usage_tracker)
            .expect("Failed to initialize OpenAiProvider")
    }
}

impl OpenAiProvider {
    pub fn from_config(
        config: OpenAiProviderConfig,
        model: ModelConfig,
        token_counter: Arc<TokenCounter>,
        usage_tracker: TokenUsageTracker,
    ) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .build()?;

        Ok(Self {
            config,
            model,
            client,
            token_counter,
            usage_tracker,
        })
    }

    async fn post(&self, payload: Value) -> Result<Value, ProviderError> {
        let base_url = url::Url::parse(&self.config.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let url = base_url.join(&self.config.base_path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let mut request = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.config.api_key));

        // Add organization header if present
        if let Some(org) = &self.config.organization {
            request = request.header("OpenAI-Organization", org);
        }

        // Add project header if present
        if let Some(project) = &self.config.project {
            request = request.header("OpenAI-Project", project);
        }

        if let Some(custom_headers) = &self.config.custom_headers {
            for (key, value) in custom_headers {
                request = request.header(key, value);
            }
        }

        let response = request.json(&payload).send().await?;

        handle_response_openai_compat(response).await
    }
}

#[async_trait]
impl Provider for OpenAiProvider {
    #[tracing::instrument(
        skip(self, system, messages, tools),
        fields(model_config, input, output, input_tokens, output_tokens, total_tokens)
    )]
    fn usage_tracker(&self) -> TokenUsageTracker {
        self.usage_tracker.clone()
    }

    async fn complete(
        &self,
        system: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> Result<ProviderCompleteResponse, ProviderError> {
        let payload = create_request(&self.model, system, messages, tools, &ImageFormat::OpenAi)?;

        let calculated_input_tokens = self
            .token_counter
            .count_chat_tokens(system, messages, tools) as u64;

        // Make request
        let response = self.post(payload.clone()).await?;

        // Parse response
        let message = response_to_message(response.clone())?;
        let usage = match get_usage(&response) {
            Ok(usage) => usage,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage data: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);

        let final_input_tokens = if usage.input_tokens.unwrap_or_default() > 0 {
            usage.input_tokens.unwrap_or_default() as u64
        } else {
            calculated_input_tokens
        };

        let final_output_tokens = if usage.output_tokens.unwrap_or_default() > 0 {
            usage.output_tokens.unwrap_or_default() as u64
        } else {
            let response_text = message
                .content
                .iter()
                .find_map(|c| c.as_text())
                .unwrap_or_default();
            self.token_counter.count_tokens(response_text) as u64
        };

        self.usage_tracker.record_usage(
            &self.model.model_id,
            final_input_tokens,
            final_output_tokens,
        );

        emit_debug_trace(&self.model, &payload, &response, &usage);
        Ok(ProviderCompleteResponse::new(message, model, usage))
    }

    async fn extract(
        &self,
        system: &str,
        messages: &[Message],
        schema: &Value,
    ) -> Result<ProviderExtractResponse, ProviderError> {
        // 1. Build base payload (no tools)
        let mut payload = create_request(&self.model, system, messages, &[], &ImageFormat::OpenAi)?;

        // 2. Inject strict JSON‐Schema wrapper
        payload
            .as_object_mut()
            .expect("payload must be an object")
            .insert(
                "response_format".to_string(),
                json!({
                    "type": "json_schema",
                    "json_schema": {
                        "name": "extraction",
                        "schema": schema,
                        "strict": true
                    }
                }),
            );

        // For extract, tools are empty
        let calculated_input_tokens = self
            .token_counter
            .count_chat_tokens(system, messages, &[]) as u64;

        // 3. Call OpenAI
        let response = self.post(payload.clone()).await?;

        // 4. Extract the assistant’s `content` and parse it into JSON
        let msg = &response["choices"][0]["message"];
        let raw = msg.get("content").cloned().ok_or_else(|| {
            ProviderError::ResponseParseError("Missing content in extract response".into())
        })?;
        let data = match raw {
            Value::String(s) => serde_json::from_str(&s)
                .map_err(|e| ProviderError::ResponseParseError(format!("Invalid JSON: {}", e)))?,
            Value::Object(_) | Value::Array(_) => raw,
            other => {
                return Err(ProviderError::ResponseParseError(format!(
                    "Unexpected content type: {:?}",
                    other
                )))
            }
        };

        // 5. Gather usage & model info
        let usage = match get_usage(&response) {
            Ok(u) => u,
            Err(ProviderError::UsageError(e)) => {
                tracing::debug!("Failed to get usage in extract: {}", e);
                Usage::default()
            }
            Err(e) => return Err(e),
        };
        let model = get_model(&response);

        let final_input_tokens = if usage.input_tokens.unwrap_or_default() > 0 {
            usage.input_tokens.unwrap_or_default() as u64
        } else {
            calculated_input_tokens
        };

        let final_output_tokens = if usage.output_tokens.unwrap_or_default() > 0 {
            usage.output_tokens.unwrap_or_default() as u64
        } else {
            match serde_json::to_string(&data) {
                Ok(data_str) => self.token_counter.count_tokens(&data_str) as u64,
                Err(_) => 0, // Or handle error appropriately
            }
        };

        self.usage_tracker.record_usage(
            &self.model.model_id,
            final_input_tokens,
            final_output_tokens,
        );

        Ok(ProviderExtractResponse::new(data, model, usage))
    }
}
