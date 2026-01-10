use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;

pub struct AnthropicProvider {
    #[allow(dead_code)]
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| RuleyError::Config("ANTHROPIC_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, "claude-sonnet-4-5-20250929".to_string()))
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // TODO: Implement Anthropic API client
        todo!("Anthropic provider not yet implemented")
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
