use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

const DEFAULT_MAX_TOKENS: usize = 4096;

/// Ollama LLM provider for local model inference.
///
/// Uses the OpenAI-compatible API endpoint at `{host}/v1/chat/completions`.
/// Optimized for local use with a shorter timeout (30s) and no rate limiting.
///
/// # Configuration
///
/// - `OLLAMA_HOST` env var sets the server address (default: `http://localhost:11434`)
/// - Config file: `[providers.ollama] host = "..."` overrides the env var
///
/// # Examples
///
/// ```no_run
/// use ruley::llm::providers::ollama::OllamaProvider;
///
/// let provider = OllamaProvider::new(
///     "http://localhost:11434".to_string(),
///     "llama3.1:70b".to_string(),
/// ).expect("Failed to create provider");
/// ```
pub struct OllamaProvider {
    host: String,
    model: String,
    client: Client,
}

/// Request body for the Ollama OpenAI-compatible API.
#[derive(Debug, Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    messages: Vec<OllamaMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
}

/// A message in the OpenAI-compatible format.
#[derive(Debug, Serialize)]
struct OllamaMessage<'a> {
    role: &'a str,
    content: &'a str,
}

/// Response from the Ollama OpenAI-compatible API.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
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

/// Response from the Ollama `/api/tags` endpoint listing available models.
#[derive(Debug, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelInfo>,
}

/// Model info returned from the `/api/tags` endpoint.
#[derive(Debug, Deserialize)]
struct OllamaModelInfo {
    name: String,
}

/// Error response from the Ollama API.
#[derive(Debug, Deserialize)]
struct OllamaError {
    error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: Option<String>,
    #[allow(dead_code)] // Deserialized but not directly read; kept for future use
    #[serde(rename = "type")]
    error_type: Option<String>,
}

impl OllamaProvider {
    /// Creates a new Ollama provider with the given host and model.
    ///
    /// Uses a 30-second timeout optimized for local model inference.
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn new(host: String, model: String) -> Result<Self, RuleyError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| RuleyError::Config(format!("Failed to create HTTP client: {}", e)))?;
        Ok(Self {
            host,
            model,
            client,
        })
    }

    /// Validates that `self.model` exists on the Ollama server.
    ///
    /// Calls `/api/tags` and checks that the configured model appears in the
    /// response. Returns a `RuleyError::Provider` with a pull suggestion when
    /// the model is missing.
    async fn validate_model(&self) -> Result<(), RuleyError> {
        let url = format!("{}/api/tags", self.host.trim_end_matches('/'));

        let response = self.client.get(&url).send().await.map_err(|e| {
            if e.is_connect() {
                RuleyError::Provider {
                    provider: "ollama".to_string(),
                    message: "Ollama server not running. Start with: ollama serve".to_string(),
                }
            } else {
                RuleyError::Provider {
                    provider: "ollama".to_string(),
                    message: format!("Failed to query Ollama models: {e}"),
                }
            }
        })?;

        let tags: OllamaTagsResponse = response.json().await.map_err(|e| RuleyError::Provider {
            provider: "ollama".to_string(),
            message: format!("Failed to parse model list: {e}"),
        })?;

        let model_exists = tags
            .models
            .iter()
            .any(|m| m.name == self.model || m.name.strip_suffix(":latest") == Some(&self.model));

        if !model_exists {
            return Err(RuleyError::Provider {
                provider: "ollama".to_string(),
                message: format!(
                    "Model '{}' not found. Pull with: ollama pull {}",
                    self.model, self.model
                ),
            });
        }

        Ok(())
    }

    /// Creates a new Ollama provider from environment variables.
    ///
    /// Reads the `OLLAMA_HOST` environment variable (default: `http://localhost:11434`).
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP client cannot be created.
    pub fn from_env() -> Result<Self, RuleyError> {
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Self::new(host, "llama3.1:70b".to_string())
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        self.validate_model().await?;

        let ollama_messages: Vec<OllamaMessage<'_>> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        let request_body = OllamaRequest {
            model: &self.model,
            messages: ollama_messages,
            max_tokens: Some(options.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS)),
            temperature: options.temperature,
        };

        let url = format!("{}/v1/chat/completions", self.host.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                if e.is_connect() {
                    RuleyError::Provider {
                        provider: "ollama".to_string(),
                        message: "Ollama server not running. Start with: ollama serve".to_string(),
                    }
                } else if e.is_timeout() {
                    RuleyError::Provider {
                        provider: "ollama".to_string(),
                        message:
                            "Local model processing timeout. Try a smaller model or increase timeout."
                                .to_string(),
                    }
                } else {
                    RuleyError::Provider {
                        provider: "ollama".to_string(),
                        message: format!("Failed to connect to Ollama: {}", e),
                    }
                }
            })?;

        let status = response.status();

        // Handle HTTP errors
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Check for model not found (404)
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(RuleyError::Provider {
                    provider: "ollama".to_string(),
                    message: format!(
                        "Model '{}' not found. Pull with: ollama pull {}",
                        self.model, self.model
                    ),
                });
            }

            // Try to parse structured error
            if let Ok(error) = serde_json::from_str::<OllamaError>(&error_text) {
                if let Some(detail) = error.error {
                    let msg = detail
                        .message
                        .unwrap_or_else(|| "Unknown error".to_string());
                    return Err(RuleyError::Provider {
                        provider: "ollama".to_string(),
                        message: msg,
                    });
                }
            }

            return Err(RuleyError::Provider {
                provider: "ollama".to_string(),
                message: format!("HTTP {}: {}", status, error_text),
            });
        }

        // Parse successful response
        let response_body: OllamaResponse =
            response.json().await.map_err(|e| RuleyError::Provider {
                provider: "ollama".to_string(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let content = response_body
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| RuleyError::Provider {
                provider: "ollama".to_string(),
                message: "LLM returned empty response content".to_string(),
            })?;

        let (prompt_tokens, completion_tokens) = match response_body.usage {
            Some(u) => (u.prompt_tokens, u.completion_tokens),
            None => {
                tracing::warn!("Ollama response missing usage data; token counts will be zero");
                (0, 0)
            }
        };

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
        Pricing {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
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

        let ollama_messages: Vec<OllamaMessage<'_>> = messages
            .iter()
            .map(|m| OllamaMessage {
                role: &m.role,
                content: &m.content,
            })
            .collect();

        assert_eq!(ollama_messages.len(), 2);
        assert_eq!(ollama_messages[0].role, "system");
        assert_eq!(ollama_messages[1].role, "user");
    }

    #[test]
    fn test_request_serialization() {
        let messages = vec![OllamaMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OllamaRequest {
            model: "llama3.1:70b",
            messages,
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(json.contains("\"model\":\"llama3.1:70b\""));
        assert!(json.contains("\"max_tokens\":1024"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn test_request_serialization_without_optional_fields() {
        let messages = vec![OllamaMessage {
            role: "user",
            content: "Hello",
        }];

        let request = OllamaRequest {
            model: "llama3.1:70b",
            messages,
            max_tokens: None,
            temperature: None,
        };

        let json = serde_json::to_string(&request).expect("serialization should succeed");
        assert!(!json.contains("max_tokens"));
        assert!(!json.contains("temperature"));
    }

    #[test]
    fn test_zero_pricing() {
        let provider = OllamaProvider::new(
            "http://localhost:11434".to_string(),
            "llama3.1:70b".to_string(),
        )
        .expect("should create provider");
        let pricing = provider.pricing();
        assert_eq!(pricing.input_per_1k, 0.0);
        assert_eq!(pricing.output_per_1k, 0.0);
    }

    #[test]
    fn test_url_trailing_slash_handling() {
        let host = "http://localhost:11434/";
        let url = format!("{}/v1/chat/completions", host.trim_end_matches('/'));
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
    }
}
