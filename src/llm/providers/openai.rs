use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MAX_TOKENS: usize = 4096;

pub struct OpenAIProvider {
    api_key: String,
    model: String,
    client: Client,
}

/// Request body for the OpenAI Chat Completions API.
#[derive(Debug, Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// A message in the OpenAI format.
#[derive(Debug, Serialize)]
struct OpenAIMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Response from the OpenAI Chat Completions API.
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

/// A choice in the OpenAI response.
#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

/// The message content in a choice.
#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

/// Token usage information from OpenAI.
#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

/// Error response from the OpenAI API.
#[derive(Debug, Deserialize)]
struct OpenAIError {
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    error_type: Option<String>,
    message: String,
    code: Option<String>,
}

impl OpenAIProvider {
    /// Creates a new OpenAI provider with the given API key and model.
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

    /// Creates a new OpenAI provider from environment variables.
    ///
    /// Reads the `OPENAI_API_KEY` environment variable.
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is not set or if the HTTP client cannot be created.
    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key =
            std::env::var("OPENAI_API_KEY").map_err(|_| RuleyError::missing_api_key("openai"))?;
        Self::new(api_key, "gpt-4o".to_string())
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // Convert messages to OpenAI format
        let openai_messages: Vec<OpenAIMessage<'_>> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        let request_body = OpenAIRequest {
            model: &self.model,
            messages: openai_messages,
            max_tokens: Some(options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
            temperature: options.temperature,
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
                provider: "openai".to_string(),
                retry_after,
            });
        }

        // Handle other HTTP errors
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse the error response
            if let Ok(error) = serde_json::from_str::<OpenAIError>(&error_text) {
                let error_type = error
                    .error
                    .error_type
                    .or(error.error.code)
                    .unwrap_or_else(|| "unknown".to_string());
                return Err(RuleyError::Provider {
                    provider: "openai".to_string(),
                    message: format!("{}: {}", error_type, error.error.message),
                });
            }

            return Err(RuleyError::Provider {
                provider: "openai".to_string(),
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        // Parse successful response
        let response_body: OpenAIResponse = response.json().await?;

        // Extract content from the first choice
        let content = response_body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            tokens_used: response_body.usage.prompt_tokens + response_body.usage.completion_tokens,
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 2.5,
            output_per_1k: 10.0,
        }
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

        let openai_messages: Vec<OpenAIMessage<'_>> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        assert_eq!(openai_messages.len(), 2);
        assert_eq!(openai_messages[0].role, "system");
        assert_eq!(openai_messages[1].role, "user");
    }

    #[test]
    fn test_request_serialization() {
        let messages = vec![OpenAIMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OpenAIRequest {
            model: "gpt-4o",
            messages,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(json.contains("\"model\":\"gpt-4o\""));
        assert!(json.contains("\"max_tokens\":1024"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn test_request_serialization_without_optional_fields() {
        let messages = vec![OpenAIMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OpenAIRequest {
            model: "gpt-4o",
            messages,
            max_tokens: None,
            temperature: None,
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("temperature"));
    }
}
