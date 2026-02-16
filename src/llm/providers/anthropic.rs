use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: usize = 4096;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    client: Client,
}

/// Request body for the Anthropic Messages API.
#[derive(Debug, Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    max_tokens: usize,
    messages: Vec<AnthropicMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<&'a str>,
}

/// A message in the Anthropic format.
#[derive(Debug, Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Response from the Anthropic Messages API.
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

/// Content block in the Anthropic response.
#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Token usage information from Anthropic.
#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

/// Error response from the Anthropic API.
#[derive(Debug, Deserialize)]
struct AnthropicError {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

impl AnthropicProvider {
    /// Creates a new Anthropic provider with the given API key and model.
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
        })
    }

    /// Creates a new Anthropic provider from environment variables.
    ///
    /// Reads the `ANTHROPIC_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is not set or if the HTTP client cannot be created.
    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| RuleyError::missing_api_key("anthropic"))?;
        Self::new(api_key, "claude-sonnet-4-5-20250929".to_string())
    }

    /// Construct the system prompt from the first message if it has role "system".
    /// Anthropic requires system prompts to be passed separately from messages.
    fn extract_system_prompt(messages: &[Message]) -> (Option<&str>, &[Message]) {
        if let Some(first) = messages.first() {
            if first.role == "system" {
                return (Some(&first.content), &messages[1..]);
            }
        }
        (None, messages)
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        let (system_prompt, user_messages) = Self::extract_system_prompt(messages);

        // Convert messages to Anthropic format
        let anthropic_messages: Vec<AnthropicMessage<'_>> = user_messages
            .iter()
            .map(|m| AnthropicMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        let request_body = AnthropicRequest {
            model: &self.model,
            max_tokens: options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            messages: anthropic_messages,
            temperature: options.temperature,
            system: system_prompt,
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
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
                provider: "anthropic".to_string(),
                retry_after,
            });
        }

        // Handle other HTTP errors
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse the error response
            if let Ok(error) = serde_json::from_str::<AnthropicError>(&error_text) {
                return Err(RuleyError::Provider {
                    provider: "anthropic".to_string(),
                    message: format!("{}: {}", error.error.error_type, error.error.message),
                });
            }

            return Err(RuleyError::Provider {
                provider: "anthropic".to_string(),
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        // Parse successful response
        let response_body: AnthropicResponse = response.json().await?;

        // Extract text content from the response
        let content = response_body
            .content
            .into_iter()
            .filter_map(|block| {
                if block.content_type == "text" {
                    block.text
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse::new(
            content,
            response_body.usage.input_tokens,
            response_body.usage.output_tokens,
        ))
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 0.003,  // $0.003 per 1K input tokens
            output_per_1k: 0.015, // $0.015 per 1K output tokens
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_system_prompt_with_system() {
        let messages = vec![
            Message {
                role: "system".to_string(),
                content: "You are helpful".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
        ];

        let (system, remaining) = AnthropicProvider::extract_system_prompt(&messages);
        assert_eq!(system, Some("You are helpful"));
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].role, "user");
    }

    #[test]
    fn test_extract_system_prompt_without_system() {
        let messages = vec![Message {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }];

        let (system, remaining) = AnthropicProvider::extract_system_prompt(&messages);
        assert!(system.is_none());
        assert_eq!(remaining.len(), 1);
    }

    #[test]
    fn test_extract_system_prompt_empty() {
        let messages: Vec<Message> = vec![];
        let (system, remaining) = AnthropicProvider::extract_system_prompt(&messages);
        assert!(system.is_none());
        assert!(remaining.is_empty());
    }
}
