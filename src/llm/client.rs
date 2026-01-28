use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message};
use crate::utils::error::RuleyError;

/// Configuration for retry behavior on transient failures.
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries in milliseconds.
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
        }
    }
}

pub struct LLMClient {
    provider: Box<dyn LLMProvider>,
    retry_config: RetryConfig,
}

impl LLMClient {
    pub fn new(provider: Box<dyn LLMProvider>) -> Self {
        Self {
            provider,
            retry_config: RetryConfig::default(),
        }
    }

    /// Create a new LLMClient with custom retry configuration.
    pub fn with_retry_config(provider: Box<dyn LLMProvider>, retry_config: RetryConfig) -> Self {
        Self {
            provider,
            retry_config,
        }
    }

    /// Complete a prompt using the configured LLM provider.
    ///
    /// This method wraps the provider's complete method and will be extended
    /// with retry logic in a future implementation.
    pub async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // Note: Retry logic with exponential backoff will be implemented in Task #7.
        // For now, this passes through directly to the provider.
        self.provider.complete(messages, options).await
    }

    /// Get the model name from the provider.
    pub fn model(&self) -> &str {
        self.provider.model()
    }

    /// Get pricing information from the provider.
    pub fn pricing(&self) -> crate::llm::provider::Pricing {
        self.provider.pricing()
    }

    /// Get the retry configuration.
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 30000);
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 60000,
        };
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 60000);
    }
}
