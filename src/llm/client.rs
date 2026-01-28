use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message};
use crate::utils::error::RuleyError;

/// Configuration for retry behavior on transient failures.
///
/// This struct configures exponential backoff with jitter for retrying
/// failed LLM requests. Retries are performed on transient errors such
/// as rate limiting (HTTP 429) and server errors (HTTP 5xx).
///
/// # Example
///
/// ```
/// use ruley::llm::client::RetryConfig;
///
/// let config = RetryConfig {
///     max_retries: 5,
///     initial_delay_ms: 500,
///     max_delay_ms: 60000,
/// };
/// ```
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

/// A high-level client for interacting with LLM providers.
///
/// `LLMClient` wraps an [`LLMProvider`] implementation and provides additional
/// functionality such as retry logic with exponential backoff. It serves as
/// the primary interface for making LLM requests in ruley.
///
/// # Example
///
/// ```no_run
/// use ruley::llm::client::{LLMClient, RetryConfig};
/// use ruley::llm::providers::anthropic::AnthropicProvider;
/// use ruley::llm::provider::{Message, CompletionOptions};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let provider = AnthropicProvider::from_env()?;
/// let client = LLMClient::new(Box::new(provider));
///
/// let messages = vec![
///     Message { role: "user".into(), content: "Hello!".into() },
/// ];
/// let response = client.complete(&messages, &CompletionOptions::default()).await?;
/// println!("Response: {}", response.content);
/// # Ok(())
/// # }
/// ```
pub struct LLMClient {
    provider: Box<dyn LLMProvider>,
    retry_config: RetryConfig,
}

impl LLMClient {
    /// Creates a new `LLMClient` with the given provider and default retry configuration.
    ///
    /// # Arguments
    ///
    /// * `provider` - A boxed LLM provider implementation.
    pub fn new(provider: Box<dyn LLMProvider>) -> Self {
        Self {
            provider,
            retry_config: RetryConfig::default(),
        }
    }

    /// Creates a new `LLMClient` with custom retry configuration.
    ///
    /// # Arguments
    ///
    /// * `provider` - A boxed LLM provider implementation.
    /// * `retry_config` - Custom retry configuration for handling transient failures.
    pub fn with_retry_config(provider: Box<dyn LLMProvider>, retry_config: RetryConfig) -> Self {
        Self {
            provider,
            retry_config,
        }
    }

    /// Completes a prompt using the configured LLM provider.
    ///
    /// This method sends the given messages to the LLM provider and returns
    /// the completion response. Retry logic with exponential backoff will be
    /// applied for transient failures (to be implemented in Task #7).
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation messages to send to the LLM.
    /// * `options` - Configuration options for the completion request.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM provider fails to complete the request.
    pub async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // Note: Retry logic with exponential backoff will be implemented in Task #7.
        // For now, this passes through directly to the provider.
        self.provider.complete(messages, options).await
    }

    /// Returns the model name from the underlying provider.
    pub fn model(&self) -> &str {
        self.provider.model()
    }

    /// Returns pricing information from the underlying provider.
    pub fn pricing(&self) -> crate::llm::provider::Pricing {
        self.provider.pricing()
    }

    /// Returns a reference to the retry configuration.
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
