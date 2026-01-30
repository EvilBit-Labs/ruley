use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message};
use crate::utils::error::RuleyError;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

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
///     jitter: true,
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
    /// Whether to add random jitter to prevent thundering herd.
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            jitter: true,
        }
    }
}

impl RetryConfig {
    /// Calculate the delay for a given retry attempt (0-indexed).
    ///
    /// Uses exponential backoff: `initial_delay * 2^attempt`, capped at `max_delay`.
    /// If jitter is enabled, adds random jitter of +/- 25% to prevent thundering herd.
    #[must_use]
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        // Exponential backoff: initial_delay * 2^attempt
        let base_delay_ms = self
            .initial_delay_ms
            .saturating_mul(1u64 << attempt.min(10));

        // Cap at max delay
        let capped_delay_ms = base_delay_ms.min(self.max_delay_ms);

        if self.jitter {
            // Add jitter: +/- 25% of the delay
            let jitter_range = capped_delay_ms / 4;
            let jitter = Self::random_jitter(jitter_range);
            let final_delay_ms = if jitter >= 0 {
                capped_delay_ms.saturating_add(jitter as u64)
            } else {
                capped_delay_ms.saturating_sub(jitter.unsigned_abs())
            };
            Duration::from_millis(final_delay_ms)
        } else {
            Duration::from_millis(capped_delay_ms)
        }
    }

    /// Generate a random jitter value in the range [-range, range].
    ///
    /// Uses a simple PRNG based on the current time for randomness.
    fn random_jitter(range: u64) -> i64 {
        if range == 0 {
            return 0;
        }
        // Use nanoseconds from current time as a simple random source
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);

        // Map to range [-range, range]
        let range_i64 = range as i64;
        (nanos as i64 % (range_i64 * 2 + 1)) - range_i64
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
    /// the completion response. Implements retry logic with exponential backoff
    /// for transient failures (rate limiting, server errors, network timeouts).
    ///
    /// # Retry Behavior
    ///
    /// The following errors are retried with exponential backoff:
    /// - HTTP 429 (rate limited)
    /// - HTTP 500, 502, 503, 504 (server errors)
    /// - Network timeouts and connection errors
    ///
    /// The following errors are NOT retried:
    /// - HTTP 400, 401, 403 (client errors)
    /// - Token/context length exceeded
    /// - Configuration errors
    ///
    /// # Arguments
    ///
    /// * `messages` - The conversation messages to send to the LLM.
    /// * `options` - Configuration options for the completion request.
    ///
    /// # Errors
    ///
    /// Returns an error if the LLM provider fails to complete the request
    /// after all retry attempts are exhausted.
    pub async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        let mut last_error: Option<RuleyError> = None;

        for attempt in 0..=self.retry_config.max_retries {
            match self.provider.complete(messages, options).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if !Self::is_retryable(&err) {
                        debug!(
                            attempt = attempt,
                            error = %err,
                            "Non-retryable error encountered, failing immediately"
                        );
                        return Err(err);
                    }

                    if attempt == self.retry_config.max_retries {
                        warn!(
                            attempts = attempt + 1,
                            error = %err,
                            "Max retries exhausted"
                        );
                        return Err(err);
                    }

                    // Calculate delay, respecting rate limit retry_after if present
                    let delay = if let RuleyError::RateLimited {
                        retry_after: Some(retry_after),
                        ..
                    } = &err
                    {
                        // Use the server-suggested retry time, but add jitter
                        let base_delay = *retry_after;
                        if self.retry_config.jitter {
                            let jitter_ms =
                                RetryConfig::random_jitter((base_delay.as_millis() as u64) / 4);
                            if jitter_ms >= 0 {
                                base_delay + Duration::from_millis(jitter_ms as u64)
                            } else {
                                base_delay
                                    .saturating_sub(Duration::from_millis(jitter_ms.unsigned_abs()))
                            }
                        } else {
                            base_delay
                        }
                    } else {
                        self.retry_config.calculate_delay(attempt)
                    };

                    warn!(
                        attempt = attempt + 1,
                        max_retries = self.retry_config.max_retries,
                        delay_ms = delay.as_millis(),
                        error = %err,
                        "Retrying after transient error"
                    );

                    last_error = Some(err);
                    sleep(delay).await;
                }
            }
        }

        // Defensive fallback: in normal operation, all error paths return early within the loop.
        // This provides a safe fallback in case of unexpected control flow.
        Err(last_error.unwrap_or_else(|| RuleyError::Provider {
            provider: self.provider.model().to_string(),
            message: "Unknown error during retry".to_string(),
        }))
    }

    /// Determines if an error is retryable.
    ///
    /// Retryable errors:
    /// - `RateLimited` (HTTP 429)
    /// - `NetworkError` (timeouts, connection errors)
    /// - `Provider` errors with 5xx status codes
    ///
    /// Non-retryable errors:
    /// - `Config` errors
    /// - `TokenLimitExceeded` (context length exceeded)
    /// - `ValidationError` (client errors like 400, 401, 403)
    /// - All other error types
    fn is_retryable(error: &RuleyError) -> bool {
        match error {
            // Always retry rate limiting
            RuleyError::RateLimited { .. } => true,

            // Always retry network errors (timeouts, connection issues)
            RuleyError::NetworkError { .. } => true,

            // Provider errors may or may not be retryable
            RuleyError::Provider { message, .. } => {
                // Retry on 5xx server errors
                Self::is_server_error(message)
            }

            // Never retry these
            RuleyError::Config(_)
            | RuleyError::Repository(_)
            | RuleyError::FileSystem(_)
            | RuleyError::TokenLimitExceeded { .. }
            | RuleyError::Compression { .. }
            | RuleyError::OutputFormat(_)
            | RuleyError::ParseError { .. }
            | RuleyError::ValidationError { .. }
            | RuleyError::Cache(_)
            | RuleyError::State(_) => false,
        }
    }

    /// Check if a provider error message indicates a 5xx server error.
    fn is_server_error(message: &str) -> bool {
        // Check for HTTP 5xx status codes in the error message
        message.contains("HTTP 500")
            || message.contains("HTTP 501")
            || message.contains("HTTP 502")
            || message.contains("HTTP 503")
            || message.contains("HTTP 504")
            || message.contains("Internal Server Error")
            || message.contains("Bad Gateway")
            || message.contains("Service Unavailable")
            || message.contains("Gateway Timeout")
            || message.to_lowercase().contains("server error")
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
    use crate::llm::provider::Pricing;
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 1000);
        assert_eq!(config.max_delay_ms, 60000);
        assert!(config.jitter);
    }

    #[test]
    fn test_retry_config_custom() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 500,
            max_delay_ms: 60000,
            jitter: false,
        };
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.initial_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 60000);
        assert!(!config.jitter);
    }

    #[test]
    fn test_calculate_delay_exponential_backoff() {
        let config = RetryConfig {
            max_retries: 5,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter: false, // Disable jitter for predictable testing
        };

        // Test exponential backoff: 1s -> 2s -> 4s -> 8s
        assert_eq!(config.calculate_delay(0), Duration::from_millis(1000));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(2000));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(4000));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(8000));
    }

    #[test]
    fn test_calculate_delay_caps_at_max() {
        let config = RetryConfig {
            max_retries: 10,
            initial_delay_ms: 1000,
            max_delay_ms: 5000,
            jitter: false,
        };

        // Should cap at 5000ms regardless of attempt number
        assert_eq!(config.calculate_delay(0), Duration::from_millis(1000));
        assert_eq!(config.calculate_delay(1), Duration::from_millis(2000));
        assert_eq!(config.calculate_delay(2), Duration::from_millis(4000));
        assert_eq!(config.calculate_delay(3), Duration::from_millis(5000)); // Capped
        assert_eq!(config.calculate_delay(4), Duration::from_millis(5000)); // Capped
        assert_eq!(config.calculate_delay(10), Duration::from_millis(5000)); // Capped
    }

    #[test]
    fn test_calculate_delay_with_jitter() {
        let config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter: true,
        };

        // With jitter, delay should be within +/- 25% of base delay
        let delay = config.calculate_delay(0);
        // Base is 1000ms, jitter range is +/- 250ms, so delay should be 750-1250ms
        assert!(delay >= Duration::from_millis(750));
        assert!(delay <= Duration::from_millis(1250));
    }

    #[test]
    fn test_is_retryable_rate_limited() {
        let error = RuleyError::RateLimited {
            provider: "test".to_string(),
            retry_after: Some(Duration::from_secs(5)),
        };
        assert!(LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_retryable_network_error() {
        let error = RuleyError::NetworkError {
            message: "Connection timeout".to_string(),
            source: None,
        };
        assert!(LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_retryable_server_error() {
        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 500 Internal Server Error".to_string(),
        };
        assert!(LLMClient::is_retryable(&error));

        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 502 Bad Gateway".to_string(),
        };
        assert!(LLMClient::is_retryable(&error));

        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 503 Service Unavailable".to_string(),
        };
        assert!(LLMClient::is_retryable(&error));

        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 504 Gateway Timeout".to_string(),
        };
        assert!(LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_not_retryable_client_error() {
        // 400 Bad Request
        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 400 Bad Request".to_string(),
        };
        assert!(!LLMClient::is_retryable(&error));

        // 401 Unauthorized
        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 401 Unauthorized".to_string(),
        };
        assert!(!LLMClient::is_retryable(&error));

        // 403 Forbidden
        let error = RuleyError::Provider {
            provider: "test".to_string(),
            message: "HTTP 403 Forbidden".to_string(),
        };
        assert!(!LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_not_retryable_token_limit() {
        let error = RuleyError::TokenLimitExceeded {
            tokens: 100000,
            limit: 50000,
        };
        assert!(!LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_not_retryable_config_error() {
        let error = RuleyError::Config("Missing API key".to_string());
        assert!(!LLMClient::is_retryable(&error));
    }

    #[test]
    fn test_is_not_retryable_validation_error() {
        let error = RuleyError::ValidationError {
            message: "Invalid format".to_string(),
            suggestion: "Use a valid format".to_string(),
        };
        assert!(!LLMClient::is_retryable(&error));
    }

    // Mock provider for testing retry logic
    struct MockProvider {
        call_count: Arc<AtomicUsize>,
        fail_times: usize,
        error_type: MockErrorType,
    }

    #[derive(Clone)]
    enum MockErrorType {
        RateLimited,
        ServerError,
        NetworkError,
        ClientError,
    }

    impl MockProvider {
        fn new(fail_times: usize, error_type: MockErrorType) -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
                fail_times,
                error_type,
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _options: &CompletionOptions,
        ) -> Result<CompletionResponse, RuleyError> {
            let current_count = self.call_count.fetch_add(1, Ordering::SeqCst);

            if current_count < self.fail_times {
                match self.error_type {
                    MockErrorType::RateLimited => Err(RuleyError::RateLimited {
                        provider: "mock".to_string(),
                        retry_after: Some(Duration::from_millis(10)),
                    }),
                    MockErrorType::ServerError => Err(RuleyError::Provider {
                        provider: "mock".to_string(),
                        message: "HTTP 500 Internal Server Error".to_string(),
                    }),
                    MockErrorType::NetworkError => Err(RuleyError::NetworkError {
                        message: "Connection timeout".to_string(),
                        source: None,
                    }),
                    MockErrorType::ClientError => Err(RuleyError::Provider {
                        provider: "mock".to_string(),
                        message: "HTTP 400 Bad Request".to_string(),
                    }),
                }
            } else {
                Ok(CompletionResponse {
                    content: "Success".to_string(),
                    tokens_used: 10,
                })
            }
        }

        fn model(&self) -> &str {
            "mock-model"
        }

        fn pricing(&self) -> Pricing {
            Pricing {
                input_per_1k: 0.0,
                output_per_1k: 0.0,
            }
        }
    }

    #[tokio::test]
    async fn test_retry_succeeds_after_failures() {
        let provider = MockProvider::new(2, MockErrorType::RateLimited);
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            jitter: false,
        };
        let client = LLMClient::with_retry_config(Box::new(provider), retry_config);

        let result = client.complete(&[], &CompletionOptions::default()).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().content, "Success");
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let provider = MockProvider::new(5, MockErrorType::ServerError);
        let call_count = provider.call_count.clone();
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            jitter: false,
        };
        let client = LLMClient::with_retry_config(Box::new(provider), retry_config);

        let result = client.complete(&[], &CompletionOptions::default()).await;

        assert!(result.is_err());
        // Should have tried 4 times (initial + 3 retries)
        assert_eq!(call_count.load(Ordering::SeqCst), 4);
    }

    #[tokio::test]
    async fn test_no_retry_on_client_error() {
        let provider = MockProvider::new(5, MockErrorType::ClientError);
        let call_count = provider.call_count.clone();
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            jitter: false,
        };
        let client = LLMClient::with_retry_config(Box::new(provider), retry_config);

        let result = client.complete(&[], &CompletionOptions::default()).await;

        assert!(result.is_err());
        // Should have tried only once (no retries for client errors)
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_on_network_error() {
        let provider = MockProvider::new(1, MockErrorType::NetworkError);
        let call_count = provider.call_count.clone();
        let retry_config = RetryConfig {
            max_retries: 3,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            jitter: false,
        };
        let client = LLMClient::with_retry_config(Box::new(provider), retry_config);

        let result = client.complete(&[], &CompletionOptions::default()).await;

        assert!(result.is_ok());
        // Should have tried twice (first failure + successful retry)
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_zero_retries_fails_immediately() {
        let provider = MockProvider::new(1, MockErrorType::RateLimited);
        let call_count = provider.call_count.clone();
        let retry_config = RetryConfig {
            max_retries: 0,
            initial_delay_ms: 1,
            max_delay_ms: 10,
            jitter: false,
        };
        let client = LLMClient::with_retry_config(Box::new(provider), retry_config);

        let result = client.complete(&[], &CompletionOptions::default()).await;

        assert!(result.is_err());
        // Should have tried exactly once (no retries when max_retries = 0)
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
