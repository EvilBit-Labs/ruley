use crate::utils::error::RuleyError;
use tiktoken_rs::{cl100k_base, o200k_base};

pub struct TokenCounter {
    encoding: tiktoken_rs::CoreBPE,
}

impl TokenCounter {
    pub fn new(encoding_name: &str) -> Result<Self, RuleyError> {
        let encoding = match encoding_name {
            "cl100k_base" => cl100k_base().map_err(|e| RuleyError::Config(e.to_string()))?,
            "o200k_base" => o200k_base().map_err(|e| RuleyError::Config(e.to_string()))?,
            _ => {
                return Err(RuleyError::Config(format!(
                    "Unknown encoding name '{}'. Supported encodings: cl100k_base, o200k_base",
                    encoding_name
                )));
            }
        };

        Ok(Self { encoding })
    }

    pub fn count(&self, text: &str) -> usize {
        self.encoding.encode_with_special_tokens(text).len()
    }
}
