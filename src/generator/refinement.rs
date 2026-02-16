//! Iterative refinement module for auto-fixing validation errors.
//!
//! When validation fails and `retry_on_failure` is enabled, this module
//! sends the invalid output back to the LLM with specific error information
//! and asks it to produce corrected output.
//!
//! Retry logic uses increasing temperature (0.7 -> 0.9) for creativity
//! on subsequent attempts.

use crate::llm::client::LLMClient;
use crate::llm::cost::CostTracker;
use crate::llm::provider::{CompletionOptions, Message};
use crate::utils::validation::ValidationError;
use anyhow::{Context, Result};

/// Tracks a single fix attempt during iterative refinement.
#[derive(Debug, Clone)]
pub struct FixAttempt {
    /// Which attempt number (1-indexed)
    pub attempt_number: usize,
    /// The errors that were being fixed
    pub errors: Vec<String>,
    /// Cost of this attempt in dollars
    pub cost: f64,
}

/// Result of the iterative refinement process.
#[derive(Debug, Clone)]
pub struct RefinementResult {
    /// Whether refinement succeeded (determined by caller's validation)
    pub success: bool,
    /// All fix attempts made
    pub attempts: Vec<FixAttempt>,
    /// Total cost of all refinement attempts
    pub total_cost: f64,
    /// Whether all retry attempts have been exhausted
    pub retries_exhausted: bool,
}

/// Attempt to fix invalid output by sending it back to the LLM with error details.
///
/// Performs a single refinement attempt with temperature scaled by attempt number.
/// The caller is responsible for looping over attempts and re-validating between calls.
///
/// Temperature scaling: starts at 0.7 and increases by 0.1 per attempt, capped at 0.9.
///
/// # Arguments
///
/// * `invalid_output` - The output that failed validation (latest from previous attempt)
/// * `errors` - The validation errors that need fixing
/// * `format` - The output format (e.g., "cursor", "claude")
/// * `client` - LLM client for making fix requests
/// * `cost_tracker` - Cost tracker for recording fix costs
/// * `attempt` - Current attempt number (1-indexed)
/// * `max_retries` - Maximum number of fix attempts
///
/// # Returns
///
/// A tuple of (fixed_content, refinement_result). The `success` field is `false` by
/// default; the caller should set it to `true` after validation passes.
/// `retries_exhausted` indicates whether this was the final allowed attempt.
pub async fn refine_invalid_output(
    invalid_output: &str,
    errors: &[ValidationError],
    format: &str,
    client: &LLMClient,
    cost_tracker: &mut Option<CostTracker>,
    attempt: usize,
    max_retries: usize,
) -> Result<(String, RefinementResult)> {
    // Temperature increases per attempt: 0.7, 0.8, 0.9 (capped)
    let temperature = (0.7 + (attempt as f32 - 1.0) * 0.1).min(0.9);

    tracing::info!(
        "Refinement attempt {}/{} for {} format ({} errors, temp={:.1})",
        attempt,
        max_retries,
        format,
        errors.len(),
        temperature
    );

    let prompt = build_fix_prompt(invalid_output, errors, format);

    let options = CompletionOptions {
        temperature: Some(temperature),
        ..CompletionOptions::default()
    };

    let messages = vec![Message {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = client
        .complete(&messages, &options)
        .await
        .with_context(|| format!("Failed to refine {} format (attempt {})", format, attempt))?;

    // Track cost
    let attempt_cost = if let Some(tracker) = cost_tracker {
        let cost_before = tracker.total_cost();
        tracker.add_operation(
            format!("refinement_{}_{}", format, attempt),
            response.prompt_tokens,
            response.completion_tokens,
        );
        tracker.total_cost() - cost_before
    } else {
        0.0
    };

    let fix_attempt = FixAttempt {
        attempt_number: attempt,
        errors: errors.iter().map(|e| e.message.clone()).collect(),
        cost: attempt_cost,
    };

    tracing::info!(
        "Refinement attempt {}/{} completed (cost: ${:.4})",
        attempt,
        max_retries,
        attempt_cost
    );

    Ok((
        response.content,
        RefinementResult {
            success: false, // Caller validates and sets to true if passes
            attempts: vec![fix_attempt],
            total_cost: attempt_cost,
            retries_exhausted: attempt >= max_retries,
        },
    ))
}

/// Build the fix prompt for the LLM.
fn build_fix_prompt(invalid_output: &str, errors: &[ValidationError], format: &str) -> String {
    let error_list: String = errors
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let mut desc = format!("{}. [{}] {}", i + 1, e.layer, e.message);
            if let Some(ref loc) = e.location {
                desc.push_str(&format!(" (at {})", loc));
            }
            if let Some(ref sug) = e.suggestion {
                desc.push_str(&format!("\n   Suggestion: {}", sug));
            }
            desc
        })
        .collect::<Vec<_>>()
        .join("\n");

    let format_requirements = get_format_requirements(format);

    format!(
        r#"You generated {} format rules that have validation errors. Please fix them.

<original_output>
{}
</original_output>

<validation_errors>
{}
</validation_errors>

<format_requirements>
{}
</format_requirements>

Please generate corrected output that fixes these specific issues while preserving the original intent and content. Return only the corrected output, nothing else."#,
        format, invalid_output, error_list, format_requirements
    )
}

/// Get format-specific requirements for the fix prompt.
fn get_format_requirements(format: &str) -> &'static str {
    match format {
        "cursor" => {
            "Cursor .mdc format: Optional YAML frontmatter (---...---) with description, globs, alwaysApply fields. Markdown body with rules. Properly closed code blocks."
        }
        "claude" => {
            "Claude CLAUDE.md format: Markdown with section headings (# and ##). Must include project overview and coding standards sections."
        }
        "copilot" => {
            "GitHub Copilot format: Markdown file with coding instructions. Non-empty content required."
        }
        "windsurf" => "Windsurf format: Markdown rules file. Non-empty content required.",
        "aider" => {
            "Aider CONVENTIONS.md format: Markdown conventions file. Non-empty content required."
        }
        "generic" => {
            "Generic AI_RULES.md format: Markdown rules file with section headings. Non-empty content required."
        }
        "json" => "JSON format: Must be valid JSON. Should be a non-empty object with rules data.",
        _ => "Standard format: Valid Markdown with proper structure.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::validation::ValidationLayer;

    #[test]
    fn test_build_fix_prompt_contains_errors() {
        let errors = vec![
            ValidationError {
                layer: ValidationLayer::Syntax,
                message: "Unclosed code block".to_string(),
                location: Some("line 15".to_string()),
                suggestion: Some("Add closing ```".to_string()),
            },
            ValidationError {
                layer: ValidationLayer::Schema,
                message: "Missing heading".to_string(),
                location: None,
                suggestion: None,
            },
        ];

        let prompt = build_fix_prompt("# Invalid content\n```rust\nfn main()", &errors, "claude");
        assert!(prompt.contains("Unclosed code block"));
        assert!(prompt.contains("Missing heading"));
        assert!(prompt.contains("line 15"));
        assert!(prompt.contains("Add closing ```"));
        assert!(prompt.contains("claude"));
    }

    #[test]
    fn test_get_format_requirements() {
        assert!(get_format_requirements("cursor").contains("frontmatter"));
        assert!(get_format_requirements("claude").contains("CLAUDE.md"));
        assert!(get_format_requirements("json").contains("JSON"));
        assert!(get_format_requirements("unknown").contains("Markdown"));
    }

    #[test]
    fn test_fix_attempt_creation() {
        let attempt = FixAttempt {
            attempt_number: 1,
            errors: vec!["test error".to_string()],
            cost: 0.01,
        };
        assert_eq!(attempt.attempt_number, 1);
        assert_eq!(attempt.errors.len(), 1);
    }

    #[test]
    fn test_refinement_result_creation() {
        let result = RefinementResult {
            success: false,
            attempts: vec![],
            total_cost: 0.0,
            retries_exhausted: false,
        };
        assert!(!result.success);
        assert!(result.attempts.is_empty());
        assert!(!result.retries_exhausted);
    }

    #[test]
    fn test_refinement_result_retries_exhausted() {
        let result = RefinementResult {
            success: false,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["error".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec!["error".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 3,
                    errors: vec!["error".to_string()],
                    cost: 0.01,
                },
            ],
            total_cost: 0.03,
            retries_exhausted: true,
        };
        assert!(!result.success);
        assert_eq!(result.attempts.len(), 3);
        assert!(result.retries_exhausted);
    }
}
