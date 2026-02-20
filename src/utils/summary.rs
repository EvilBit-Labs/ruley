// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Success summary display for completed pipeline runs.
//!
//! This module provides functions for displaying a comprehensive summary
//! after a successful rule generation run, including output files,
//! statistics, and next steps guidance.
//!
//! # Example
//!
//! ```ignore
//! use ruley::utils::summary::display_success_summary;
//! use std::time::Duration;
//!
//! display_success_summary(
//!     &output_results,
//!     files_analyzed,
//!     tokens_processed,
//!     compression_ratio,
//!     actual_cost,
//!     Duration::from_secs_f32(12.3),
//!     false, // quiet
//! )?;
//! ```

use crate::output::OutputResult;
use crate::utils::formatting::format_number;
use anyhow::Result;
use console::{Term, style};
use std::io::Write;
use std::time::Duration;

/// Display a success summary after rule generation completes.
///
/// Shows output files with sizes, statistics, and next steps.
///
/// # Arguments
///
/// * `output_results` - The results from writing output files
/// * `files_analyzed` - Number of files that were analyzed
/// * `tokens_processed` - Total tokens sent to LLM
/// * `compression_ratio` - Compression ratio if compression was used (0.0-1.0)
/// * `actual_cost` - Actual cost incurred (in dollars)
/// * `elapsed` - Time elapsed for the entire operation
/// * `quiet` - If true, suppresses output entirely
///
/// # Errors
///
/// Returns an error if writing to the terminal fails.
#[allow(clippy::too_many_arguments)]
pub fn display_success_summary(
    output_results: &[OutputResult],
    files_analyzed: usize,
    tokens_processed: usize,
    compression_ratio: Option<f32>,
    actual_cost: f64,
    elapsed: Duration,
    quiet: bool,
) -> Result<()> {
    if quiet {
        return Ok(());
    }

    let mut term = Term::stdout();

    // Success header with checkmark
    writeln!(term)?;
    writeln!(
        term,
        "{} {}",
        style("\u{2713}").green().bold(),
        style("Rules generated successfully").bold()
    )?;

    // Output Files section
    writeln!(term)?;
    writeln!(term, "{}:", style("Output Files").bold())?;

    for (i, result) in output_results.iter().enumerate() {
        let prefix = if i == output_results.len() - 1 {
            "\u{2514}\u{2500}"
        } else {
            "\u{251c}\u{2500}"
        };

        let size = get_file_size(&result.path);
        let format_display = format_name_display(&result.format);

        writeln!(
            term,
            "{} {}: {} ({})",
            style(prefix).dim(),
            format_display,
            result.path.display(),
            format_size(size)
        )?;
    }

    // Statistics section
    writeln!(term)?;
    writeln!(term, "{}:", style("Statistics").bold())?;

    // Files analyzed
    writeln!(
        term,
        "{} Files analyzed: {}",
        style("\u{251c}\u{2500}").dim(),
        format_number(files_analyzed)
    )?;

    // Tokens processed
    writeln!(
        term,
        "{} Tokens processed: {}",
        style("\u{251c}\u{2500}").dim(),
        format_number(tokens_processed)
    )?;

    // Compression (if applicable)
    if let Some(ratio) = compression_ratio
        && ratio < 1.0
        && ratio > 0.0
    {
        let percent = ((1.0 - ratio) * 100.0) as u32;
        writeln!(
            term,
            "{} Compression: {}% reduction",
            style("\u{251c}\u{2500}").dim(),
            percent
        )?;
    }

    // Actual cost
    writeln!(
        term,
        "{} Actual cost: {}",
        style("\u{251c}\u{2500}").dim(),
        style(format!("${:.2}", actual_cost)).green()
    )?;

    // Time elapsed
    writeln!(
        term,
        "{} Time: {}",
        style("\u{2514}\u{2500}").dim(),
        format_duration(elapsed)
    )?;

    // Next Steps section
    writeln!(term)?;
    writeln!(term, "{}:", style("Next Steps").bold())?;
    writeln!(term, "\u{2022} Restart your AI IDE to load the new rules")?;
    writeln!(term, "\u{2022} Test AI suggestions in your codebase")?;
    writeln!(term, "\u{2022} Re-run ruley when conventions change")?;
    writeln!(term)?;

    Ok(())
}

/// Get the file size in bytes.
fn get_file_size(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Format a file size for display (e.g., "3.2 KB").
fn format_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

/// Format a duration for display (e.g., "12.3s" or "1m 23s").
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs_f64();
    if secs >= 60.0 {
        let mins = (secs / 60.0).floor() as u64;
        let remaining_secs = secs - (mins as f64 * 60.0);
        format!("{}m {:.1}s", mins, remaining_secs)
    } else {
        format!("{:.1}s", secs)
    }
}

/// Get a display name for a format.
fn format_name_display(format: &str) -> String {
    match format.to_lowercase().as_str() {
        "cursor" => "Cursor".to_string(),
        "claude" => "Claude".to_string(),
        "copilot" => "Copilot".to_string(),
        "windsurf" => "Windsurf".to_string(),
        "aider" => "Aider".to_string(),
        "generic" => "Generic".to_string(),
        "json" => "JSON".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_output_result(format: &str, path: PathBuf) -> OutputResult {
        OutputResult {
            format: format.to_string(),
            path,
            backup_created: false,
            backup_path: None,
            is_new: true,
            skipped: false,
            smart_merged: false,
        }
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(3276), "3.2 KB");
        assert_eq!(format_size(1048576), "1.0 MB");
        assert_eq!(format_size(2621440), "2.5 MB");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs_f64(0.5)), "0.5s");
        assert_eq!(format_duration(Duration::from_secs_f64(12.3)), "12.3s");
        assert_eq!(format_duration(Duration::from_secs_f64(59.9)), "59.9s");
        assert_eq!(format_duration(Duration::from_secs_f64(60.0)), "1m 0.0s");
        assert_eq!(format_duration(Duration::from_secs_f64(83.5)), "1m 23.5s");
        assert_eq!(format_duration(Duration::from_secs_f64(150.0)), "2m 30.0s");
    }

    #[test]
    fn test_format_name_display() {
        assert_eq!(format_name_display("cursor"), "Cursor");
        assert_eq!(format_name_display("claude"), "Claude");
        assert_eq!(format_name_display("copilot"), "Copilot");
        assert_eq!(format_name_display("windsurf"), "Windsurf");
        assert_eq!(format_name_display("aider"), "Aider");
        assert_eq!(format_name_display("generic"), "Generic");
        assert_eq!(format_name_display("json"), "JSON");
        assert_eq!(format_name_display("unknown"), "unknown");
    }

    #[test]
    fn test_display_success_summary_quiet_mode() {
        let results = vec![];
        let result = display_success_summary(
            &results,
            100,
            50000,
            Some(0.3),
            0.14,
            Duration::from_secs(12),
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_success_summary_with_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create test files with content
        let cursor_path = temp_dir.path().join(".cursor/rules/project.mdc");
        std::fs::create_dir_all(cursor_path.parent().unwrap()).unwrap();
        std::fs::write(&cursor_path, "a".repeat(3276)).unwrap(); // ~3.2 KB

        let claude_path = temp_dir.path().join("CLAUDE.md");
        std::fs::write(&claude_path, "b".repeat(2867)).unwrap(); // ~2.8 KB

        let results = vec![
            create_test_output_result("cursor", cursor_path),
            create_test_output_result("claude", claude_path),
        ];

        let result = display_success_summary(
            &results,
            127,
            48234,
            Some(0.31),
            0.14,
            Duration::from_secs_f64(12.3),
            false,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_display_success_summary_no_compression() {
        let results = vec![];
        let result = display_success_summary(
            &results,
            50,
            10000,
            None, // No compression
            0.05,
            Duration::from_secs(5),
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_display_success_summary_long_duration() {
        let results = vec![];
        let result = display_success_summary(
            &results,
            500,
            200000,
            Some(0.25),
            1.50,
            Duration::from_secs_f64(150.5), // 2m 30.5s
            false,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_file_size_nonexistent() {
        let path = PathBuf::from("/nonexistent/path/file.txt");
        assert_eq!(get_file_size(&path), 0);
    }

    #[test]
    fn test_get_file_size_existing() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let size = get_file_size(&file_path);
        assert_eq!(size, 11);
    }
}
