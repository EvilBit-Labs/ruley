use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;

pub struct OpenRouterProvider {
    #[allow(dead_code)] // Will be used when provider is implemented
    api_key: String,
    model: String,
}

impl OpenRouterProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| RuleyError::missing_api_key("openrouter"))?;
        Ok(Self::new(
            api_key,
            "anthropic/claude-3.5-sonnet".to_string(),
        ))
    }
}

#[async_trait]
impl LLMProvider for OpenRouterProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // TODO: Implement OpenRouter API client
        Err(RuleyError::Provider {
            provider: "openrouter".to_string(),
            message: "OpenRouter provider not yet implemented".to_string(),
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 0.003,  // Placeholder, varies by model
            output_per_1k: 0.015, // Placeholder, varies by model
        }
    }
}
