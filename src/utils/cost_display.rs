//! Cost estimation display and user confirmation prompts.
//!
//! This module provides functions for displaying detailed cost breakdowns
//! with tree formatting and prompting users for confirmation before
//! proceeding with LLM operations.
//!
//! # Example
//!
//! ```ignore
//! use ruley::utils::cost_display::{display_cost_estimate, prompt_confirmation};
//! use ruley::llm::provider::Pricing;
//!
//! let pricing = Pricing {
//!     input_per_1k: 0.003,
//!     output_per_1k: 0.015,
//! };
//!
//! display_cost_estimate(
//!     &codebase,
//!     &chunks,
//!     &["cursor".to_string(), "claude".to_string()],
//!     "anthropic",
//!     &pricing,
//!     false, // quiet
//! )?;
//!
//! let confirmed = prompt_confirmation("Continue?", true).await?;
//! ```

use crate::llm::chunker::Chunk;
use crate::llm::cost::CostCalculator;
use crate::llm::provider::Pricing;
use crate::packer::{CompressedCodebase, Language};
use crate::utils::formatting::format_number;
use anyhow::Result;
use console::{Term, style};
use std::collections::HashMap;
use std::io::Write;

/// Estimated output tokens per chunk analysis.
const ESTIMATED_OUTPUT_TOKENS_PER_CHUNK: usize = 4096;

/// Estimated tokens per format refinement call.
const ESTIMATED_TOKENS_PER_FORMAT: usize = 500;

/// Estimated tokens for merge call in multi-chunk scenarios.
const ESTIMATED_MERGE_TOKENS: usize = 10_000;

/// Display a detailed cost estimate with tree formatting.
///
/// Shows file breakdown by language, token counts, compression ratio,
/// chunk information, and per-operation cost breakdown.
///
/// # Arguments
///
/// * `codebase` - The compressed codebase with metadata
/// * `chunks` - The chunks prepared for analysis
/// * `formats` - The output formats to generate
/// * `provider` - The LLM provider name (for display)
/// * `pricing` - The pricing information for cost calculation
/// * `quiet` - If true, suppresses output entirely
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
pub fn display_cost_estimate(
    codebase: &CompressedCodebase,
    chunks: &[Chunk],
    formats: &[String],
    provider: &str,
    pricing: &Pricing,
    quiet: bool,
) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let mut term = Term::stdout();
    let calculator = CostCalculator::new(pricing.clone());

    // Calculate token counts
    let total_compressed_tokens: usize = chunks.iter().map(|c| c.token_count).sum();
    let is_multi_chunk = chunks.len() > 1;

    // Calculate costs
    let breakdown = calculate_cost_breakdown(chunks, formats, &calculator);

    // Format language breakdown
    let language_breakdown = format_language_breakdown(&codebase.metadata.languages);

    // Calculate compression ratio display
    let compression_percent = if codebase.metadata.compression_ratio < 1.0 {
        ((1.0 - codebase.metadata.compression_ratio) * 100.0) as u32
    } else {
        0
    };

    // Format the provider and model display
    let provider_display = format_provider_display(provider);

    // Print the tree-formatted output
    writeln!(term)?;
    writeln!(term, "{}", style("Analysis Summary:").bold())?;

    // Files line
    writeln!(
        term,
        "{} Files: {} files ({})",
        style("\u{251c}\u{2500}").dim(),
        codebase.metadata.total_files,
        language_breakdown
    )?;

    // Tokens line
    if codebase.metadata.compression_ratio < 1.0 {
        // When compression_ratio == 0.0, treat total_compressed_tokens as already-uncompressed
        // to avoid division by zero or infinity. Otherwise derive original_tokens from
        // total_compressed_tokens / compression_ratio.
        let original_tokens = if codebase.metadata.compression_ratio > 0.0 {
            (total_compressed_tokens as f32 / codebase.metadata.compression_ratio) as usize
        } else {
            total_compressed_tokens
        };
        writeln!(
            term,
            "{} Tokens: {} (before compression: {})",
            style("\u{251c}\u{2500}").dim(),
            format_number(total_compressed_tokens),
            format_number(original_tokens)
        )?;
    } else {
        writeln!(
            term,
            "{} Tokens: {}",
            style("\u{251c}\u{2500}").dim(),
            format_number(total_compressed_tokens)
        )?;
    }

    // Compression line (only if compression was applied)
    if codebase.metadata.compression_ratio < 1.0 && compression_percent > 0 {
        writeln!(
            term,
            "{} Compression: {}% reduction",
            style("\u{251c}\u{2500}").dim(),
            compression_percent
        )?;
    }

    // Chunks line
    if is_multi_chunk {
        writeln!(
            term,
            "{} Chunks: {} (exceeds context limit)",
            style("\u{251c}\u{2500}").dim(),
            chunks.len()
        )?;
    } else {
        writeln!(
            term,
            "{} Chunks: 1 (within context limit)",
            style("\u{251c}\u{2500}").dim()
        )?;
    }

    // Formats line
    writeln!(
        term,
        "{} Formats: {}",
        style("\u{251c}\u{2500}").dim(),
        formats.join(", ")
    )?;

    // Estimated cost line (last item in summary)
    writeln!(
        term,
        "{} Estimated cost: {} ({})",
        style("\u{2514}\u{2500}").dim(),
        style(format!("${:.2}", breakdown.total_cost))
            .green()
            .bold(),
        provider_display
    )?;

    // Print breakdown
    writeln!(term)?;
    writeln!(term, "{}:", style("Breakdown").bold())?;

    if is_multi_chunk {
        // Multi-chunk breakdown
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_cost =
                calculator.calculate_cost(chunk.token_count, ESTIMATED_OUTPUT_TOKENS_PER_CHUNK);
            writeln!(
                term,
                "{} Chunk {} analysis: ${:.2} ({} tokens)",
                style("\u{251c}\u{2500}").dim(),
                i + 1,
                chunk_cost,
                format_number(chunk.token_count)
            )?;
        }

        // Merge call
        let merge_cost =
            calculator.calculate_cost(ESTIMATED_MERGE_TOKENS, ESTIMATED_OUTPUT_TOKENS_PER_CHUNK);
        writeln!(
            term,
            "{} Merge call: ${:.2} (~{} tokens)",
            style("\u{251c}\u{2500}").dim(),
            merge_cost,
            format_number(ESTIMATED_MERGE_TOKENS)
        )?;

        // Format refinements (last item)
        writeln!(
            term,
            "{} Format refinements: ${:.2} ({} formats x ~{} tokens each)",
            style("\u{2514}\u{2500}").dim(),
            breakdown.format_refinement_cost,
            formats.len(),
            ESTIMATED_TOKENS_PER_FORMAT
        )?;
    } else {
        // Single chunk breakdown
        writeln!(
            term,
            "{} Initial analysis: ${:.2} ({} tokens)",
            style("\u{251c}\u{2500}").dim(),
            breakdown.analysis_cost,
            format_number(total_compressed_tokens)
        )?;

        // Format refinements (last item)
        writeln!(
            term,
            "{} Format refinements: ${:.2} ({} formats x ~{} tokens each)",
            style("\u{2514}\u{2500}").dim(),
            breakdown.format_refinement_cost,
            formats.len(),
            ESTIMATED_TOKENS_PER_FORMAT
        )?;
    }

    // Note for large codebases
    if is_multi_chunk {
        writeln!(term)?;
        writeln!(
            term,
            "{}: Large codebase requires chunking. Use --include patterns to reduce scope.",
            style("Note").yellow().bold()
        )?;
    }

    writeln!(term)?;

    Ok(())
}

/// Prompt the user for confirmation with a default value.
///
/// Displays a prompt and waits for user input. Respects terminal capabilities
/// and falls back gracefully in non-TTY environments.
///
/// # Arguments
///
/// * `message` - The prompt message to display
/// * `default_yes` - If true, defaults to "Y" (pressing Enter confirms)
///
/// # Returns
///
/// `true` if the user confirmed, `false` otherwise.
///
/// # Errors
///
/// Returns an error if reading from stdin fails.
pub async fn prompt_confirmation(message: &str, default_yes: bool) -> Result<bool> {
    // Use spawn_blocking since dialoguer is synchronous
    let message = message.to_string();
    let result =
        tokio::task::spawn_blocking(move || prompt_confirmation_sync(&message, default_yes))
            .await??;

    Ok(result)
}

/// Synchronous confirmation prompt implementation.
fn prompt_confirmation_sync(message: &str, default_yes: bool) -> Result<bool> {
    let term = Term::stdout();

    // Check if we're in a TTY
    if !term.is_term() {
        // In non-TTY mode, use default
        return Ok(default_yes);
    }

    // Build the prompt suffix based on default
    let suffix = if default_yes { "[Y/n]" } else { "[y/N]" };
    let prompt = format!("{} {}", message, suffix);

    // Use dialoguer for confirmation
    // Esc/q cancellation returns None; treat explicit cancel as "no" (not default)
    let confirmed = dialoguer::Confirm::new()
        .with_prompt(&prompt)
        .default(default_yes)
        .show_default(false) // We show it in the prompt text
        .interact_opt()?
        .unwrap_or(false);

    Ok(confirmed)
}

/// Cost breakdown for display purposes.
struct CostBreakdownDisplay {
    /// Total estimated cost
    total_cost: f64,
    /// Cost for initial analysis (single chunk) or all chunk analyses (multi-chunk)
    analysis_cost: f64,
    /// Cost for merge call (multi-chunk only)
    #[allow(dead_code)]
    merge_cost: f64,
    /// Cost for format refinement calls
    format_refinement_cost: f64,
}

/// Calculate the cost breakdown for display.
fn calculate_cost_breakdown(
    chunks: &[Chunk],
    formats: &[String],
    calculator: &CostCalculator,
) -> CostBreakdownDisplay {
    let is_multi_chunk = chunks.len() > 1;

    // Calculate analysis cost
    let analysis_cost: f64 = chunks
        .iter()
        .map(|c| calculator.calculate_cost(c.token_count, ESTIMATED_OUTPUT_TOKENS_PER_CHUNK))
        .sum();

    // Calculate merge cost (only for multi-chunk)
    let merge_cost = if is_multi_chunk {
        calculator.calculate_cost(ESTIMATED_MERGE_TOKENS, ESTIMATED_OUTPUT_TOKENS_PER_CHUNK)
    } else {
        0.0
    };

    // Calculate format refinement cost
    let format_refinement_cost = formats.len() as f64
        * calculator.calculate_cost(ESTIMATED_TOKENS_PER_FORMAT, ESTIMATED_TOKENS_PER_FORMAT);

    let total_cost = analysis_cost + merge_cost + format_refinement_cost;

    CostBreakdownDisplay {
        total_cost,
        analysis_cost,
        merge_cost,
        format_refinement_cost,
    }
}

/// Format the language breakdown for display.
fn format_language_breakdown(languages: &HashMap<Language, usize>) -> String {
    if languages.is_empty() {
        return "various".to_string();
    }

    // Sort languages by count (descending)
    let mut sorted: Vec<_> = languages.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));

    // Take top 3 languages and count the rest as "other"
    let (top_langs, rest): (Vec<_>, Vec<_>) =
        sorted.into_iter().enumerate().partition(|(i, _)| *i < 3);

    let mut parts: Vec<String> = top_langs
        .into_iter()
        .map(|(_, (lang, count))| format!("{} {}", count, lang))
        .collect();

    let other_count: usize = rest.into_iter().map(|(_, (_, count))| count).sum();
    if other_count > 0 {
        parts.push(format!("{} other", other_count));
    }

    parts.join(", ")
}

/// Format the provider display string.
fn format_provider_display(provider: &str) -> String {
    match provider.to_lowercase().as_str() {
        "anthropic" => "Anthropic Claude Sonnet".to_string(),
        "openai" => "OpenAI GPT-4o".to_string(),
        "ollama" => "Ollama (local)".to_string(),
        "openrouter" => "OpenRouter".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packer::{CodebaseMetadata, CompressedFile, CompressionMethod};
    use std::path::PathBuf;

    fn create_test_pricing() -> Pricing {
        Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        }
    }

    fn create_test_codebase() -> CompressedCodebase {
        let mut languages = HashMap::new();
        languages.insert(Language::TypeScript, 45);
        languages.insert(Language::Python, 32);
        languages.insert(Language::Rust, 20);
        languages.insert(Language::Go, 15);
        languages.insert(Language::JavaScript, 15);

        CompressedCodebase {
            files: vec![CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() {}".to_string(),
                compressed_content: "fn main() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 12,
                compressed_size: 12,
                language: Some(Language::Rust),
            }],
            metadata: CodebaseMetadata {
                total_files: 127,
                total_original_size: 156891,
                total_compressed_size: 48234,
                languages,
                compression_ratio: 0.31,
            },
        }
    }

    fn create_test_chunks(count: usize, tokens_per_chunk: usize) -> Vec<Chunk> {
        (0..count)
            .map(|i| Chunk {
                id: i,
                content: "test content".to_string(),
                token_count: tokens_per_chunk,
                overlap_token_count: if i > 0 { 1000 } else { 0 },
            })
            .collect()
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1234567), "1,234,567");
        assert_eq!(format_number(48234), "48,234");
    }

    #[test]
    fn test_format_language_breakdown() {
        let mut languages = HashMap::new();
        languages.insert(Language::TypeScript, 45);
        languages.insert(Language::Python, 32);
        languages.insert(Language::Rust, 20);
        languages.insert(Language::Go, 15);
        languages.insert(Language::JavaScript, 15);

        let result = format_language_breakdown(&languages);

        // Should contain top 3 languages (lowercase as per Language::Display) and "other"
        assert!(result.contains("typescript"));
        assert!(result.contains("python"));
        assert!(result.contains("rust"));
        assert!(result.contains("other"));
    }

    #[test]
    fn test_format_language_breakdown_empty() {
        let languages = HashMap::new();
        let result = format_language_breakdown(&languages);
        assert_eq!(result, "various");
    }

    #[test]
    fn test_format_provider_display() {
        assert_eq!(
            format_provider_display("anthropic"),
            "Anthropic Claude Sonnet"
        );
        assert_eq!(format_provider_display("openai"), "OpenAI GPT-4o");
        assert_eq!(format_provider_display("ollama"), "Ollama (local)");
        assert_eq!(format_provider_display("unknown"), "unknown");
    }

    #[test]
    fn test_calculate_cost_breakdown_single_chunk() {
        let pricing = create_test_pricing();
        let calculator = CostCalculator::new(pricing);
        let chunks = create_test_chunks(1, 50000);
        let formats = vec!["cursor".to_string(), "claude".to_string()];

        let breakdown = calculate_cost_breakdown(&chunks, &formats, &calculator);

        assert!(breakdown.total_cost > 0.0);
        assert!(breakdown.analysis_cost > 0.0);
        assert_eq!(breakdown.merge_cost, 0.0); // No merge for single chunk
        assert!(breakdown.format_refinement_cost > 0.0);
    }

    #[test]
    fn test_calculate_cost_breakdown_multi_chunk() {
        let pricing = create_test_pricing();
        let calculator = CostCalculator::new(pricing);
        let chunks = create_test_chunks(3, 78189);
        let formats = vec![
            "cursor".to_string(),
            "claude".to_string(),
            "copilot".to_string(),
        ];

        let breakdown = calculate_cost_breakdown(&chunks, &formats, &calculator);

        assert!(breakdown.total_cost > 0.0);
        assert!(breakdown.analysis_cost > 0.0);
        assert!(breakdown.merge_cost > 0.0); // Merge for multi-chunk
        assert!(breakdown.format_refinement_cost > 0.0);

        // Total should equal sum of parts
        let expected_total =
            breakdown.analysis_cost + breakdown.merge_cost + breakdown.format_refinement_cost;
        assert!((breakdown.total_cost - expected_total).abs() < 0.0001);
    }

    #[test]
    fn test_display_cost_estimate_quiet_mode() {
        let codebase = create_test_codebase();
        let chunks = create_test_chunks(1, 48234);
        let formats = vec!["cursor".to_string()];
        let pricing = create_test_pricing();

        // Should not error in quiet mode
        let result = display_cost_estimate(
            &codebase,
            &chunks,
            &formats,
            "anthropic",
            &pricing,
            true, // quiet
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_display_cost_estimate_single_chunk() {
        let codebase = create_test_codebase();
        let chunks = create_test_chunks(1, 48234);
        let formats = vec![
            "cursor".to_string(),
            "claude".to_string(),
            "copilot".to_string(),
        ];
        let pricing = create_test_pricing();

        // Should not error
        let result =
            display_cost_estimate(&codebase, &chunks, &formats, "anthropic", &pricing, false);

        assert!(result.is_ok());
    }

    #[test]
    fn test_display_cost_estimate_multi_chunk() {
        let codebase = create_test_codebase();
        let chunks = create_test_chunks(3, 78189);
        let formats = vec![
            "cursor".to_string(),
            "claude".to_string(),
            "copilot".to_string(),
        ];
        let pricing = create_test_pricing();

        // Should not error
        let result =
            display_cost_estimate(&codebase, &chunks, &formats, "anthropic", &pricing, false);

        assert!(result.is_ok());
    }
}
