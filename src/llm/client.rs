use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message};
use crate::utils::error::RuleyError;

pub struct LLMClient {
    provider: Box<dyn LLMProvider>,
}

impl LLMClient {
    pub fn new(provider: Box<dyn LLMProvider>) -> Self {
        Self { provider }
    }

    pub async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        self.provider.complete(messages, options).await
    }

    pub fn model(&self) -> &str {
        self.provider.model()
    }

    pub fn pricing(&self) -> crate::llm::provider::Pricing {
        self.provider.pricing()
    }
}
