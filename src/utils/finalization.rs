//! Finalization module for post-processing generated rules.
//!
//! Handles:
//! - Post-processing (normalize line endings, trim whitespace, ensure trailing newline)
//! - Metadata injection (timestamp, version, provider, cost, token counts)
//! - LLM-based deconfliction with existing rule files in the project

use crate::cli::config::FinalizationConfig;
use crate::generator::rules::{FormattedRules, GeneratedRules};
use crate::llm::client::LLMClient;
use crate::llm::cost::{CostCalculator, CostTracker};
use crate::llm::provider::{CompletionOptions, Message};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Tracks optimization metrics from the finalization pass.
#[derive(Debug, Clone, Default)]
pub struct OptimizationResult {
    /// Number of duplicate rules removed
    pub duplicates_removed: usize,
    /// Number of rules merged
    pub rules_merged: usize,
    /// Whether formatting was normalized
    pub formatting_normalized: bool,
}

/// Tracks consistency metrics from the finalization pass.
#[derive(Debug, Clone, Default)]
pub struct ConsistencyResult {
    /// Number of conventions aligned across formats
    pub conventions_aligned: usize,
    /// Number of missing conventions added
    pub missing_conventions_added: usize,
    /// Formats that were updated
    pub formats_updated: Vec<String>,
}

/// Result of the finalization stage.
#[derive(Debug, Clone, Default)]
pub struct FinalizationResult {
    /// Optimization metrics
    pub optimizations: OptimizationResult,
    /// Consistency metrics
    pub consistency: ConsistencyResult,
    /// Whether metadata was injected
    pub metadata_injected: bool,
    /// Whether deconfliction was performed
    pub deconflicted: bool,
}

/// Known rule file names for simple file-exists checks.
const KNOWN_RULE_FILE_NAMES: &[&str] = &[
    "CLAUDE.md",
    ".windsurfrules",
    "CONVENTIONS.md",
    "AI_RULES.md",
    ".cursorrules",
    ".aider.conf.yml",
];

/// Finalize generated rules with post-processing, metadata, and optional deconfliction.
///
/// # Arguments
///
/// * `rules` - The generated rules to finalize
/// * `config` - Finalization configuration
/// * `client` - LLM client for deconfliction (used only if deconfliction is enabled)
/// * `cost_tracker` - Cost tracker for recording deconfliction costs
/// * `project_path` - Path to the project root
/// * `formats` - The formats being generated
/// * `no_confirm` - Whether to skip cost confirmation prompts
/// * `quiet` - Whether to suppress output
///
/// # Returns
///
/// A tuple of the finalized rules and the finalization result.
#[allow(clippy::too_many_arguments)]
pub async fn finalize_rules(
    rules: GeneratedRules,
    config: &FinalizationConfig,
    client: &LLMClient,
    cost_tracker: &mut Option<CostTracker>,
    project_path: &Path,
    formats: &[String],
    no_confirm: bool,
    quiet: bool,
) -> Result<(GeneratedRules, FinalizationResult)> {
    let mut finalized = rules;
    let mut result = FinalizationResult::default();

    if !config.enabled {
        tracing::info!("Finalization disabled, skipping");
        return Ok((finalized, result));
    }

    // Step 1: Normalize formatting
    if config.normalize_formatting {
        normalize_formatting(&mut finalized);
        result.optimizations.formatting_normalized = true;
        tracing::debug!("Formatting normalized");
    }

    // Step 2: Inject metadata
    if config.inject_metadata {
        inject_metadata(&mut finalized);
        result.metadata_injected = true;
        tracing::debug!("Metadata injected");
    }

    // Step 3: Deconfliction with existing rules
    if config.deconflict {
        let existing_rules = detect_existing_rules(project_path, formats);
        if !existing_rules.is_empty() {
            tracing::info!(
                "Detected {} existing rule file(s) for deconfliction",
                existing_rules.len()
            );

            // Build cost calculator from client pricing for accurate cost confirmation
            let calculator = CostCalculator::new(client.pricing());

            let deconfliction_performed = deconflict_rules(
                &mut finalized,
                &existing_rules,
                client,
                cost_tracker,
                &calculator,
                no_confirm,
                quiet,
            )
            .await?;

            result.deconflicted = deconfliction_performed;
        } else {
            tracing::debug!("No existing rule files detected, skipping deconfliction");
        }
    }

    Ok((finalized, result))
}

/// Normalize formatting in all generated rules.
///
/// - Normalizes line endings to LF
/// - Trims trailing whitespace from each line
/// - Ensures single trailing newline
fn normalize_formatting(rules: &mut GeneratedRules) {
    let formats: Vec<String> = rules.rules_by_format.keys().cloned().collect();
    for format in formats {
        if let Some(formatted) = rules.rules_by_format.get(&format) {
            let normalized = normalize_content(&formatted.content);
            let updated = FormattedRules {
                format: formatted.format.clone(),
                content: normalized,
                rule_type: formatted.rule_type,
            };
            rules.rules_by_format.insert(format, updated);
        }
    }
}

/// Normalize content: LF line endings, trim trailing whitespace, single trailing newline.
fn normalize_content(content: &str) -> String {
    // Normalize line endings to LF
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");

    // Trim trailing whitespace from each line
    let lines: Vec<&str> = normalized.lines().map(|line| line.trim_end()).collect();
    let mut result = lines.join("\n");

    // Ensure single trailing newline
    if !result.ends_with('\n') {
        result.push('\n');
    }

    result
}

/// Inject metadata header comments into generated rules.
fn inject_metadata(rules: &mut GeneratedRules) {
    let version = env!("CARGO_PKG_VERSION");
    let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let provider = rules.metadata.provider.clone();
    let model = rules.metadata.model.clone();
    let input_tokens = rules.metadata.input_tokens;
    let output_tokens = rules.metadata.output_tokens;
    let cost = rules.metadata.cost;

    let formats: Vec<String> = rules.rules_by_format.keys().cloned().collect();
    for format_name in formats {
        if let Some(formatted) = rules.rules_by_format.get(&format_name) {
            // JSON doesn't support comments, skip metadata injection
            if format_name == "json" {
                continue;
            }

            let metadata_comment = format!(
                "<!-- Generated by ruley v{} | {} | {}/{} | tokens: {}/{} | cost: ${:.4} -->",
                version, timestamp, provider, model, input_tokens, output_tokens, cost
            );

            let content_with_metadata = format!("{}\n{}", metadata_comment, formatted.content);

            let updated = FormattedRules {
                format: formatted.format.clone(),
                content: content_with_metadata,
                rule_type: formatted.rule_type,
            };
            rules.rules_by_format.insert(format_name, updated);
        }
    }
}

/// Detect existing rule files in the project that won't be overwritten.
fn detect_existing_rules(
    project_path: &Path,
    formats_being_generated: &[String],
) -> HashMap<String, String> {
    let mut existing = HashMap::new();

    for filename in KNOWN_RULE_FILE_NAMES {
        let path = project_path.join(filename);
        if path.exists() {
            // Determine what format this file belongs to
            let format = match *filename {
                "CLAUDE.md" => "claude",
                ".windsurfrules" => "windsurf",
                "CONVENTIONS.md" | ".aider.conf.yml" => "aider",
                "AI_RULES.md" => "generic",
                ".cursorrules" => "cursor",
                _ => continue,
            };

            // Only include if we're NOT generating this format (existing files we keep)
            if !formats_being_generated
                .iter()
                .any(|f| f.to_lowercase() == format)
            {
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        existing.insert(filename.to_string(), content);
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to read existing rule file '{}': {e}",
                            path.display()
                        );
                    }
                }
            }
        }
    }

    // Check .github/copilot-instructions.md
    let copilot_path = project_path.join(".github/copilot-instructions.md");
    if copilot_path.exists()
        && !formats_being_generated
            .iter()
            .any(|f| f.to_lowercase() == "copilot")
    {
        match std::fs::read_to_string(&copilot_path) {
            Ok(content) => {
                existing.insert(".github/copilot-instructions.md".to_string(), content);
            }
            Err(e) => {
                tracing::warn!("Failed to read existing copilot instructions: {e}");
            }
        }
    }

    // Check .cursor/rules/ directory for .mdc files
    let cursor_rules_dir = project_path.join(".cursor/rules");
    if cursor_rules_dir.exists()
        && !formats_being_generated
            .iter()
            .any(|f| f.to_lowercase() == "cursor")
    {
        if let Ok(entries) = std::fs::read_dir(&cursor_rules_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "mdc") {
                    match std::fs::read_to_string(&path) {
                        Ok(content) => {
                            let relative = path
                                .strip_prefix(project_path)
                                .unwrap_or(&path)
                                .to_string_lossy()
                                .to_string();
                            existing.insert(relative, content);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to read cursor rule file '{}': {e}",
                                path.display()
                            );
                        }
                    }
                }
            }
        }
    }

    existing
}

/// Perform LLM-based deconfliction with existing rule files.
async fn deconflict_rules(
    rules: &mut GeneratedRules,
    existing_rules: &HashMap<String, String>,
    client: &LLMClient,
    cost_tracker: &mut Option<CostTracker>,
    calculator: &CostCalculator,
    no_confirm: bool,
    quiet: bool,
) -> Result<bool> {
    if existing_rules.is_empty() {
        return Ok(false);
    }

    // Build existing rules summary
    let existing_summary: String = existing_rules
        .iter()
        .map(|(path, content)| format!("=== {} ===\n{}\n", path, content))
        .collect::<Vec<_>>()
        .join("\n");

    // Estimate total tokens using both existing and generated content
    let estimated_input_tokens = estimate_deconfliction_tokens(existing_rules, rules);
    // Estimate output: similar size to generated rules (deconflicted output)
    let estimated_output_tokens: usize = rules
        .rules_by_format
        .values()
        .map(|r| r.content.len() / 4)
        .sum();
    let cost_estimate = calculator.estimate_cost(estimated_input_tokens, estimated_output_tokens);

    // Show cost confirmation unless --no-confirm
    if !no_confirm && !quiet {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        println!();
        println!("Deconfliction Cost Estimation");
        println!("=============================");
        println!();
        println!(
            "Found {} existing rule file(s) that will remain in the repository.",
            existing_rules.len()
        );
        println!("Deconfliction will merge generated rules to avoid conflicts.");
        println!();
        println!(
            "Input tokens:  {:>10} (${:.4})",
            cost_estimate.input_tokens, cost_estimate.input_cost
        );
        println!(
            "Output tokens: {:>10} (${:.4}) [estimated]",
            cost_estimate.output_tokens, cost_estimate.output_cost
        );
        println!(
            "Total tokens:  {:>10}",
            cost_estimate.input_tokens + cost_estimate.output_tokens
        );
        println!("----------------------------");
        println!("Estimated cost: ${:.4}", cost_estimate.total_cost);
        println!();

        let mut stdout = tokio::io::stdout();
        stdout
            .write_all(b"Proceed with deconfliction? [y/n/s(kip)] ")
            .await
            .context("Failed to write to stdout")?;
        stdout.flush().await.context("Failed to flush stdout")?;

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut input = String::new();
        reader
            .read_line(&mut input)
            .await
            .context("Failed to read user input")?;

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => {} // proceed
            "s" | "skip" => {
                tracing::info!("User chose to skip deconfliction");
                return Ok(false);
            }
            _ => {
                return Err(anyhow::anyhow!("User cancelled deconfliction"));
            }
        }
    }

    // Deconflict each format
    let formats: Vec<String> = rules.rules_by_format.keys().cloned().collect();
    for format in formats {
        if let Some(formatted) = rules.rules_by_format.get(&format) {
            let prompt = build_deconfliction_prompt(&existing_summary, &formatted.content);

            let messages = vec![Message {
                role: "user".to_string(),
                content: prompt,
            }];

            let response = client
                .complete(&messages, &CompletionOptions::default())
                .await
                .with_context(|| format!("Failed to deconflict {} format rules", format))?;

            // Track cost
            if let Some(tracker) = cost_tracker {
                tracker.add_operation(
                    format!("deconfliction_{}", format),
                    response.prompt_tokens,
                    response.completion_tokens,
                );
            }

            // Update rules with deconflicted content
            let updated = FormattedRules {
                format: formatted.format.clone(),
                content: response.content,
                rule_type: formatted.rule_type,
            };
            rules.rules_by_format.insert(format, updated);
        }
    }

    tracing::info!("Deconfliction completed");
    Ok(true)
}

/// Build the prompt for LLM-based deconfliction.
fn build_deconfliction_prompt(existing_rules: &str, generated_rules: &str) -> String {
    format!(
        r#"You are helping to merge AI IDE rules. Existing rules will remain in the repository. Your task is to modify the generated rules to avoid duplication or conflict.

<existing_rules>
{}
</existing_rules>

<generated_rules>
{}
</generated_rules>

Task: Modify the generated rules to:
- Remove rules that duplicate existing conventions
- Rephrase rules that conflict with existing rules
- Preserve all unique insights from generated rules
- Maintain the same format and structure

Return only the modified generated rules, nothing else."#,
        existing_rules, generated_rules
    )
}

/// Estimate the cost of deconfliction based on existing and generated content.
pub fn estimate_deconfliction_tokens(
    existing_rules: &HashMap<String, String>,
    generated_rules: &GeneratedRules,
) -> usize {
    let existing_chars: usize = existing_rules.values().map(|c| c.len()).sum();
    let generated_chars: usize = generated_rules
        .rules_by_format
        .values()
        .map(|r| r.content.len())
        .sum();

    // Rough token estimate: ~4 chars per token
    (existing_chars + generated_chars) / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_content_crlf() {
        let content = "line1\r\nline2\r\nline3";
        let normalized = normalize_content(content);
        assert_eq!(normalized, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_normalize_content_trailing_whitespace() {
        let content = "line1   \nline2\t\nline3  ";
        let normalized = normalize_content(content);
        assert_eq!(normalized, "line1\nline2\nline3\n");
    }

    #[test]
    fn test_normalize_content_trailing_newline() {
        let content = "line1\nline2";
        let normalized = normalize_content(content);
        assert!(normalized.ends_with('\n'));

        // Already has trailing newline
        let content2 = "line1\nline2\n";
        let normalized2 = normalize_content(content2);
        assert_eq!(normalized2, "line1\nline2\n");
    }

    #[test]
    fn test_inject_metadata_markdown() {
        let mut rules = GeneratedRules::new("analysis");
        rules.metadata =
            crate::generator::rules::GenerationMetadata::new("anthropic", "claude-3-opus")
                .with_usage(1000, 500, 0.05);
        rules.add_format(FormattedRules::new("cursor", "# Cursor Rules"));

        inject_metadata(&mut rules);

        let cursor = rules.get_format("cursor").unwrap();
        assert!(cursor.content.contains("<!-- Generated by ruley"));
        assert!(cursor.content.contains("anthropic/claude-3-opus"));
        assert!(cursor.content.contains("1000/500"));
    }

    #[test]
    fn test_inject_metadata_skips_json() {
        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("json", r#"{"rules": []}"#));

        inject_metadata(&mut rules);

        let json = rules.get_format("json").unwrap();
        // JSON content should not have metadata comment
        assert!(!json.content.contains("<!--"));
    }

    #[test]
    fn test_build_deconfliction_prompt() {
        let prompt = build_deconfliction_prompt("existing content", "generated content");
        assert!(prompt.contains("existing content"));
        assert!(prompt.contains("generated content"));
        assert!(prompt.contains("Remove rules that duplicate"));
    }

    #[test]
    fn test_estimate_deconfliction_tokens() {
        let mut existing = HashMap::new();
        existing.insert("file1".to_string(), "a".repeat(400));
        existing.insert("file2".to_string(), "b".repeat(400));

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("cursor", "c".repeat(400)));

        let tokens = estimate_deconfliction_tokens(&existing, &rules);
        // (800 + 400) / 4 = 300
        assert_eq!(tokens, 300);
    }
}
