use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// Represents token usage for a specific LLM provider.
#[derive(Debug, Clone, Default)]
pub struct ProviderUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

// Tracks token usage across multiple LLM providers.
#[derive(Debug, Clone, Default)]
pub struct TokenUsageTracker {
    usage_by_provider: Arc<Mutex<HashMap<String, ProviderUsage>>>,
}

impl TokenUsageTracker {
    pub fn new() -> Self {
        Self {
            usage_by_provider: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Records token usage for a given provider.
    pub fn record_usage(&self, provider_id: &str, input_tokens: u64, output_tokens: u64) {
        let mut usage_map = self.usage_by_provider.lock().unwrap();
        let provider_usage = usage_map.entry(provider_id.to_string()).or_default();
        provider_usage.input_tokens += input_tokens;
        provider_usage.output_tokens += output_tokens;
    }

    // Retrieves token usage for a specific provider.
    pub fn get_usage(&self, provider_id: &str) -> Option<ProviderUsage> {
        let usage_map = self.usage_by_provider.lock().unwrap();
        usage_map.get(provider_id).cloned()
    }

    // Retrieves all token usage data.
    pub fn get_all_usage(&self) -> HashMap<String, ProviderUsage> {
        let usage_map = self.usage_by_provider.lock().unwrap();
        usage_map.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_get_usage() {
        let tracker = TokenUsageTracker::new();
        let provider_id = "test_provider";

        tracker.record_usage(provider_id, 100, 200);
        let usage = tracker.get_usage(provider_id).unwrap();
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);

        tracker.record_usage(provider_id, 50, 75);
        let usage = tracker.get_usage(provider_id).unwrap();
        assert_eq!(usage.input_tokens, 150);
        assert_eq!(usage.output_tokens, 275);
    }

    #[test]
    fn test_get_all_usage() {
        let tracker = TokenUsageTracker::new();
        tracker.record_usage("provider1", 10, 20);
        tracker.record_usage("provider2", 30, 40);

        let all_usage = tracker.get_all_usage();
        assert_eq!(all_usage.len(), 2);
        assert_eq!(all_usage.get("provider1").unwrap().input_tokens, 10);
        assert_eq!(all_usage.get("provider2").unwrap().output_tokens, 40);
    }

    #[test]
    fn test_get_usage_non_existent_provider() {
        let tracker = TokenUsageTracker::new();
        let usage = tracker.get_usage("non_existent_provider");
        assert!(usage.is_none());
    }
}
