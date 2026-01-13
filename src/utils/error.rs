use std::sync::LazyLock;
use std::time::Duration;
use thiserror::Error;

/// Compiled regex patterns for redacting sensitive data.
/// Using LazyLock for thread-safe one-time initialization.
///
/// Note: These patterns are static compile-time constants that are validated by tests.
/// The expect() calls here are acceptable because:
/// 1. Patterns are known-valid literals, not runtime input
/// 2. Tests verify all patterns compile successfully
/// 3. Any regex error would be caught immediately at first use
static REDACTION_PATTERNS: LazyLock<[(regex::Regex, &'static str); 4]> = LazyLock::new(|| {
    [
        (
            regex::Regex::new(r"(api[_-]?key[=:\s]+)[^\s]+")
                .expect("api_key redaction pattern is invalid"),
            "${1}[REDACTED]",
        ),
        (
            regex::Regex::new(r"(token[=:\s]+)[^\s]+").expect("token redaction pattern is invalid"),
            "${1}[REDACTED]",
        ),
        (
            regex::Regex::new(r"(?i)(bearer\s+)[^\s]+")
                .expect("bearer redaction pattern is invalid"),
            "${1}[REDACTED]",
        ),
        (
            regex::Regex::new(r"(sk-[a-zA-Z0-9]{8,})")
                .expect("sk-key redaction pattern is invalid"),
            "[REDACTED]",
        ),
    ]
});

#[derive(Debug, Error)]
pub enum RuleyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Repository error: {0}")]
    Repository(#[from] git2::Error),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("LLM provider error: {provider} - {}", redact_sensitive_data(message))]
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

    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Validation error: {message}\nSuggestion: {suggestion}")]
    ValidationError { message: String, suggestion: String },

    #[error("Network error: {message}")]
    NetworkError {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Redact sensitive information from error messages.
fn redact_sensitive_data(message: &str) -> String {
    let mut result = message.to_string();
    for (pattern, replacement) in REDACTION_PATTERNS.iter() {
        result = pattern.replace_all(&result, *replacement).to_string();
    }
    result
}

impl RuleyError {
    pub fn invalid_format(format: &str) -> Self {
        RuleyError::ValidationError {
            message: format!("Invalid output format: '{}'", format),
            suggestion:
                "Valid formats are: cursor, claude, copilot, windsurf, aider, generic, json"
                    .to_string(),
        }
    }

    pub fn invalid_provider(provider: &str) -> Self {
        RuleyError::ValidationError {
            message: format!("Invalid provider: '{}'", provider),
            suggestion:
                "Valid providers are: anthropic, openai, ollama, openrouter, xai, groq, gemini"
                    .to_string(),
        }
    }

    pub fn invalid_chunk_size(size: usize) -> Self {
        RuleyError::ValidationError {
            message: format!("Invalid chunk size: {}", size),
            suggestion: "Chunk size must be between 1000 and 1000000 tokens".to_string(),
        }
    }

    pub fn missing_api_key(provider: &str) -> Self {
        let env_var = format!("{}_API_KEY", provider.to_uppercase());
        RuleyError::ValidationError {
            message: format!("API key not configured for provider '{}'", provider),
            suggestion: format!(
                "Set the {} environment variable or add it to your config file",
                env_var
            ),
        }
    }
}

impl From<serde_json::Error> for RuleyError {
    fn from(err: serde_json::Error) -> Self {
        RuleyError::ParseError {
            message: "Failed to parse JSON response".to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<toml::de::Error> for RuleyError {
    fn from(err: toml::de::Error) -> Self {
        RuleyError::ParseError {
            message: "Failed to parse TOML configuration".to_string(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<reqwest::Error> for RuleyError {
    fn from(err: reqwest::Error) -> Self {
        let message = if err.is_timeout() {
            "Request timed out. Check your network connection.".to_string()
        } else if err.is_connect() {
            "Failed to connect to server. Check your network connection.".to_string()
        } else if err.is_status() {
            format!(
                "HTTP error: {}",
                err.status()
                    .map_or("unknown".to_string(), |s| s.to_string())
            )
        } else {
            "Network request failed".to_string()
        };

        RuleyError::NetworkError {
            message,
            source: Some(Box::new(err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_data_redaction() {
        let message = "Error with api_key=sk-1234567890abcdefghij and token=secret123";
        let redacted = redact_sensitive_data(message);
        assert!(!redacted.contains("sk-1234567890abcdefghij"));
        assert!(!redacted.contains("secret123"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_bearer_redaction_variants() {
        // Case-insensitive bearer token redaction
        assert!(!redact_sensitive_data("Bearer abc123token").contains("abc123token"));
        assert!(!redact_sensitive_data("BEARER xyz789secret").contains("xyz789secret"));
        assert!(!redact_sensitive_data("bearer lowercase123").contains("lowercase123"));

        // Bearer with trailing content
        let msg = "Authorization: Bearer token123 and more text";
        let redacted = redact_sensitive_data(msg);
        assert!(!redacted.contains("token123"));
        assert!(redacted.contains("more text")); // Non-token content preserved
    }

    #[test]
    fn test_provider_error_redacts_api_key() {
        let err = RuleyError::Provider {
            provider: "anthropic".to_string(),
            message: "Failed with key sk-test123456789012345678901234".to_string(),
        };
        let msg = err.to_string();
        assert!(!msg.contains("sk-test123456789012345678901234"));
        assert!(msg.contains("[REDACTED]"));
    }

    #[test]
    fn test_missing_api_key_shows_env_var() {
        let err = RuleyError::missing_api_key("anthropic");
        let msg = err.to_string();
        assert!(msg.contains("ANTHROPIC_API_KEY"));
    }
}
