use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuleyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Repository error: {0}")]
    Repository(#[from] git2::Error),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("LLM provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Rate limited by {provider}, retry after {retry_after:?}")]
    RateLimited {
        provider: String,
        retry_after: Option<Duration>,
    },

    #[error("Token limit exceeded: {tokens} tokens > {limit} limit")]
    TokenLimitExceeded { tokens: usize, limit: usize },

    #[error("Compression error for {language}: {message}")]
    Compression { language: String, message: String },

    #[error("Output format error: {0}")]
    OutputFormat(String),
}
