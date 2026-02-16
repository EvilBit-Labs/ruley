use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;
use std::time::Duration;

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
const OPENROUTER_MODELS_URL: &str = "https://openrouter.ai/api/v1/models";
const DEFAULT_MAX_TOKENS: usize = 4096;

/// OpenRouter LLM provider for accessing multiple models via a unified API.
///
/// Uses the OpenAI-compatible API at `https://openrouter.ai/api/v1/chat/completions`.
/// Supports rate limiting (429 handling) and 120-second timeout like cloud providers.
///
/// # Configuration
///
/// - `OPENROUTER_API_KEY` env var is required for authentication
/// - Config file: `[providers.openrouter] model = "..."` sets the default model
///
/// # Examples
///
/// ```no_run
/// use ruley::llm::providers::openrouter::OpenRouterProvider;
///
/// let provider = OpenRouterProvider::new(
///     "your-api-key".to_string(),
///     "anthropic/claude-3.5-sonnet".to_string(),
/// ).expect("Failed to create provider");
/// ```
pub struct OpenRouterProvider {
    api_key: String,
    model: String,
    client: Client,
    /// Dynamic pricing fetched from OpenRouter's models API.
    /// Updated by `fetch_model_pricing()` with actual per-1k-token rates.
    cached_pricing: RwLock<Pricing>,
}

/// Request body for the OpenRouter API (OpenAI-compatible).
#[derive(Debug, Serialize)]
struct OpenRouterRequest<'a> {
    model: &'a str,
    messages: Vec<OpenRouterMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// A message in the OpenAI-compatible format.
#[derive(Debug, Serialize)]
struct OpenRouterMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Response from the OpenRouter API.
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

/// A choice in the response.
#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// The message content in a choice.
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

/// Token usage information.
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

/// Error response from the OpenRouter API.
#[derive(Debug, Deserialize)]
struct OpenRouterError {
    error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    error_type: Option<String>,
    message: Option<String>,
    code: Option<String>,
}

/// Response from the OpenRouter models list API (`/api/v1/models`).
#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

/// Model metadata from the OpenRouter models API.
#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
    pricing: Option<ModelPricing>,
}

/// Per-token pricing from the OpenRouter models API.
/// Values are strings representing USD cost per token (e.g. "0.000003").
#[derive(Debug, Deserialize)]
struct ModelPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

impl OpenRouterProvider {
    /// Creates a new OpenRouter provider with the given API key and model.
    ///
    /// Uses a 120-second timeout consistent with other cloud providers.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(api_key: String, model: String) -> Result<Self, RuleyError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|e| RuleyError::Config(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self {
            api_key,
            model,
            client,
            cached_pricing: RwLock::new(Pricing {
                input_per_1k: 0.0,
                output_per_1k: 0.0,
            }),
        })
    }

    /// Creates a new OpenRouter provider from environment variables.
    ///
    /// Reads the `OPENROUTER_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is not set or if the HTTP client cannot be created.
    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| RuleyError::missing_api_key("openrouter"))?;
        Self::new(api_key, "anthropic/claude-3.5-sonnet".to_string())
    }

    /// Fetches model pricing from the OpenRouter models API and caches it.
    ///
    /// Queries `GET /api/v1/models`, finds the matching model by ID, and
    /// stores the per-1k-token input/output prices (including any OpenRouter markup).
    /// Falls back to zero pricing if the fetch fails or the model is not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP request fails or the response cannot be parsed.
    pub async fn fetch_model_pricing(&self) -> Result<(), RuleyError> {
        let response = self
            .client
            .get(OPENROUTER_MODELS_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| RuleyError::Provider {
                provider: "openrouter".to_string(),
                message: format!("Failed to fetch model pricing: {e}"),
            })?;

        if !response.status().is_success() {
            return Err(RuleyError::Provider {
                provider: "openrouter".to_string(),
                message: format!("Models API returned HTTP {}", response.status()),
            });
        }

        let models: ModelsResponse = response.json().await.map_err(|e| RuleyError::Provider {
            provider: "openrouter".to_string(),
            message: format!("Failed to parse models response: {e}"),
        })?;

        let model_info = models.data.iter().find(|m| m.id == self.model);

        let Some(info) = model_info else {
            tracing::warn!(
                "Model '{}' not found in OpenRouter models list; pricing unavailable",
                self.model
            );
            return Ok(());
        };

        let Some(ref api_pricing) = info.pricing else {
            tracing::warn!(
                "No pricing data for model '{}' in OpenRouter response",
                self.model
            );
            return Ok(());
        };

        let input_per_token: f64 = api_pricing
            .prompt
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let output_per_token: f64 = api_pricing
            .completion
            .as_deref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        // Convert per-token to per-1k-token pricing
        let pricing = Pricing {
            input_per_1k: input_per_token * 1000.0,
            output_per_1k: output_per_token * 1000.0,
        };

        tracing::debug!(
            "Fetched OpenRouter pricing for '{}': ${:.6}/1k input, ${:.6}/1k output",
            self.model,
            pricing.input_per_1k,
            pricing.output_per_1k
        );

        if let Ok(mut cached) = self.cached_pricing.write() {
            *cached = pricing;
        }

        Ok(())
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        let openrouter_messages: Vec<OpenRouterMessage<'_>> = messages
            .iter()
            .map(|m| OpenRouterMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        let request_body = OpenRouterRequest {
            model: &self.model,
            messages: openrouter_messages,
            max_tokens: Some(options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
            temperature: options.temperature,
        };

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/ruley-ai/ruley")
            .header("X-Title", "ruley")
            .json(&request_body)
            .send()
            .await?;

        let status = response.status();

        // Handle rate limiting
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(Duration::from_secs);

            return Err(RuleyError::RateLimited {
                provider: "openrouter".to_string(),
                retry_after,
            });
        }

        // Handle other HTTP errors
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse structured error response
            if let Ok(error) = serde_json::from_str::<OpenRouterError>(&error_text) {
                if let Some(detail) = error.error {
                    let error_type = detail
                        .error_type
                        .or(detail.code)
                        .unwrap_or_else(|| "unknown".to_string());
                    let message = detail
                        .message
                        .unwrap_or_else(|| "Unknown error".to_string());
                    return Err(RuleyError::Provider {
                        provider: "openrouter".to_string(),
                        message: format!("{}: {}", error_type, message),
                    });
                }
            }

            return Err(RuleyError::Provider {
                provider: "openrouter".to_string(),
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        // Parse successful response
        let response_body: OpenRouterResponse = response.json().await?;

        let content = response_body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();

        let (prompt_tokens, completion_tokens) = response_body
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(CompletionResponse::new(
            content,
            prompt_tokens,
            completion_tokens,
        ))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        self.cached_pricing
            .read()
            .map(|p| p.clone())
            .unwrap_or(Pricing {
                input_per_1k: 0.0,
                output_per_1k: 0.0,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_conversion() {
        let messages = [
            Message {
                role: "system".to_string(),
                content: "You are helpful".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];

        let openrouter_messages: Vec<OpenRouterMessage<'_>> = messages
            .iter()
            .map(|m| OpenRouterMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        assert_eq!(openrouter_messages.len(), 2);
        assert_eq!(openrouter_messages[0].role, "system");
        assert_eq!(openrouter_messages[1].role, "user");
    }

    #[test]
    fn test_request_serialization() {
        let messages = vec![OpenRouterMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OpenRouterRequest {
            model: "anthropic/claude-3.5-sonnet",
            messages,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(json.contains("\"model\":\"anthropic/claude-3.5-sonnet\""));
        assert!(json.contains("\"max_tokens\":1024"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn test_request_serialization_without_optional_fields() {
        let messages = vec![OpenRouterMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OpenRouterRequest {
            model: "anthropic/claude-3.5-sonnet",
            messages,
            max_tokens: None,
            temperature: None,
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn test_default_pricing_before_fetch() {
        let provider = OpenRouterProvider::new(
            "test-key".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
        )
        .expect("should create provider");
        let pricing = provider.pricing();
        // Before fetch_model_pricing is called, pricing defaults to zero
        assert_eq!(pricing.input_per_1k, 0.0);
        assert_eq!(pricing.output_per_1k, 0.0);
    }

    #[test]
    fn test_pricing_updates_from_rwlock() {
        let provider = OpenRouterProvider::new(
            "test-key".to_string(),
            "anthropic/claude-3.5-sonnet".to_string(),
        )
        .expect("should create provider");

        // Simulate pricing being populated by fetch_model_pricing
        {
            let mut cached = provider.cached_pricing.write().unwrap();
            *cached = Pricing {
                input_per_1k: 0.003,
                output_per_1k: 0.015,
            };
        }

        let pricing = provider.pricing();
        assert_eq!(pricing.input_per_1k, 0.003);
        assert_eq!(pricing.output_per_1k, 0.015);
    }

    #[test]
    fn test_model_pricing_parse() {
        let json = r#"{
            "data": [
                {
                    "id": "anthropic/claude-3.5-sonnet",
                    "pricing": {
                        "prompt": "0.000003",
                        "completion": "0.000015"
                    }
                }
            ]
        }"#;

        let models: ModelsResponse = serde_json::from_str(json).expect("should parse");
        let info = models
            .data
            .iter()
            .find(|m| m.id == "anthropic/claude-3.5-sonnet");
        assert!(info.is_some());

        let pricing = info.unwrap().pricing.as_ref().unwrap();
        let input_per_token: f64 = pricing.prompt.as_deref().unwrap().parse().unwrap();
        let output_per_token: f64 = pricing.completion.as_deref().unwrap().parse().unwrap();

        assert!((input_per_token * 1000.0 - 0.003).abs() < 1e-9);
        assert!((output_per_token * 1000.0 - 0.015).abs() < 1e-9);
    }
}
