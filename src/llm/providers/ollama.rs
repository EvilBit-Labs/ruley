use crate::llm::provider::{CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing};
use crate::utils::error::RuleyError;
use async_trait::async_trait;

pub struct OllamaProvider {
    host: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(host: String, model: String) -> Self {
        Self { host, model }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let host =
            std::env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
        Ok(Self::new(host, "llama3.1:70b".to_string()))
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        // TODO: Implement Ollama API client
        todo!("Ollama provider not yet implemented")
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
