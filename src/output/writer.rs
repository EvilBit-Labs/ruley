// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! File writing logic for output formatters.
//!
//! This module handles the file writing stage of the pipeline:
//! - Determining output paths (respecting user overrides)
//! - Detecting file conflicts with configurable resolution strategies
//! - Interactive prompts for conflict resolution (when TTY is available)
//! - Smart merge via LLM for merging existing and new rules
//! - Creating timestamped backup files with automatic cleanup
//! - Writing output files to disk

use crate::generator::prompts::build_smart_merge_prompt;
use crate::generator::rules::GeneratedRules;
use crate::llm::client::LLMClient;
use crate::llm::cost::{CostCalculator, CostTracker};
use crate::llm::provider::{CompletionOptions, Message};
use crate::output::{Metadata, get_formatter};
use crate::utils::error::RuleyError;
use anyhow::{Context, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Maximum number of backup files to keep per output file.
const MAX_BACKUPS: usize = 5;

/// Conflict resolution strategy for existing output files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Prompt the user interactively for each conflict (default)
    Prompt,
    /// Overwrite existing files without confirmation
    Overwrite,
    /// Skip writing if file already exists
    Skip,
    /// Use LLM to smart-merge existing and new content
    SmartMerge,
}

impl FromStr for ConflictStrategy {
    type Err = RuleyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "prompt" => Ok(Self::Prompt),
            "overwrite" => Ok(Self::Overwrite),
            "skip" => Ok(Self::Skip),
            "smart-merge" | "smartmerge" | "smart_merge" => Ok(Self::SmartMerge),
            _ => Err(RuleyError::OutputFormat(format!(
                "Invalid conflict strategy: '{}'. Valid values: prompt, overwrite, skip, smart-merge",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ConflictStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prompt => write!(f, "prompt"),
            Self::Overwrite => write!(f, "overwrite"),
            Self::Skip => write!(f, "skip"),
            Self::SmartMerge => write!(f, "smart-merge"),
        }
    }
}

/// Resolution chosen for a specific file conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConflictResolution {
    /// Overwrite the existing file
    Overwrite,
    /// Skip this file
    Skip,
    /// Smart merge with LLM
    SmartMerge,
    /// Apply this resolution to all remaining files
    All(Box<ConflictResolution>),
    /// Abort the entire write operation
    Quit,
}

/// Options for controlling output file writing.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Base directory for output files (usually project root)
    pub base_path: PathBuf,
    /// Custom output paths by format (overrides defaults)
    pub output_paths: HashMap<String, String>,
    /// Whether to create backups of existing files
    pub create_backups: bool,
    /// Conflict resolution strategy
    pub conflict_strategy: ConflictStrategy,
    /// Whether running in an interactive TTY environment
    pub is_interactive: bool,
}

impl WriteOptions {
    /// Create new write options with the given base path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            output_paths: HashMap::new(),
            create_backups: true,
            conflict_strategy: ConflictStrategy::Prompt,
            is_interactive: false,
        }
    }

    /// Set custom output paths.
    pub fn with_output_paths(self, paths: HashMap<String, String>) -> Self {
        Self {
            output_paths: paths,
            ..self
        }
    }

    /// Set whether to create backups.
    pub fn with_backups(self, create_backups: bool) -> Self {
        Self {
            create_backups,
            ..self
        }
    }

    /// Set the conflict resolution strategy.
    pub fn with_conflict_strategy(self, conflict_strategy: ConflictStrategy) -> Self {
        Self {
            conflict_strategy,
            ..self
        }
    }

    /// Set whether the environment is interactive (TTY).
    pub fn with_interactive(self, is_interactive: bool) -> Self {
        Self {
            is_interactive,
            ..self
        }
    }
}

/// Result of writing an output file.
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Format that was written
    pub format: String,
    /// Path where the file was written
    pub path: PathBuf,
    /// Whether a backup was created
    pub backup_created: bool,
    /// Path to the backup file (if created)
    pub backup_path: Option<PathBuf>,
    /// Whether the file was newly created (vs overwritten)
    pub is_new: bool,
    /// Whether the file was skipped due to conflict resolution
    pub skipped: bool,
    /// Whether the file was smart-merged via LLM
    pub smart_merged: bool,
}

/// Context for smart merge operations requiring LLM access.
pub struct SmartMergeContext<'a> {
    /// LLM client for calling the merge prompt
    pub client: Option<&'a LLMClient>,
    /// Cost tracker for recording LLM usage
    pub cost_tracker: &'a mut Option<CostTracker>,
    /// Cost calculator for estimating merge costs
    pub calculator: Option<&'a CostCalculator>,
    /// Whether to skip cost confirmation prompts
    pub no_confirm: bool,
}

/// Write output files for all generated formats.
///
/// Handles conflict resolution for each file, including interactive prompts,
/// smart merge via LLM, and backup management.
#[allow(clippy::too_many_arguments)]
pub async fn write_output(
    rules: &GeneratedRules,
    formats: &[String],
    project_name: &str,
    options: &WriteOptions,
    client: Option<&LLMClient>,
    cost_tracker: &mut Option<CostTracker>,
    calculator: Option<&CostCalculator>,
    no_confirm: bool,
) -> Result<Vec<OutputResult>, RuleyError> {
    let mut results = Vec::with_capacity(formats.len());
    // Tracks "All" choice: once chosen, applies to remaining files
    let mut apply_all: Option<ConflictResolution> = None;

    let mut merge_ctx = SmartMergeContext {
        client,
        cost_tracker,
        calculator,
        no_confirm,
    };

    for format in formats {
        let result = write_format(
            rules,
            format,
            project_name,
            options,
            &mut apply_all,
            &mut merge_ctx,
        )
        .await?;
        results.push(result);
    }

    Ok(results)
}

/// Write output for a single format, handling conflict resolution.
async fn write_format(
    rules: &GeneratedRules,
    format: &str,
    project_name: &str,
    options: &WriteOptions,
    apply_all: &mut Option<ConflictResolution>,
    merge_ctx: &mut SmartMergeContext<'_>,
) -> Result<OutputResult, RuleyError> {
    let formatter = get_formatter(format)?;

    let metadata = Metadata {
        project_name: project_name.to_string(),
        format: format.to_string(),
    };

    // Get the formatted content
    let content = formatter.format(rules, &metadata)?;

    // Determine output path
    let output_path = determine_output_path(format, formatter.as_ref(), options);

    // Check for existing file
    let file_exists = output_path.exists();

    // No conflict - write directly
    if !file_exists {
        ensure_parent_dir(&output_path)?;
        write_file(&output_path, &content)?;

        tracing::info!("Wrote {} format to {}", format, output_path.display());

        return Ok(OutputResult {
            format: format.to_string(),
            path: output_path,
            backup_created: false,
            backup_path: None,
            is_new: true,
            skipped: false,
            smart_merged: false,
        });
    }

    // File exists - determine resolution
    let resolution = if let Some(all_resolution) = apply_all {
        // "All" was previously chosen, use that resolution
        all_resolution.clone()
    } else {
        determine_resolution(format, &output_path, options).await?
    };

    // Handle the resolution
    match resolution {
        ConflictResolution::Skip => {
            tracing::info!("Skipped {} (file exists)", format);
            Ok(OutputResult {
                format: format.to_string(),
                path: output_path,
                backup_created: false,
                backup_path: None,
                is_new: false,
                skipped: true,
                smart_merged: false,
            })
        }
        ConflictResolution::Overwrite => {
            let (backup_created, backup_path) =
                handle_backup_and_write(&output_path, &content, options)?;

            tracing::info!("Wrote {} format to {}", format, output_path.display());

            Ok(OutputResult {
                format: format.to_string(),
                path: output_path,
                backup_created,
                backup_path,
                is_new: false,
                skipped: false,
                smart_merged: false,
            })
        }
        ConflictResolution::SmartMerge => {
            let merged_content = smart_merge_file(
                &output_path,
                &rules.analysis,
                merge_ctx.client,
                merge_ctx.cost_tracker,
                merge_ctx.calculator,
                merge_ctx.no_confirm,
            )
            .await
            .map_err(|e| {
                RuleyError::OutputFormat(format!("Smart merge failed for {}: {}", format, e))
            })?;

            let (backup_created, backup_path) =
                handle_backup_and_write(&output_path, &merged_content, options)?;

            tracing::info!(
                "Smart merged {} format to {}",
                format,
                output_path.display()
            );

            Ok(OutputResult {
                format: format.to_string(),
                path: output_path,
                backup_created,
                backup_path,
                is_new: false,
                skipped: false,
                smart_merged: true,
            })
        }
        ConflictResolution::All(inner) => {
            // Store the "All" choice for remaining files
            *apply_all = Some(*inner.clone());

            // Process this file with the inner resolution
            match *inner {
                ConflictResolution::Skip => {
                    tracing::info!("Skipped {} (file exists, applied to all)", format);
                    Ok(OutputResult {
                        format: format.to_string(),
                        path: output_path,
                        backup_created: false,
                        backup_path: None,
                        is_new: false,
                        skipped: true,
                        smart_merged: false,
                    })
                }
                ConflictResolution::Overwrite => {
                    let (backup_created, backup_path) =
                        handle_backup_and_write(&output_path, &content, options)?;

                    tracing::info!(
                        "Wrote {} format to {} (applied to all)",
                        format,
                        output_path.display()
                    );

                    Ok(OutputResult {
                        format: format.to_string(),
                        path: output_path,
                        backup_created,
                        backup_path,
                        is_new: false,
                        skipped: false,
                        smart_merged: false,
                    })
                }
                ConflictResolution::SmartMerge => {
                    let merged_content = smart_merge_file(
                        &output_path,
                        &rules.analysis,
                        merge_ctx.client,
                        merge_ctx.cost_tracker,
                        merge_ctx.calculator,
                        merge_ctx.no_confirm,
                    )
                    .await
                    .map_err(|e| {
                        RuleyError::OutputFormat(format!(
                            "Smart merge failed for {}: {}",
                            format, e
                        ))
                    })?;

                    let (backup_created, backup_path) =
                        handle_backup_and_write(&output_path, &merged_content, options)?;

                    tracing::info!(
                        "Smart merged {} format to {} (applied to all)",
                        format,
                        output_path.display()
                    );

                    Ok(OutputResult {
                        format: format.to_string(),
                        path: output_path,
                        backup_created,
                        backup_path,
                        is_new: false,
                        skipped: false,
                        smart_merged: true,
                    })
                }
                _ => Err(RuleyError::OutputFormat(
                    "Invalid 'All' resolution".to_string(),
                )),
            }
        }
        ConflictResolution::Quit => Err(RuleyError::OutputFormat(
            "Write operation aborted by user".to_string(),
        )),
    }
}

/// Determine the conflict resolution for a file based on strategy and interactivity.
async fn determine_resolution(
    format: &str,
    output_path: &Path,
    options: &WriteOptions,
) -> Result<ConflictResolution, RuleyError> {
    match options.conflict_strategy {
        ConflictStrategy::Overwrite => Ok(ConflictResolution::Overwrite),
        ConflictStrategy::Skip => Ok(ConflictResolution::Skip),
        ConflictStrategy::SmartMerge => {
            if !options.is_interactive {
                return Err(RuleyError::OutputFormat(
                    "Smart merge requires interactive mode for cost confirmation. \
                     Use --on-conflict overwrite or --on-conflict skip in non-interactive mode."
                        .to_string(),
                ));
            }
            Ok(ConflictResolution::SmartMerge)
        }
        ConflictStrategy::Prompt => {
            if !options.is_interactive {
                return Err(RuleyError::OutputFormat(format!(
                    "Output file exists: {}. \
                     Use --on-conflict to specify behavior in non-interactive mode.",
                    output_path.display()
                )));
            }
            prompt_conflict_resolution(output_path, format).await
        }
    }
}

/// Prompt the user interactively for conflict resolution.
///
/// Displays the conflicting file and offers choices:
/// [O]verwrite / [S]kip / Smart [M]erge / [A]ll (apply choice to remaining) / [Q]uit
///
/// When "All" is selected, a follow-up prompt asks which resolution to apply
/// to all remaining files (overwrite, skip, or smart-merge).
async fn prompt_conflict_resolution(
    path: &Path,
    format: &str,
) -> Result<ConflictResolution, RuleyError> {
    let file_size = format_file_size(path);

    loop {
        let mut stdout = tokio::io::stdout();
        let prompt_msg = format!(
            "\nFile exists: {} ({}, {})\n\
             [O]verwrite / [S]kip / Smart [M]erge / [A]ll (apply choice to remaining) / [Q]uit: ",
            path.display(),
            format,
            file_size
        );

        stdout
            .write_all(prompt_msg.as_bytes())
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to write prompt: {}", e)))?;
        stdout
            .flush()
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to flush stdout: {}", e)))?;

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut input = String::new();
        reader
            .read_line(&mut input)
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to read input: {}", e)))?;

        match input.trim().to_lowercase().as_str() {
            "o" | "overwrite" => return Ok(ConflictResolution::Overwrite),
            "s" | "skip" => return Ok(ConflictResolution::Skip),
            "m" | "merge" | "smart-merge" => return Ok(ConflictResolution::SmartMerge),
            "a" | "all" => {
                let inner = prompt_all_resolution().await?;
                return Ok(ConflictResolution::All(Box::new(inner)));
            }
            "q" | "quit" => return Ok(ConflictResolution::Quit),
            _ => {
                let mut stdout = tokio::io::stdout();
                stdout
                    .write_all(b"Invalid choice. Please enter O, S, M, A, or Q.\n")
                    .await
                    .map_err(|e| {
                        RuleyError::OutputFormat(format!("Failed to write error: {}", e))
                    })?;
                stdout.flush().await.map_err(|e| {
                    RuleyError::OutputFormat(format!("Failed to flush stdout: {}", e))
                })?;
            }
        }
    }
}

/// Prompt the user for which resolution to apply to all remaining files.
async fn prompt_all_resolution() -> Result<ConflictResolution, RuleyError> {
    loop {
        let mut stdout = tokio::io::stdout();
        let prompt_msg = "Apply which resolution to all remaining files?\n\
             [O]verwrite / [S]kip / Smart [M]erge: ";

        stdout
            .write_all(prompt_msg.as_bytes())
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to write prompt: {}", e)))?;
        stdout
            .flush()
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to flush stdout: {}", e)))?;

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut input = String::new();
        reader
            .read_line(&mut input)
            .await
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to read input: {}", e)))?;

        match input.trim().to_lowercase().as_str() {
            "o" | "overwrite" => return Ok(ConflictResolution::Overwrite),
            "s" | "skip" => return Ok(ConflictResolution::Skip),
            "m" | "merge" | "smart-merge" => return Ok(ConflictResolution::SmartMerge),
            _ => {
                let mut stdout = tokio::io::stdout();
                stdout
                    .write_all(b"Invalid choice. Please enter O, S, or M.\n")
                    .await
                    .map_err(|e| {
                        RuleyError::OutputFormat(format!("Failed to write error: {}", e))
                    })?;
                stdout.flush().await.map_err(|e| {
                    RuleyError::OutputFormat(format!("Failed to flush stdout: {}", e))
                })?;
            }
        }
    }
}

/// Perform smart merge of existing file content with new analysis using LLM.
///
/// Reads the existing file, builds a smart merge prompt from the existing rules
/// and the raw analysis text, estimates cost, shows confirmation (unless
/// `no_confirm`), and calls the LLM.
async fn smart_merge_file(
    existing_path: &Path,
    new_analysis: &str,
    client: Option<&LLMClient>,
    cost_tracker: &mut Option<CostTracker>,
    calculator: Option<&CostCalculator>,
    no_confirm: bool,
) -> Result<String> {
    let client = client.ok_or_else(|| anyhow::anyhow!("LLM client required for smart merge"))?;

    let existing_content = std::fs::read_to_string(existing_path)
        .with_context(|| format!("Failed to read existing file: {}", existing_path.display()))?;

    let prompt = build_smart_merge_prompt(&existing_content, new_analysis);

    // Show cost estimation if calculator is available
    if let Some(calc) = calculator {
        // Rough token estimate: ~4 chars per token
        let estimated_input = prompt.len() / 4;
        let estimated_output = (existing_content.len() + new_analysis.len()) / 4;
        let estimate = calc.estimate_cost(estimated_input, estimated_output);

        if !no_confirm {
            let mut stdout = tokio::io::stdout();
            let cost_msg = format!(
                "\nSmart merge estimated cost: ${:.4} (~{} input tokens, ~{} output tokens)\n\
                 Proceed? [y/n/s] (y=yes, n=cancel, s=skip and use new content): ",
                estimate.total_cost, estimated_input, estimated_output
            );
            stdout
                .write_all(cost_msg.as_bytes())
                .await
                .context("Failed to write cost prompt")?;
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
                "s" | "skip" => return Ok(existing_content),
                _ => return Err(anyhow::anyhow!("Smart merge cancelled by user")),
            }
        }
    }

    let messages = vec![Message {
        role: "user".to_string(),
        content: prompt,
    }];

    let response = client
        .complete(&messages, &CompletionOptions::default())
        .await
        .context("LLM smart merge call failed")?;

    // Track cost
    if let Some(tracker) = cost_tracker {
        tracker.add_operation(
            format!("smart_merge_{}", existing_path.display()),
            response.prompt_tokens,
            response.completion_tokens,
        );
    }

    Ok(response.content)
}

/// Handle backup creation and file writing.
///
/// Creates a backup if configured, cleans up old backups, then writes the new content.
fn handle_backup_and_write(
    output_path: &Path,
    content: &str,
    options: &WriteOptions,
) -> Result<(bool, Option<PathBuf>), RuleyError> {
    let mut backup_created = false;
    let mut backup_path = None;

    if options.create_backups {
        let backup = create_backup(output_path)?;
        cleanup_old_backups(output_path, MAX_BACKUPS)?;
        backup_created = true;
        backup_path = Some(backup);
    }

    ensure_parent_dir(output_path)?;
    write_file(output_path, content)?;

    Ok((backup_created, backup_path))
}

/// Ensure the parent directory of a path exists.
fn ensure_parent_dir(path: &Path) -> Result<(), RuleyError> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                RuleyError::OutputFormat(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }
    Ok(())
}

/// Write content to a file.
fn write_file(path: &Path, content: &str) -> Result<(), RuleyError> {
    std::fs::write(path, content)
        .map_err(|e| RuleyError::OutputFormat(format!("Failed to write {}: {}", path.display(), e)))
}

/// Format a file's size for display.
fn format_file_size(path: &Path) -> String {
    match std::fs::metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            if size < 1024 {
                format!("{} B", size)
            } else if size < 1024 * 1024 {
                format!("{:.1} KB", size as f64 / 1024.0)
            } else {
                format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
            }
        }
        Err(_) => "unknown size".to_string(),
    }
}

/// Determine the output path for a format.
fn determine_output_path(
    format: &str,
    formatter: &dyn crate::output::OutputFormatter,
    options: &WriteOptions,
) -> PathBuf {
    // Check for custom path override
    if let Some(custom_path) = options.output_paths.get(format) {
        return options.base_path.join(custom_path);
    }

    // Use formatter defaults
    let dir = formatter.default_directory();
    let filename = formatter.default_filename();
    let ext = formatter.extension();

    let file_with_ext = format!("{}.{}", filename, ext);

    if dir.is_empty() {
        options.base_path.join(file_with_ext)
    } else {
        options.base_path.join(dir).join(file_with_ext)
    }
}

/// Create a backup of an existing file.
///
/// If a `.bak` file already exists, uses a timestamped suffix to avoid collisions.
/// Returns the path to the backup file.
fn create_backup(path: &Path) -> Result<PathBuf, RuleyError> {
    let backup_path = generate_backup_path(path);

    std::fs::copy(path, &backup_path).map_err(|e| {
        RuleyError::OutputFormat(format!(
            "Failed to create backup of {}: {}",
            path.display(),
            e
        ))
    })?;

    tracing::debug!(
        "Created backup: {} -> {}",
        path.display(),
        backup_path.display()
    );

    Ok(backup_path)
}

/// Generate a backup path for a file.
///
/// If `file.ext.bak` already exists, generates `file.ext.YYYYMMDD_HHMMSS.bak`
/// to avoid overwriting previous backups.
fn generate_backup_path(path: &Path) -> PathBuf {
    let filename = path
        .file_name()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| {
            tracing::warn!(
                "Path '{}' has no filename component, using empty name for backup",
                path.display()
            );
            std::borrow::Cow::Borrowed("")
        });

    let simple_backup = path.with_file_name(format!("{}.bak", filename));

    if simple_backup.exists() {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        path.with_file_name(format!("{}.{}.bak", filename, timestamp))
    } else {
        simple_backup
    }
}

/// Clean up old backup files, keeping the most recent `keep_count`.
///
/// Finds all `.bak` files matching the original filename pattern,
/// sorts by modification time (newest first), and removes older ones.
fn cleanup_old_backups(original_path: &Path, keep_count: usize) -> Result<(), RuleyError> {
    let Some(parent) = original_path.parent() else {
        return Ok(());
    };
    let Some(filename) = original_path.file_name().map(|s| s.to_string_lossy()) else {
        return Ok(());
    };

    let prefix = format!("{}.", filename);

    let mut backups: Vec<(PathBuf, std::time::SystemTime)> = std::fs::read_dir(parent)
        .map_err(|e| {
            RuleyError::OutputFormat(format!(
                "Failed to read directory {}: {}",
                parent.display(),
                e
            ))
        })?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".bak")
                && (name == format!("{}.bak", filename) || name.starts_with(&prefix))
            {
                let modified = entry.metadata().ok()?.modified().ok()?;
                Some((entry.path(), modified))
            } else {
                None
            }
        })
        .collect();

    if backups.len() <= keep_count {
        return Ok(());
    }

    // Sort newest first
    backups.sort_by(|a, b| b.1.cmp(&a.1));

    // Remove older backups beyond keep_count
    for (path, _) in backups.iter().skip(keep_count) {
        if let Err(e) = std::fs::remove_file(path) {
            tracing::warn!("Failed to remove old backup {}: {}", path.display(), e);
        } else {
            tracing::debug!("Removed old backup: {}", path.display());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_conflict_strategy_from_str() {
        assert_eq!(
            "prompt".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::Prompt
        );
        assert_eq!(
            "overwrite".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::Overwrite
        );
        assert_eq!(
            "skip".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::Skip
        );
        assert_eq!(
            "smart-merge".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::SmartMerge
        );
        assert_eq!(
            "smartmerge".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::SmartMerge
        );
        assert_eq!(
            "smart_merge".parse::<ConflictStrategy>().unwrap(),
            ConflictStrategy::SmartMerge
        );
        assert!("invalid".parse::<ConflictStrategy>().is_err());
    }

    #[test]
    fn test_conflict_strategy_display() {
        assert_eq!(ConflictStrategy::Prompt.to_string(), "prompt");
        assert_eq!(ConflictStrategy::Overwrite.to_string(), "overwrite");
        assert_eq!(ConflictStrategy::Skip.to_string(), "skip");
        assert_eq!(ConflictStrategy::SmartMerge.to_string(), "smart-merge");
    }

    #[test]
    fn test_determine_output_path_default() {
        let formatter = get_formatter("cursor").unwrap();
        let options = WriteOptions::new("/project");

        let path = determine_output_path("cursor", formatter.as_ref(), &options);
        assert_eq!(path, PathBuf::from("/project/.cursor/rules/project.mdc"));
    }

    #[test]
    fn test_determine_output_path_custom() {
        let formatter = get_formatter("cursor").unwrap();
        let mut output_paths = HashMap::new();
        output_paths.insert("cursor".to_string(), "custom/path/rules.mdc".to_string());

        let options = WriteOptions::new("/project").with_output_paths(output_paths);

        let path = determine_output_path("cursor", formatter.as_ref(), &options);
        assert_eq!(path, PathBuf::from("/project/custom/path/rules.mdc"));
    }

    #[test]
    fn test_generate_backup_path_simple() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("CLAUDE.md");
        fs::write(&path, "content").unwrap();

        let backup = generate_backup_path(&path);
        // No existing .bak file, so uses simple suffix
        assert_eq!(backup, temp_dir.path().join("CLAUDE.md.bak"));
    }

    #[test]
    fn test_generate_backup_path_with_existing_bak() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("CLAUDE.md");
        fs::write(&path, "content").unwrap();
        // Create an existing .bak file
        fs::write(temp_dir.path().join("CLAUDE.md.bak"), "old backup").unwrap();

        let backup = generate_backup_path(&path);
        // Should use timestamped format
        let backup_name = backup.file_name().unwrap().to_string_lossy().to_string();
        assert!(backup_name.starts_with("CLAUDE.md."));
        assert!(backup_name.ends_with(".bak"));
        assert_ne!(backup_name, "CLAUDE.md.bak");
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.txt");
        fs::write(&original, "original content").unwrap();

        let backup_path = create_backup(&original).unwrap();

        assert!(backup_path.exists());
        assert_eq!(
            fs::read_to_string(&backup_path).unwrap(),
            "original content"
        );
    }

    #[test]
    fn test_cleanup_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.txt");
        fs::write(&original, "content").unwrap();

        // Create 7 backup files
        for i in 0..7 {
            let backup_name = if i == 0 {
                "test.txt.bak".to_string()
            } else {
                format!("test.txt.2024010{}_120000.bak", i)
            };
            fs::write(temp_dir.path().join(&backup_name), format!("backup {}", i)).unwrap();
        }

        cleanup_old_backups(&original, 3).unwrap();

        let remaining: Vec<_> = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bak"))
            .collect();

        // Should keep at most 3 backups
        assert!(remaining.len() <= 3);
    }

    #[test]
    fn test_write_options_builder() {
        let mut paths = HashMap::new();
        paths.insert("cursor".to_string(), "custom.mdc".to_string());

        let options = WriteOptions::new("/project")
            .with_output_paths(paths.clone())
            .with_backups(false)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(true);

        assert_eq!(options.base_path, PathBuf::from("/project"));
        assert_eq!(options.output_paths, paths);
        assert!(!options.create_backups);
        assert_eq!(options.conflict_strategy, ConflictStrategy::Overwrite);
        assert!(options.is_interactive);
    }

    #[test]
    fn test_format_file_size() {
        let temp_dir = TempDir::new().unwrap();

        let small = temp_dir.path().join("small.txt");
        fs::write(&small, "hello").unwrap();
        let size = format_file_size(&small);
        assert!(size.contains("B"));

        let medium = temp_dir.path().join("medium.txt");
        fs::write(&medium, "x".repeat(2048)).unwrap();
        let size = format_file_size(&medium);
        assert!(size.contains("KB"));
    }

    #[test]
    fn test_format_file_size_nonexistent() {
        let size = format_file_size(Path::new("/nonexistent/file.txt"));
        assert_eq!(size, "unknown size");
    }
}
