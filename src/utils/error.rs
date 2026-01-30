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

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("State error: {0}")]
    State(String),
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

/// Format a `RuleyError` with contextual information and suggestions.
///
/// Provides user-friendly error output with:
/// - Clear error type identification
/// - Structured "What happened" section with context
/// - Actionable suggestions for resolution
/// - Optional verbose mode for full error chain
///
/// # Arguments
///
/// * `error` - The error to format
/// * `verbose` - If true, includes full error chain and debug info
///
/// # Returns
///
/// A formatted error string ready for display.
pub fn format_error(error: &RuleyError, verbose: bool) -> String {
    use std::fmt::Write;

    let mut output = String::new();

    // Error header with warning symbol
    let _ = writeln!(output, "\n\u{26a0} Error: {}", error_title(error));

    // What happened section
    let _ = writeln!(output, "\nWhat happened:");
    for (i, line) in error_context_lines(error).iter().enumerate() {
        let prefix = if i == error_context_lines(error).len() - 1 {
            "\u{2514}\u{2500}"
        } else {
            "\u{251c}\u{2500}"
        };
        let _ = writeln!(output, "{} {}", prefix, line);
    }

    // Suggestions section
    let suggestions = error_suggestions(error);
    if !suggestions.is_empty() {
        let _ = writeln!(output, "\n{}:", suggestion_header(error));
        for suggestion in suggestions {
            let _ = writeln!(output, "\u{2022} {}", suggestion);
        }
    }

    // Verbose mode: show full error chain
    if verbose {
        let _ = writeln!(output, "\nDebug info:");
        let _ = writeln!(output, "{:?}", error);

        // Show source chain if available
        if let Some(source) = get_error_source(error) {
            let _ = writeln!(output, "\nCaused by:");
            let _ = writeln!(output, "  {}", source);
        }
    } else if has_source(error) {
        let _ = writeln!(output, "\nFor more details, run with --verbose");
    }

    output
}

/// Get a short title for the error type.
fn error_title(error: &RuleyError) -> &'static str {
    match error {
        RuleyError::Config(_) => "Configuration error",
        RuleyError::Repository(_) => "Repository error",
        RuleyError::FileSystem(_) => "File system error",
        RuleyError::Provider { .. } => "LLM provider error",
        RuleyError::RateLimited { .. } => "Rate limit exceeded",
        RuleyError::TokenLimitExceeded { .. } => "Codebase too large",
        RuleyError::Compression { .. } => "Compression error",
        RuleyError::OutputFormat(_) => "Output format error",
        RuleyError::ParseError { .. } => "Parse error",
        RuleyError::ValidationError { .. } => "Validation error",
        RuleyError::NetworkError { .. } => "Network error",
        RuleyError::Cache(_) => "Cache error",
        RuleyError::State(_) => "State error",
    }
}

/// Get context lines explaining what happened.
fn error_context_lines(error: &RuleyError) -> Vec<String> {
    match error {
        RuleyError::Config(msg) => {
            vec![
                "Stage: Loading configuration".to_string(),
                format!("Error: {}", msg),
            ]
        }
        RuleyError::Repository(err) => {
            vec![
                "Stage: Accessing git repository".to_string(),
                format!("Error: {}", err),
            ]
        }
        RuleyError::FileSystem(err) => {
            vec![
                "Stage: File system operation".to_string(),
                format!("Error: {}", err),
            ]
        }
        RuleyError::Provider { provider, message } => {
            vec![
                format!("Stage: Communicating with {} LLM", provider),
                format!("Error: {}", redact_sensitive_data(message)),
            ]
        }
        RuleyError::RateLimited {
            provider,
            retry_after,
        } => {
            let mut lines = vec![
                format!("Stage: Analyzing codebase with {}", provider),
                "Error: Rate limit exceeded (429)".to_string(),
            ];
            if let Some(duration) = retry_after {
                lines.push(format!("Retry after: {} seconds", duration.as_secs()));
            }
            lines
        }
        RuleyError::TokenLimitExceeded { tokens, limit } => {
            vec![
                "Stage: Analyzing codebase".to_string(),
                format!("Tokens: {}", format_number(*tokens)),
                format!("Context limit: {} tokens", format_number(*limit)),
            ]
        }
        RuleyError::Compression { language, message } => {
            vec![
                format!("Stage: Compressing {} files", language),
                format!("Error: {}", message),
            ]
        }
        RuleyError::OutputFormat(msg) => {
            vec![
                "Stage: Generating output".to_string(),
                format!("Error: {}", msg),
            ]
        }
        RuleyError::ParseError { message, .. } => {
            vec![
                "Stage: Parsing response".to_string(),
                format!("Error: {}", message),
            ]
        }
        RuleyError::ValidationError { message, .. } => {
            vec![
                "Stage: Validating input".to_string(),
                format!("Error: {}", message),
            ]
        }
        RuleyError::NetworkError { message, .. } => {
            vec![
                "Stage: Network communication".to_string(),
                format!("Error: {}", message),
            ]
        }
        RuleyError::Cache(msg) => {
            vec![
                "Stage: Cache operation".to_string(),
                format!("Error: {}", msg),
            ]
        }
        RuleyError::State(msg) => {
            vec![
                "Stage: State management".to_string(),
                format!("Error: {}", msg),
            ]
        }
    }
}

/// Get the appropriate header for suggestions section.
fn suggestion_header(error: &RuleyError) -> &'static str {
    match error {
        RuleyError::ValidationError { .. } => "How to fix",
        RuleyError::RateLimited { .. } => "Suggestion",
        RuleyError::TokenLimitExceeded { .. } => "Suggestion",
        RuleyError::Provider { .. } => "How to fix",
        _ => "Suggestion",
    }
}

/// Get actionable suggestions for the error.
fn error_suggestions(error: &RuleyError) -> Vec<String> {
    match error {
        RuleyError::Config(msg) => {
            if msg.contains("not found") || msg.contains("missing") {
                vec![
                    "Create a ruley.toml config file in your project root".to_string(),
                    "Or run without a config file to use defaults".to_string(),
                ]
            } else {
                vec!["Check your ruley.toml syntax and values".to_string()]
            }
        }
        RuleyError::Repository(_) => {
            vec![
                "Ensure you're running ruley from a git repository".to_string(),
                "Or specify a path to a valid git repository".to_string(),
            ]
        }
        RuleyError::FileSystem(err) => {
            if err.kind() == std::io::ErrorKind::PermissionDenied {
                vec!["Check file permissions for the target directory".to_string()]
            } else if err.kind() == std::io::ErrorKind::NotFound {
                vec!["Verify the path exists and is accessible".to_string()]
            } else {
                vec!["Check disk space and file system permissions".to_string()]
            }
        }
        RuleyError::Provider { provider, message } => {
            if message.to_lowercase().contains("api key")
                || message.to_lowercase().contains("unauthorized")
                || message.to_lowercase().contains("401")
            {
                let env_var = format!("{}_API_KEY", provider.to_uppercase());
                vec![
                    format!("Set the {} environment variable:", env_var),
                    format!("  export {}=your-key-here", env_var),
                    get_provider_url(provider),
                ]
            } else if message.contains("timeout") {
                vec![
                    "Check your network connection".to_string(),
                    "Try again in a few moments".to_string(),
                ]
            } else {
                vec![
                    "Check the provider's status page for outages".to_string(),
                    format!("Try --provider <other> to use a different provider"),
                ]
            }
        }
        RuleyError::RateLimited {
            retry_after,
            provider,
        } => {
            let wait_suggestion = retry_after
                .map(|d| format!("Wait {} seconds and try again", d.as_secs()))
                .unwrap_or_else(|| "Wait 60 seconds and try again".to_string());
            vec![
                wait_suggestion,
                format!("Or use --provider <other> to switch providers"),
                "Or reduce scope with --include patterns".to_string(),
                format!("Check {}'s rate limits and usage", provider),
            ]
        }
        RuleyError::TokenLimitExceeded { .. } => {
            vec![
                "Use --include patterns to reduce scope".to_string(),
                r#"  Example: ruley --include "src/**/*.ts""#.to_string(),
                "Or use --compress to enable tree-sitter compression".to_string(),
                "Or increase chunk size with --chunk-size".to_string(),
            ]
        }
        RuleyError::Compression { language, .. } => {
            vec![
                format!("Try disabling compression with --no-compress"),
                format!("Or exclude {} files with --exclude patterns", language),
            ]
        }
        RuleyError::OutputFormat(msg) => {
            if msg.contains("exists") || msg.contains("overwrite") {
                vec![
                    "Use --force to overwrite existing files".to_string(),
                    "Or specify a different output path with --output".to_string(),
                ]
            } else {
                vec!["Check the output format name and try again".to_string()]
            }
        }
        RuleyError::ParseError { .. } => {
            vec![
                "The LLM response was malformed".to_string(),
                "Try again - LLM outputs can vary".to_string(),
                "If the problem persists, try a different model".to_string(),
            ]
        }
        RuleyError::ValidationError { suggestion, .. } => {
            vec![suggestion.clone()]
        }
        RuleyError::NetworkError { message, .. } => {
            if message.contains("timeout") {
                vec![
                    "Check your network connection".to_string(),
                    "The LLM provider may be experiencing high load".to_string(),
                    "Try again in a few moments".to_string(),
                ]
            } else if message.contains("connect") {
                vec![
                    "Check your internet connection".to_string(),
                    "Verify the provider's API endpoint is accessible".to_string(),
                    "Check if a proxy or firewall is blocking the connection".to_string(),
                ]
            } else {
                vec![
                    "Check your network connection".to_string(),
                    "Try again in a few moments".to_string(),
                ]
            }
        }
        RuleyError::Cache(msg) => {
            if msg.contains("corrupt") || msg.contains("invalid") {
                vec![
                    "Delete the .ruley directory and run again".to_string(),
                    "  rm -rf .ruley".to_string(),
                ]
            } else {
                vec!["Check disk space and permissions".to_string()]
            }
        }
        RuleyError::State(msg) => {
            if msg.contains("version") || msg.contains("migrate") {
                vec![
                    "Your state file may be from an older version".to_string(),
                    "Delete .ruley/state.json to reset state".to_string(),
                ]
            } else {
                vec!["Delete .ruley/state.json and try again".to_string()]
            }
        }
    }
}

/// Get the sign-up/docs URL for a provider.
fn get_provider_url(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "anthropic" => "Get your key at: https://console.anthropic.com/".to_string(),
        "openai" => "Get your key at: https://platform.openai.com/api-keys".to_string(),
        "openrouter" => "Get your key at: https://openrouter.ai/keys".to_string(),
        "xai" => "Get your key at: https://x.ai/".to_string(),
        "groq" => "Get your key at: https://console.groq.com/keys".to_string(),
        "gemini" => "Get your key at: https://aistudio.google.com/apikey".to_string(),
        _ => format!("Check the {} documentation for API key setup", provider),
    }
}

/// Check if the error has a source error.
fn has_source(error: &RuleyError) -> bool {
    matches!(
        error,
        RuleyError::ParseError {
            source: Some(_),
            ..
        } | RuleyError::NetworkError {
            source: Some(_),
            ..
        } | RuleyError::Repository(_)
            | RuleyError::FileSystem(_)
    )
}

/// Get the source error if available.
fn get_error_source(error: &RuleyError) -> Option<String> {
    match error {
        RuleyError::ParseError {
            source: Some(src), ..
        } => Some(src.to_string()),
        RuleyError::NetworkError {
            source: Some(src), ..
        } => Some(src.to_string()),
        RuleyError::Repository(err) => Some(err.to_string()),
        RuleyError::FileSystem(err) => Some(err.to_string()),
        _ => None,
    }
}

/// Format a number with thousand separators.
fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<_> = s.chars().collect();
    let len = chars.len();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }

    result
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

    #[test]
    fn test_cache_error_display_format() {
        let err = RuleyError::Cache("test cache error".to_string());
        let msg = err.to_string();
        assert!(
            msg.contains("Cache error:"),
            "Should contain 'Cache error:'"
        );
        assert!(
            msg.contains("test cache error"),
            "Should contain the error message"
        );
    }

    #[test]
    fn test_format_error_config() {
        let err = RuleyError::Config("Invalid provider name".to_string());
        let formatted = format_error(&err, false);

        assert!(formatted.contains("\u{26a0} Error:"));
        assert!(formatted.contains("Configuration error"));
        assert!(formatted.contains("What happened:"));
        assert!(formatted.contains("Loading configuration"));
        assert!(formatted.contains("Invalid provider name"));
    }

    #[test]
    fn test_format_error_rate_limited() {
        let err = RuleyError::RateLimited {
            provider: "anthropic".to_string(),
            retry_after: Some(Duration::from_secs(60)),
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Rate limit exceeded"));
        assert!(formatted.contains("Analyzing codebase with anthropic"));
        assert!(formatted.contains("Wait 60 seconds"));
        assert!(formatted.contains("--include patterns"));
    }

    #[test]
    fn test_format_error_token_limit() {
        let err = RuleyError::TokenLimitExceeded {
            tokens: 456789,
            limit: 200000,
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Codebase too large"));
        assert!(formatted.contains("456,789"));
        assert!(formatted.contains("200,000"));
        assert!(formatted.contains("--include patterns"));
        assert!(formatted.contains("--compress"));
    }

    #[test]
    fn test_format_error_provider_api_key() {
        let err = RuleyError::Provider {
            provider: "openai".to_string(),
            message: "401 Unauthorized - invalid API key".to_string(),
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("LLM provider error"));
        assert!(formatted.contains("OPENAI_API_KEY"));
        assert!(formatted.contains("platform.openai.com"));
    }

    #[test]
    fn test_format_error_verbose_mode() {
        let err = RuleyError::ParseError {
            message: "Invalid JSON".to_string(),
            source: Some(Box::new(std::io::Error::other("test error"))),
        };
        let formatted = format_error(&err, true);

        assert!(formatted.contains("Debug info:"));
        assert!(formatted.contains("Caused by:"));
        assert!(formatted.contains("test error"));
    }

    #[test]
    fn test_format_error_non_verbose_shows_hint() {
        let err = RuleyError::ParseError {
            message: "Invalid JSON".to_string(),
            source: Some(Box::new(std::io::Error::other("test"))),
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("For more details, run with --verbose"));
        assert!(!formatted.contains("Debug info:"));
    }

    #[test]
    fn test_format_error_validation() {
        let err = RuleyError::ValidationError {
            message: "Invalid format 'foo'".to_string(),
            suggestion: "Valid formats are: cursor, claude, copilot".to_string(),
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Validation error"));
        assert!(formatted.contains("How to fix"));
        assert!(formatted.contains("Valid formats are:"));
    }

    #[test]
    fn test_format_error_network() {
        let err = RuleyError::NetworkError {
            message: "Connection timeout".to_string(),
            source: None,
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Network error"));
        assert!(formatted.contains("Check your network connection"));
    }

    #[test]
    fn test_format_error_compression() {
        let err = RuleyError::Compression {
            language: "TypeScript".to_string(),
            message: "Parser initialization failed".to_string(),
        };
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Compression error"));
        assert!(formatted.contains("Compressing TypeScript files"));
        assert!(formatted.contains("--no-compress"));
    }

    #[test]
    fn test_format_number_helper() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_get_provider_url() {
        assert!(get_provider_url("anthropic").contains("console.anthropic.com"));
        assert!(get_provider_url("openai").contains("platform.openai.com"));
        assert!(get_provider_url("groq").contains("console.groq.com"));
        assert!(get_provider_url("unknown").contains("documentation"));
    }

    #[test]
    fn test_error_title_all_variants() {
        // Ensure all error variants have titles
        assert_eq!(
            error_title(&RuleyError::Config("test".to_string())),
            "Configuration error"
        );
        assert_eq!(
            error_title(&RuleyError::Cache("test".to_string())),
            "Cache error"
        );
        assert_eq!(
            error_title(&RuleyError::State("test".to_string())),
            "State error"
        );
        assert_eq!(
            error_title(&RuleyError::OutputFormat("test".to_string())),
            "Output format error"
        );
    }

    #[test]
    fn test_format_error_cache_corrupt() {
        let err = RuleyError::Cache("corrupted cache file".to_string());
        let formatted = format_error(&err, false);

        assert!(formatted.contains("Cache error"));
        assert!(formatted.contains("Delete the .ruley directory"));
        assert!(formatted.contains("rm -rf .ruley"));
    }

    #[test]
    fn test_format_error_state_version() {
        let err = RuleyError::State("version mismatch during migration".to_string());
        let formatted = format_error(&err, false);

        assert!(formatted.contains("State error"));
        assert!(formatted.contains("older version"));
        assert!(formatted.contains("state.json"));
    }
}
