// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Provider-specific tokenizers for counting tokens in text.
//!
//! This module provides a trait-based abstraction for token counting,
//! with implementations for different LLM providers (OpenAI, Anthropic).
//!
//! # Example
//!
//! ```
//! use ruley::llm::tokenizer::{Tokenizer, TiktokenTokenizer, TokenizerModel};
//!
//! let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4o).unwrap();
//! let count = tokenizer.count_tokens("Hello, world!");
//! ```

use crate::packer::CompressedCodebase;
use crate::utils::error::RuleyError;
use tiktoken_rs::{cl100k_base, o200k_base};

/// Trait for counting tokens in text.
///
/// Different LLM providers use different tokenization schemes. This trait
/// provides a unified interface for token counting across providers.
pub trait Tokenizer: Send + Sync {
    /// Count the number of tokens in the given text.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to tokenize
    ///
    /// # Returns
    ///
    /// The number of tokens in the text according to this tokenizer.
    fn count_tokens(&self, text: &str) -> usize;
}

/// Model types that determine which encoding to use for tokenization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerModel {
    /// GPT-4, GPT-3.5-turbo, text-embedding-ada-002 (uses cl100k_base)
    Gpt4,
    /// GPT-4o, GPT-4o-mini (uses o200k_base)
    Gpt4o,
    /// Claude models (uses cl100k_base as approximation)
    Claude,
}

impl TokenizerModel {
    /// Create from a model name string.
    ///
    /// # Arguments
    ///
    /// * `model` - The model name (e.g., "gpt-4", "gpt-4o", "claude-3-opus")
    ///
    /// # Returns
    ///
    /// The appropriate `TokenizerModel` variant.
    pub fn from_model_name(model: &str) -> Self {
        let model_lower = model.to_lowercase();

        // GPT-4o variants use o200k_base
        if model_lower.contains("gpt-4o") || model_lower.contains("o1") {
            return Self::Gpt4o;
        }

        // Claude models use cl100k_base as approximation
        if model_lower.contains("claude") {
            return Self::Claude;
        }

        // GPT-4 and GPT-3.5 variants use cl100k_base
        if model_lower.contains("gpt-4") || model_lower.contains("gpt-3.5") {
            return Self::Gpt4;
        }

        // Default to Gpt4 (cl100k_base) for unknown models
        Self::Gpt4
    }
}

/// Tokenizer using tiktoken for OpenAI models.
///
/// Uses the appropriate encoding based on the model:
/// - cl100k_base: GPT-4, GPT-3.5-turbo
/// - o200k_base: GPT-4o, GPT-4o-mini
pub struct TiktokenTokenizer {
    encoding: tiktoken_rs::CoreBPE,
}

impl TiktokenTokenizer {
    /// Create a new tiktoken tokenizer for the specified model type.
    ///
    /// # Arguments
    ///
    /// * `model` - The model type determining which encoding to use
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding cannot be loaded.
    pub fn new(model: TokenizerModel) -> Result<Self, RuleyError> {
        let encoding = match model {
            TokenizerModel::Gpt4 | TokenizerModel::Claude => {
                cl100k_base().map_err(|e| RuleyError::Config(e.to_string()))?
            }
            TokenizerModel::Gpt4o => o200k_base().map_err(|e| RuleyError::Config(e.to_string()))?,
        };

        Ok(Self { encoding })
    }

    /// Create a tiktoken tokenizer from a model name string.
    ///
    /// # Arguments
    ///
    /// * `model_name` - The model name (e.g., "gpt-4", "gpt-4o")
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding cannot be loaded.
    pub fn from_model_name(model_name: &str) -> Result<Self, RuleyError> {
        let model = TokenizerModel::from_model_name(model_name);
        Self::new(model)
    }
}

impl Tokenizer for TiktokenTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.encoding.encode_with_special_tokens(text).len()
    }
}

/// Tokenizer for Anthropic Claude models.
///
/// Uses cl100k_base encoding as a reasonable approximation, since there is
/// no official Anthropic tokenizer in Rust. Claude's tokenization is similar
/// to GPT-4's tokenization.
///
/// This is a thin wrapper around `TiktokenTokenizer` configured for Claude models.
pub struct AnthropicTokenizer(TiktokenTokenizer);

impl AnthropicTokenizer {
    /// Create a new Anthropic tokenizer.
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding cannot be loaded.
    pub fn new() -> Result<Self, RuleyError> {
        Ok(Self(TiktokenTokenizer::new(TokenizerModel::Claude)?))
    }
}

impl Tokenizer for AnthropicTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.0.count_tokens(text)
    }
}

/// Calculate the total token count for a compressed codebase.
///
/// This function counts tokens in all compressed file contents and any
/// metadata that would be sent to the LLM.
///
/// # Arguments
///
/// * `codebase` - The compressed codebase to count tokens for
/// * `tokenizer` - The tokenizer to use for counting
///
/// # Returns
///
/// The total number of tokens in the codebase's compressed content.
///
/// # Example
///
/// ```ignore
/// let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4o)?;
/// let tokens = calculate_tokens(&compressed_codebase, &tokenizer);
/// println!("Total tokens: {}", tokens);
/// ```
pub fn calculate_tokens(codebase: &CompressedCodebase, tokenizer: &dyn Tokenizer) -> usize {
    codebase
        .files
        .iter()
        .map(|file| {
            // Count tokens in the file path (as it's typically included in prompts)
            let path_tokens = tokenizer.count_tokens(&file.path.to_string_lossy());
            // Count tokens in the compressed content
            let content_tokens = tokenizer.count_tokens(&file.compressed_content);
            path_tokens + content_tokens
        })
        .sum()
}

/// Legacy TokenCounter struct for backward compatibility.
///
/// This struct is kept for compatibility with existing code. For new code,
/// prefer using the `Tokenizer` trait with `TiktokenTokenizer` or
/// `AnthropicTokenizer`.
pub struct TokenCounter {
    encoding: tiktoken_rs::CoreBPE,
}

impl TokenCounter {
    /// Create a new TokenCounter with the specified encoding.
    ///
    /// # Arguments
    ///
    /// * `encoding_name` - The encoding name ("cl100k_base" or "o200k_base")
    ///
    /// # Errors
    ///
    /// Returns an error if the encoding name is unknown or cannot be loaded.
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

    /// Count tokens in the given text.
    pub fn count(&self, text: &str) -> usize {
        self.encoding.encode_with_special_tokens(text).len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packer::{CodebaseMetadata, CompressedFile, CompressionMethod};
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_tokenizer_model_from_model_name() {
        // GPT-4o variants
        assert_eq!(
            TokenizerModel::from_model_name("gpt-4o"),
            TokenizerModel::Gpt4o
        );
        assert_eq!(
            TokenizerModel::from_model_name("gpt-4o-mini"),
            TokenizerModel::Gpt4o
        );
        assert_eq!(
            TokenizerModel::from_model_name("o1-preview"),
            TokenizerModel::Gpt4o
        );

        // GPT-4 variants
        assert_eq!(
            TokenizerModel::from_model_name("gpt-4"),
            TokenizerModel::Gpt4
        );
        assert_eq!(
            TokenizerModel::from_model_name("gpt-4-turbo"),
            TokenizerModel::Gpt4
        );
        assert_eq!(
            TokenizerModel::from_model_name("gpt-3.5-turbo"),
            TokenizerModel::Gpt4
        );

        // Claude variants
        assert_eq!(
            TokenizerModel::from_model_name("claude-3-opus"),
            TokenizerModel::Claude
        );
        assert_eq!(
            TokenizerModel::from_model_name("claude-sonnet-4-5-20250929"),
            TokenizerModel::Claude
        );

        // Unknown defaults to Gpt4
        assert_eq!(
            TokenizerModel::from_model_name("unknown-model"),
            TokenizerModel::Gpt4
        );
    }

    #[test]
    fn test_tiktoken_tokenizer_gpt4() {
        let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4).unwrap();
        let count = tokenizer.count_tokens("Hello, world!");
        assert!(count > 0);
        // "Hello, world!" typically tokenizes to 4 tokens in cl100k_base
        assert!((3..=6).contains(&count));
    }

    #[test]
    fn test_tiktoken_tokenizer_gpt4o() {
        let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4o).unwrap();
        let count = tokenizer.count_tokens("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_tiktoken_tokenizer_from_model_name() {
        let tokenizer = TiktokenTokenizer::from_model_name("gpt-4o-mini").unwrap();
        let count = tokenizer.count_tokens("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_anthropic_tokenizer() {
        let tokenizer = AnthropicTokenizer::new().unwrap();
        let count = tokenizer.count_tokens("Hello, world!");
        assert!(count > 0);
        // Should be similar to cl100k_base
        assert!((3..=6).contains(&count));
    }

    #[test]
    fn test_calculate_tokens_empty_codebase() {
        let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4).unwrap();
        let codebase = CompressedCodebase {
            files: vec![],
            metadata: CodebaseMetadata {
                total_files: 0,
                total_original_size: 0,
                total_compressed_size: 0,
                languages: HashMap::new(),
                compression_ratio: 0.0,
            },
        };

        let tokens = calculate_tokens(&codebase, &tokenizer);
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_calculate_tokens_with_files() {
        let tokenizer = TiktokenTokenizer::new(TokenizerModel::Gpt4).unwrap();

        let files = vec![
            CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() { println!(\"Hello, world!\"); }".to_string(),
                compressed_content: "fn main() { println!(\"Hello\"); }".to_string(),
                compression_method: CompressionMethod::TreeSitter,
                original_size: 41,
                compressed_size: 32,
                language: None,
            },
            CompressedFile {
                path: PathBuf::from("src/lib.rs"),
                original_content: "pub mod utils;".to_string(),
                compressed_content: "pub mod utils;".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 14,
                compressed_size: 14,
                language: None,
            },
        ];

        let codebase = CompressedCodebase::new(files);
        let tokens = calculate_tokens(&codebase, &tokenizer);

        // Should have tokens from both file paths and contents
        assert!(tokens > 0);
    }

    #[test]
    fn test_tokenizer_trait_object() {
        // Ensure tokenizers can be used as trait objects
        let openai_tokenizer: Box<dyn Tokenizer> =
            Box::new(TiktokenTokenizer::new(TokenizerModel::Gpt4).unwrap());
        let anthropic_tokenizer: Box<dyn Tokenizer> = Box::new(AnthropicTokenizer::new().unwrap());

        let text = "This is a test sentence.";
        let openai_count = openai_tokenizer.count_tokens(text);
        let anthropic_count = anthropic_tokenizer.count_tokens(text);

        assert!(openai_count > 0);
        assert!(anthropic_count > 0);
        // Both use cl100k_base so should be equal
        assert_eq!(openai_count, anthropic_count);
    }

    #[test]
    fn test_legacy_token_counter() {
        let counter = TokenCounter::new("cl100k_base").unwrap();
        let count = counter.count("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_legacy_token_counter_o200k() {
        let counter = TokenCounter::new("o200k_base").unwrap();
        let count = counter.count("Hello, world!");
        assert!(count > 0);
    }

    #[test]
    fn test_legacy_token_counter_invalid_encoding() {
        let result = TokenCounter::new("invalid_encoding");
        assert!(result.is_err());
    }
}
