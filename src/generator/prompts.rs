// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Prompt generation for LLM analysis and format refinement.
//!
//! This module provides functions to build prompts for:
//! - Initial codebase analysis
//! - Format-specific rule refinement (Cursor, Claude, Copilot)
//! - Smart merging of existing and new rules
//!
//! # Example
//!
//! ```ignore
//! use ruley::generator::prompts::{build_analysis_prompt, build_refinement_prompt};
//! use ruley::packer::CompressedCodebase;
//!
//! let prompt = build_analysis_prompt(&codebase, Some("Focus on error handling"));
//! let refined = build_refinement_prompt(&analysis, "cursor", Some("always"));
//! ```

use crate::packer::CompressedCodebase;
use regex::Regex;
use std::sync::LazyLock;

/// Load the base analysis prompt template.
pub fn base_prompt() -> &'static str {
    include_str!("../../prompts/base.md")
}

/// Load the Cursor format refinement prompt template.
pub fn cursor_prompt() -> &'static str {
    include_str!("../../prompts/cursor.md")
}

/// Load the Claude format refinement prompt template.
pub fn claude_prompt() -> &'static str {
    include_str!("../../prompts/claude.md")
}

/// Load the Copilot format refinement prompt template.
pub fn copilot_prompt() -> &'static str {
    include_str!("../../prompts/copilot.md")
}

/// Load the smart merge prompt template.
pub fn smart_merge_prompt() -> &'static str {
    include_str!("../../prompts/smart_merge.md")
}

/// Load the Windsurf format refinement prompt template.
pub fn windsurf_prompt() -> &'static str {
    include_str!("../../prompts/windsurf.md")
}

/// Load the Aider format refinement prompt template.
pub fn aider_prompt() -> &'static str {
    include_str!("../../prompts/aider.md")
}

/// Load the generic format refinement prompt template.
pub fn generic_prompt() -> &'static str {
    include_str!("../../prompts/generic.md")
}

/// Build the analysis prompt for initial codebase analysis.
///
/// This function constructs a comprehensive prompt that includes:
/// - Codebase metadata (file count, languages, compression ratio)
/// - Optional focus area from the user
/// - The compressed codebase content
///
/// # Arguments
///
/// * `codebase` - The compressed codebase to analyze
/// * `focus` - Optional focus area or description to guide the analysis
///
/// # Returns
///
/// A complete prompt string ready to send to the LLM.
///
/// # Example
///
/// ```ignore
/// let prompt = build_analysis_prompt(&codebase, Some("Focus on async patterns"));
/// ```
pub fn build_analysis_prompt(codebase: &CompressedCodebase, focus: Option<&str>) -> String {
    let template = base_prompt();

    // Build language list from metadata
    let languages = if codebase.metadata.languages.is_empty() {
        "Unknown".to_string()
    } else {
        let mut lang_list: Vec<_> = codebase
            .metadata
            .languages
            .iter()
            .map(|(lang, count)| format!("{} ({})", lang, count))
            .collect();
        lang_list.sort();
        lang_list.join(", ")
    };

    // Build focus section
    let focus_section = if let Some(focus_text) = focus {
        format!("Special Focus:\n{}\n", focus_text)
    } else {
        String::new()
    };

    // Build codebase content
    let codebase_content = format_codebase_content(codebase);

    // Substitute variables
    template
        .replace("{{file_count}}", &codebase.metadata.total_files.to_string())
        .replace("{{languages}}", &languages)
        .replace(
            "{{compression_ratio}}",
            &format!("{:.1}%", codebase.metadata.compression_ratio * 100.0),
        )
        .replace("{{focus_section}}", &focus_section)
        .replace("{{codebase_content}}", &codebase_content)
}

/// Build a refinement prompt for format-specific rule generation.
///
/// Takes the raw analysis output and converts it to a specific format
/// (Cursor .mdc, Claude CLAUDE.md, Copilot, Windsurf, Aider, or generic).
///
/// # Arguments
///
/// * `analysis` - The raw analysis from the initial LLM call
/// * `format` - The target format ("cursor", "claude", "copilot", "windsurf", "aider", "generic")
/// * `rule_type_slug` - Optional machine-readable slug (e.g., "always", "auto", "manual", "files")
///
/// # Returns
///
/// A prompt string for format-specific refinement.
pub fn build_refinement_prompt(
    analysis: &str,
    format: &str,
    rule_type_slug: Option<&str>,
) -> String {
    let template = match format.to_lowercase().as_str() {
        "cursor" => cursor_prompt(),
        "claude" => claude_prompt(),
        "copilot" => copilot_prompt(),
        "windsurf" => windsurf_prompt(),
        "aider" => aider_prompt(),
        "generic" => generic_prompt(),
        _ => generic_prompt(), // Default to generic format
    };

    let slug = rule_type_slug.unwrap_or("auto");
    let always_apply = slug == "always";

    // Compute human-friendly label from slug for display in prompts
    let rule_type_label = match slug {
        "always" => "Always Apply",
        "auto" => "Apply Intelligently",
        "files" => "Apply to Specific Files",
        "manual" => "Apply Manually",
        other => other,
    };

    // Detect primary language from analysis
    let primary_language = detect_primary_language(analysis);

    template
        .replace("{{analysis}}", analysis)
        .replace("{{rule_type}}", rule_type_label)
        .replace("{{always_apply}}", &always_apply.to_string())
        .replace("{{primary_language}}", &primary_language)
}

/// Build a smart merge prompt for incremental rule updates.
///
/// This prompt instructs the LLM to intelligently merge existing rules
/// with new analysis, preserving valid rules while updating changed ones.
///
/// # Arguments
///
/// * `existing_rules` - The current rules content from the file
/// * `new_analysis` - The new analysis from re-scanning the codebase
///
/// # Returns
///
/// A prompt string for smart merging.
pub fn build_smart_merge_prompt(existing_rules: &str, new_analysis: &str) -> String {
    let template = smart_merge_prompt();

    template
        .replace("{{existing_rules}}", existing_rules)
        .replace("{{new_analysis}}", new_analysis)
}

/// Format the compressed codebase content for inclusion in prompts.
///
/// Creates a structured representation of all files with their paths
/// and compressed content. Pre-allocates capacity to avoid reallocations
/// for large codebases.
fn format_codebase_content(codebase: &CompressedCodebase) -> String {
    // Estimate capacity: ~50 bytes overhead per file (separators, path) + content
    let estimated_size: usize = codebase
        .files
        .iter()
        .map(|f| f.compressed_content.len() + 50)
        .sum();
    let mut content = String::with_capacity(estimated_size);

    for file in &codebase.files {
        content.push_str("--- ");
        content.push_str(&file.path.to_string_lossy());
        content.push_str(" ---\n");
        content.push_str(&file.compressed_content);
        content.push_str("\n\n");
    }

    content
}

// Static compiled regexes for language detection with word boundaries
static RE_RUST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\brust\b").unwrap());
static RE_TYPESCRIPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\btypescript\b").unwrap());
static RE_JAVASCRIPT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bjavascript\b").unwrap());
static RE_PYTHON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bpython\b").unwrap());
static RE_GO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\b(go|golang)\b").unwrap());
static RE_JAVA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bjava\b").unwrap());
static RE_CPP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bc\+\+\b").unwrap());
static RE_CSHARP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\bc#\b").unwrap());

/// Detect the primary programming language from analysis text.
///
/// Uses regex word boundaries to avoid false positives (e.g., "don't use Rust"
/// won't match incorrectly). Languages are checked in order of specificity.
fn detect_primary_language(analysis: &str) -> String {
    // Check for language mentions in order of specificity using word boundaries
    if RE_RUST.is_match(analysis) {
        "rust".to_string()
    } else if RE_TYPESCRIPT.is_match(analysis) {
        "typescript".to_string()
    } else if RE_JAVASCRIPT.is_match(analysis) {
        "javascript".to_string()
    } else if RE_PYTHON.is_match(analysis) {
        "python".to_string()
    } else if RE_GO.is_match(analysis) {
        "go".to_string()
    } else if RE_JAVA.is_match(analysis) {
        "java".to_string()
    } else if RE_CPP.is_match(analysis) {
        "cpp".to_string()
    } else if RE_CSHARP.is_match(analysis) {
        "csharp".to_string()
    } else {
        "text".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packer::{CompressedFile, CompressionMethod};
    use std::path::PathBuf;

    fn create_test_codebase() -> CompressedCodebase {
        let files = vec![
            CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() { println!(\"Hello\"); }".to_string(),
                compressed_content: "fn main() { println!(\"Hello\"); }".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 34,
                compressed_size: 34,
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

        CompressedCodebase::new(files)
    }

    #[test]
    fn test_build_analysis_prompt_without_focus() {
        let codebase = create_test_codebase();
        let prompt = build_analysis_prompt(&codebase, None);

        assert!(prompt.contains("Files: 2"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(!prompt.contains("Special Focus:"));
    }

    #[test]
    fn test_build_analysis_prompt_with_focus() {
        let codebase = create_test_codebase();
        let prompt = build_analysis_prompt(&codebase, Some("Focus on error handling"));

        assert!(prompt.contains("Special Focus:"));
        assert!(prompt.contains("Focus on error handling"));
    }

    #[test]
    fn test_build_refinement_prompt_cursor() {
        let analysis = "This is a Rust project with async patterns.";
        let prompt = build_refinement_prompt(analysis, "cursor", Some("always"));

        assert!(prompt.contains("Cursor IDE rules"));
        assert!(prompt.contains(".mdc format"));
        assert!(prompt.contains(analysis));
    }

    #[test]
    fn test_build_refinement_prompt_claude() {
        let analysis = "This is a Python project.";
        let prompt = build_refinement_prompt(analysis, "claude", None);

        assert!(prompt.contains("CLAUDE.md"));
        assert!(prompt.contains(analysis));
    }

    #[test]
    fn test_build_refinement_prompt_copilot() {
        let analysis = "This is a TypeScript project.";
        let prompt = build_refinement_prompt(analysis, "copilot", None);

        assert!(prompt.contains("Copilot"));
        assert!(prompt.contains(analysis));
    }

    #[test]
    fn test_build_smart_merge_prompt() {
        let existing = "# Existing Rules\n- Rule 1\n- Rule 2";
        let new_analysis = "New patterns found: async/await usage";
        let prompt = build_smart_merge_prompt(existing, new_analysis);

        assert!(prompt.contains("Previous Rules:"));
        assert!(prompt.contains(existing));
        assert!(prompt.contains("New Analysis:"));
        assert!(prompt.contains(new_analysis));
        assert!(prompt.contains("Preserve"));
    }

    #[test]
    fn test_detect_primary_language() {
        assert_eq!(detect_primary_language("This is a Rust project"), "rust");
        assert_eq!(
            detect_primary_language("Uses TypeScript and React"),
            "typescript"
        );
        assert_eq!(
            detect_primary_language("Python Django application"),
            "python"
        );
        assert_eq!(detect_primary_language("Written in Go lang"), "go");
        assert_eq!(detect_primary_language("Unknown stack"), "text");
    }

    #[test]
    fn test_format_codebase_content() {
        let codebase = create_test_codebase();
        let content = format_codebase_content(&codebase);

        assert!(content.contains("--- src/main.rs ---"));
        assert!(content.contains("--- src/lib.rs ---"));
        assert!(content.contains("fn main()"));
        assert!(content.contains("pub mod utils;"));
    }
}
