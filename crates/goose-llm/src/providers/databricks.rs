use std::{sync::Arc, time::Duration};

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use url::Url;

use super::{
    errors::ProviderError,
    formats::databricks::{create_request, get_usage, response_to_message},
    utils::{get_env, get_model, ImageFormat},
};
use crate::{
    message::Message,
    model::ModelConfig,
    providers::{Provider, ProviderCompleteResponse, ProviderExtractResponse, Usage},
    types::core::Tool,
    usage_tracker::TokenUsageTracker,
};
use goose::{model::CLAUDE_TOKENIZER, token_counter::TokenCounter};

pub const DATABRICKS_DEFAULT_MODEL: &str = "databricks-claude-3-7-sonnet";
// Databricks can passthrough to a wide range of models, we only provide the default
pub const _DATABRICKS_KNOWN_MODELS: &[&str] = &[
    "databricks-meta-llama-3-3-70b-instruct",
    "databricks-claude-3-7-sonnet",
];

fn default_timeout() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabricksProviderConfig {
    pub host: String,
    pub token: String,
    #[serde(default)]
    pub image_format: ImageFormat,
    #[serde(default = "default_timeout")]
    pub timeout: u64, // timeout in seconds
}

impl DatabricksProviderConfig {
    pub fn new(host: String, token: String) -> Self {
        Self {
            host,
            token,
            image_format: ImageFormat::OpenAi,
            timeout: default_timeout(),
        }
    }

    pub fn from_env() -> Self {
        let host = get_env("DATABRICKS_HOST").expect("Missing DATABRICKS_HOST");
        let token = get_env("DATABRICKS_TOKEN").expect("Missing DATABRICKS_TOKEN");
        Self::new(host, token)
    }
}

#[derive(Debug)]
pub struct DatabricksProvider {
    config: DatabricksProviderConfig,
    model: ModelConfig,
    client: Client,
    token_counter: Arc<TokenCounter>,
    usage_tracker: TokenUsageTracker,
}

impl DatabricksProvider {
    pub fn from_env(model: ModelConfig) -> Self {
        let config = DatabricksProviderConfig::from_env();
        let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        DatabricksProvider::from_config(config, model, token_counter, usage_tracker)
            .expect("Failed to initialize DatabricksProvider")
    }
}

impl Default for DatabricksProvider {
    fn default() -> Self {
        let config = DatabricksProviderConfig::from_env();
        let model = ModelConfig::new(DATABRICKS_DEFAULT_MODEL.to_string());
        let token_counter = Arc::new(TokenCounter::new(CLAUDE_TOKENIZER));
        let usage_tracker = TokenUsageTracker::new();
        DatabricksProvider::from_config(config, model, token_counter, usage_tracker)
            .expect("Failed to initialize DatabricksProvider")
    }
}

impl DatabricksProvider {
    pub fn from_config(
        config: DatabricksProviderConfig,
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
        let base_url = Url::parse(&self.config.host)
            .map_err(|e| ProviderError::RequestFailed(format!("Invalid base URL: {e}")))?;
        let path = format!("serving-endpoints/{}/invocations", self.model.model_name);
        let url = base_url.join(&path).map_err(|e| {
            ProviderError::RequestFailed(format!("Failed to construct endpoint URL: {e}"))
        })?;

        let auth_header = format!("Bearer {}", &self.config.token);
        let response = self
            .client
            .post(url)
            .header("Authorization", auth_header)
            .json(&payload)
            .send()
            .await?;

        let status = response.status();
        let payload: Option<Value> = response.json().await.ok();

        match status {
            StatusCode::OK => payload.ok_or_else(|| {
                ProviderError::RequestFailed("Response body is not valid JSON".to_string())
            }),
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(ProviderError::Authentication(format!(
                    "Authentication failed. Please ensure your API keys are valid and have the required permissions. \
                    Status: {}. Response: {:?}",
                    status, payload
                )))
            }
            StatusCode::BAD_REQUEST => {
                // Databricks provides a generic 'error' but also includes 'external_model_message' which is provider specific
                // We try to extract the error message from the payload and check for phrases that indicate context length exceeded
                let payload_str = serde_json::to_string(&payload)
                    .unwrap_or_default()
                    .to_lowercase();
                let check_phrases = [
                    "too long",
                    "context length",
                    "context_length_exceeded",
                    "reduce the length",
                    "token count",
                    "exceeds",
                    "exceed context limit",
                    "input length",
                    "max_tokens",
                    "decrease input length",
                    "context limit",
                ];
                if check_phrases.iter().any(|c| payload_str.contains(c)) {
                    return Err(ProviderError::ContextLengthExceeded(payload_str));
                }

                let mut error_msg = "Unknown error".to_string();
                if let Some(payload) = &payload {
                    // try to convert message to string, if that fails use external_model_message
                    error_msg = payload
                        .get("message")
                        .and_then(|m| m.as_str())
                        .or_else(|| {
                            payload
                                .get("external_model_message")
                                .and_then(|ext| ext.get("message"))
                                .and_then(|m| m.as_str())
                        })
                        .unwrap_or("Unknown error")
                        .to_string();
                }

                tracing::debug!(
                    "{}",
                    format!(
                        "Provider request failed with status: {}. Payload: {:?}",
                        status, payload
                    )
                );
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status: {}. Message: {}",
                    status, error_msg
                )))
            }
            StatusCode::TOO_MANY_REQUESTS => {
                Err(ProviderError::RateLimitExceeded(format!("{:?}", payload)))
            }
            StatusCode::INTERNAL_SERVER_ERROR | StatusCode::SERVICE_UNAVAILABLE => {
                Err(ProviderError::ServerError(format!("{:?}", payload)))
            }
            _ => {
                tracing::debug!(
                    "{}",
                    format!(
                        "Provider request failed with status: {}. Payload: {:?}",
                        status, payload
                    )
                );
                Err(ProviderError::RequestFailed(format!(
                    "Request failed with status: {}",
                    status
                )))
            }
        }
    }
}

#[async_trait]
impl Provider for DatabricksProvider {
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
        let mut payload = create_request(
            &self.model,
            system,
            messages,
            tools,
            &self.config.image_format,
        )?;
        // Remove the model key which is part of the url with databricks
        payload
            .as_object_mut()
            .expect("payload should have model key")
            .remove("model");

        let calculated_input_tokens = self
            .token_counter
            .count_chat_tokens(system, messages, tools) as u64;

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
        super::utils::emit_debug_trace(&self.model, &payload, &response, &usage);

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
            &self.model.model_name,
            final_input_tokens,
            final_output_tokens,
        );

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
                Err(_) => 0,
            }
        };

        self.usage_tracker.record_usage(
            &self.model.model_name,
            final_input_tokens,
            final_output_tokens,
        );

        Ok(ProviderExtractResponse::new(data, model, usage))
    }
}
