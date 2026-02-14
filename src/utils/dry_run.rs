//! Dry-run mode display for previewing pipeline operations.
//!
//! This module provides functions for displaying what would happen during
//! a rule generation run without actually making LLM calls.
//!
//! # Example
//!
//! ```ignore
//! use ruley::utils::dry_run::display_dry_run_summary;
//!
//! display_dry_run_summary(
//!     &compressed_codebase,
//!     &["cursor", "claude", "copilot"],
//!     &merged_config,
//!     &pricing,
//! )?;
//! ```

use crate::MergedConfig;
use crate::llm::provider::Pricing;
use crate::output::get_formatter;
use crate::packer::{CompressedCodebase, Language};
use crate::utils::formatting::format_number;
use anyhow::Result;
use console::{Term, style};
use std::collections::HashMap;
use std::io::Write;

/// Maximum number of files to show per language before truncating.
const MAX_FILES_PER_LANGUAGE: usize = 3;

/// Display a comprehensive dry-run summary.
///
/// Shows file breakdown by language with token counts, compression statistics,
/// estimated cost, and output locations.
///
/// # Arguments
///
/// * `codebase` - The compressed codebase with file metadata
/// * `formats` - Output formats that would be generated
/// * `config` - The merged configuration
/// * `pricing` - Pricing information for cost estimation
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
pub fn display_dry_run_summary(
    codebase: &CompressedCodebase,
    formats: &[String],
    config: &MergedConfig,
    pricing: &Pricing,
) -> Result<()> {
    let mut term = Term::stdout();

    // Header
    writeln!(term)?;
    writeln!(
        term,
        "{} - No LLM calls will be made",
        style("Dry Run").yellow().bold()
    )?;

    // Files to be analyzed section
    writeln!(term)?;
    writeln!(term, "{}:", style("Files to be analyzed").bold())?;

    // Group files by language
    let files_by_language = group_files_by_language(codebase);

    // Calculate token counts per language
    let tokens_by_language = calculate_tokens_by_language(codebase);

    // Sort languages by file count (descending)
    let mut languages: Vec<_> = files_by_language.keys().collect();
    languages.sort_by(|a, b| {
        files_by_language
            .get(*b)
            .map(|v: &Vec<(String, usize)>| v.len())
            .unwrap_or(0)
            .cmp(
                &files_by_language
                    .get(*a)
                    .map(|v: &Vec<(String, usize)>| v.len())
                    .unwrap_or(0),
            )
    });

    for (lang_idx, language) in languages.iter().enumerate() {
        let files = files_by_language
            .get(*language)
            .expect("files_by_language missing entry for language");
        let token_count = tokens_by_language.get(*language).copied().unwrap_or(0);
        let is_last_language = lang_idx == languages.len() - 1;

        let lang_prefix = if is_last_language {
            "\u{2514}\u{2500}"
        } else {
            "\u{251c}\u{2500}"
        };
        let child_prefix = if is_last_language {
            "   "
        } else {
            "\u{2502}  "
        };

        // Language header line
        writeln!(
            term,
            "{} {} ({} files, {} tokens)",
            style(lang_prefix).dim(),
            language,
            files.len(),
            format_number(token_count)
        )?;

        // Show first few files
        let files_to_show = files.len().min(MAX_FILES_PER_LANGUAGE);
        for (file_idx, (path, tokens)) in files.iter().take(files_to_show).enumerate() {
            let is_last_file =
                file_idx == files_to_show - 1 && files.len() <= MAX_FILES_PER_LANGUAGE;
            let file_prefix = if is_last_file {
                "\u{2514}\u{2500}"
            } else {
                "\u{251c}\u{2500}"
            };

            writeln!(
                term,
                "{}{} {} ({} tokens)",
                child_prefix,
                style(file_prefix).dim(),
                path,
                format_number(*tokens)
            )?;
        }

        // Show "... (N more files)" if there are more
        if files.len() > MAX_FILES_PER_LANGUAGE {
            let remaining = files.len() - MAX_FILES_PER_LANGUAGE;
            writeln!(
                term,
                "{}\u{2514}\u{2500} ... ({} more files)",
                child_prefix, remaining
            )?;
        }
    }

    // Total line
    let total_compressed_tokens: usize = codebase
        .files
        .iter()
        .map(|f| estimate_tokens(&f.compressed_content))
        .sum();

    writeln!(term)?;
    writeln!(
        term,
        "{}: {} files, {} tokens",
        style("Total").bold(),
        format_number(codebase.metadata.total_files),
        format_number(total_compressed_tokens)
    )?;

    // Compression statistics (outer if already guarantees compression_ratio > 0.0)
    if codebase.metadata.compression_ratio < 1.0 && codebase.metadata.compression_ratio > 0.0 {
        let original_tokens =
            (total_compressed_tokens as f32 / codebase.metadata.compression_ratio) as usize;
        let percent = ((1.0 - codebase.metadata.compression_ratio) * 100.0) as u32;

        writeln!(
            term,
            "{}: {}% reduction (from {} tokens)",
            style("Compression").bold(),
            percent,
            format_number(original_tokens)
        )?;
    }

    // Estimated cost
    let estimated_cost = estimate_cost(total_compressed_tokens, formats.len(), pricing);
    writeln!(
        term,
        "{}: {}",
        style("Estimated cost").bold(),
        style(format!("${:.2}", estimated_cost)).green()
    )?;

    // Output formats
    writeln!(term)?;
    writeln!(
        term,
        "{}: {}",
        style("Output formats").bold(),
        formats.join(", ")
    )?;

    // Output locations
    writeln!(term, "{}:", style("Output locations").bold())?;
    for (i, format) in formats.iter().enumerate() {
        let prefix = if i == formats.len() - 1 {
            "\u{2514}\u{2500}"
        } else {
            "\u{251c}\u{2500}"
        };

        let path = get_output_path(format, config);
        writeln!(term, "{} {}", style(prefix).dim(), path)?;
    }

    writeln!(term)?;

    Ok(())
}

/// Group files by their detected language.
fn group_files_by_language(
    codebase: &CompressedCodebase,
) -> HashMap<Language, Vec<(String, usize)>> {
    let mut by_language: HashMap<Language, Vec<(String, usize)>> = HashMap::new();

    for file in &codebase.files {
        let language = file.language.unwrap_or(Language::Unknown);
        let tokens = estimate_tokens(&file.compressed_content);
        let path = file.path.display().to_string();

        by_language
            .entry(language)
            .or_default()
            .push((path, tokens));
    }

    // Sort files within each language by token count (descending)
    for files in by_language.values_mut() {
        files.sort_by(|a: &(String, usize), b: &(String, usize)| b.1.cmp(&a.1));
    }

    by_language
}

/// Calculate total tokens per language.
fn calculate_tokens_by_language(codebase: &CompressedCodebase) -> HashMap<Language, usize> {
    let mut tokens_by_language: HashMap<Language, usize> = HashMap::new();

    for file in &codebase.files {
        let language = file.language.unwrap_or(Language::Unknown);
        let tokens = estimate_tokens(&file.compressed_content);
        *tokens_by_language.entry(language).or_default() += tokens;
    }

    tokens_by_language
}

/// Estimate tokens for a piece of content.
///
/// Uses a simple heuristic: ~4 characters per token.
fn estimate_tokens(content: &str) -> usize {
    content.len().div_ceil(4)
}

/// Estimate the cost for the dry run.
fn estimate_cost(input_tokens: usize, format_count: usize, pricing: &Pricing) -> f64 {
    // Estimate output tokens (roughly 1:1 for analysis)
    let output_tokens = 4096;
    // Format refinement tokens
    let format_tokens = format_count * 500;

    let input_cost = (input_tokens as f64 / 1000.0) * pricing.input_per_1k;
    let output_cost = ((output_tokens + format_tokens) as f64 / 1000.0) * pricing.output_per_1k;

    input_cost + output_cost
}

/// Get the output path for a format.
fn get_output_path(format: &str, config: &MergedConfig) -> String {
    // Check for custom output path in config
    if let Some(ref output) = config.output {
        return output.display().to_string();
    }

    // Get default path from formatter
    if let Ok(formatter) = get_formatter(format) {
        let dir = formatter.default_directory();
        let filename = formatter.default_filename();
        let ext = formatter.extension();

        if dir.is_empty() {
            format!("{}.{}", filename, ext)
        } else {
            format!("{}/{}.{}", dir, filename, ext)
        }
    } else {
        format!("{}.txt", format)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::config::ProvidersConfig;
    use crate::packer::{CodebaseMetadata, CompressedFile, CompressionMethod};
    use std::path::PathBuf;

    fn create_test_pricing() -> Pricing {
        Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        }
    }

    fn create_test_config() -> MergedConfig {
        MergedConfig {
            provider: "anthropic".to_string(),
            model: None,
            format: vec!["cursor".to_string(), "claude".to_string()],
            output: None,
            repomix_file: None,
            path: PathBuf::from("."),
            description: None,
            rule_type: "comprehensive".to_string(),
            include: vec![],
            exclude: vec![],
            compress: true,
            chunk_size: 100_000,
            no_confirm: false,
            dry_run: true,
            verbose: 0,
            quiet: false,
            output_paths: std::collections::HashMap::new(),
            chunking: None,
            providers: ProvidersConfig::default(),
        }
    }

    fn create_test_codebase() -> CompressedCodebase {
        let files = vec![
            CompressedFile {
                path: PathBuf::from("src/main.ts"),
                original_content: "a".repeat(1000),
                compressed_content: "a".repeat(936), // 234 tokens
                compression_method: CompressionMethod::TreeSitter,
                original_size: 1000,
                compressed_size: 936,
                language: Some(Language::TypeScript),
            },
            CompressedFile {
                path: PathBuf::from("src/lib.ts"),
                original_content: "b".repeat(800),
                compressed_content: "b".repeat(756), // 189 tokens
                compression_method: CompressionMethod::TreeSitter,
                original_size: 800,
                compressed_size: 756,
                language: Some(Language::TypeScript),
            },
            CompressedFile {
                path: PathBuf::from("src/utils.py"),
                original_content: "c".repeat(600),
                compressed_content: "c".repeat(400),
                compression_method: CompressionMethod::TreeSitter,
                original_size: 600,
                compressed_size: 400,
                language: Some(Language::Python),
            },
        ];

        let mut languages = HashMap::new();
        languages.insert(Language::TypeScript, 2);
        languages.insert(Language::Python, 1);

        CompressedCodebase {
            files,
            metadata: CodebaseMetadata {
                total_files: 3,
                total_original_size: 2400,
                total_compressed_size: 2092,
                languages,
                compression_ratio: 0.87,
            },
        }
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(12345), "12,345");
        assert_eq!(format_number(1234567), "1,234,567");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("a".repeat(100).as_str()), 25);
    }

    #[test]
    fn test_estimate_cost() {
        let pricing = create_test_pricing();
        let cost = estimate_cost(50000, 3, &pricing);

        // Should be input + output cost
        assert!(cost > 0.0);
        assert!(cost < 1.0); // Reasonable for 50k tokens
    }

    #[test]
    fn test_get_output_path_default() {
        let config = create_test_config();

        assert_eq!(
            get_output_path("cursor", &config),
            ".cursor/rules/project.mdc"
        );
        assert_eq!(get_output_path("claude", &config), "CLAUDE.md");
        assert_eq!(
            get_output_path("copilot", &config),
            ".github/copilot-instructions.md"
        );
    }

    #[test]
    fn test_get_output_path_custom() {
        let mut config = create_test_config();
        config.output = Some(PathBuf::from("custom/output.txt"));

        // Custom output path overrides default
        assert_eq!(get_output_path("cursor", &config), "custom/output.txt");
    }

    #[test]
    fn test_group_files_by_language() {
        let codebase = create_test_codebase();
        let grouped = group_files_by_language(&codebase);

        assert!(grouped.contains_key(&Language::TypeScript));
        assert!(grouped.contains_key(&Language::Python));
        assert_eq!(grouped.get(&Language::TypeScript).unwrap().len(), 2);
        assert_eq!(grouped.get(&Language::Python).unwrap().len(), 1);
    }

    #[test]
    fn test_calculate_tokens_by_language() {
        let codebase = create_test_codebase();
        let tokens = calculate_tokens_by_language(&codebase);

        assert!(tokens.contains_key(&Language::TypeScript));
        assert!(tokens.contains_key(&Language::Python));
        assert!(*tokens.get(&Language::TypeScript).unwrap() > 0);
        assert!(*tokens.get(&Language::Python).unwrap() > 0);
    }

    #[test]
    fn test_display_dry_run_summary() {
        let codebase = create_test_codebase();
        let formats = vec!["cursor".to_string(), "claude".to_string()];
        let config = create_test_config();
        let pricing = create_test_pricing();

        // Should not error
        let result = display_dry_run_summary(&codebase, &formats, &config, &pricing);
        assert!(result.is_ok());
    }
}
