use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;

pub struct OpenAIProvider {
    #[allow(dead_code)]
    api_key: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self { api_key, model }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| RuleyError::Config("OPENAI_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, "gpt-4o".to_string()))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // TODO: Implement OpenAI API client
        todo!("OpenAI provider not yet implemented")
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
