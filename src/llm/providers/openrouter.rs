use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;

pub struct OpenRouterProvider {
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
        todo!("OpenRouter provider not yet implemented")
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 3.0,
            output_per_1k: 15.0,
        }
    }
}
