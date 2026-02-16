//! Progress bar management for CLI operations.
//!
//! This module provides utilities for displaying progress bars during long-running
//! operations like file scanning, compression, and rule generation.
//!
//! # Overview
//!
//! - [`create_progress_bar`] - Simple single progress bar (backwards compatible)
//! - [`ProgressManager`] - Multi-stage progress management with `MultiProgress`

use std::collections::HashMap;

use console::Term;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

/// Stage names for progress tracking.
///
/// These constants define the recognized stages for progress bar styling.
pub mod stages {
    /// File discovery and scanning stage.
    pub const SCANNING: &str = "scanning";
    /// Tree-sitter compression stage.
    pub const COMPRESSING: &str = "compressing";
    /// LLM analysis stage (spinner-based, no determinate progress).
    pub const ANALYZING: &str = "analyzing";
    /// Format-specific rule generation stage.
    pub const FORMATTING: &str = "formatting";
    /// File writing stage.
    pub const WRITING: &str = "writing";
}

/// Creates a simple progress bar with the default style.
///
/// This is the original progress bar function, maintained for backwards compatibility.
///
/// # Arguments
///
/// * `len` - The total number of items to process.
///
/// # Returns
///
/// A configured `ProgressBar` with the default ruley style.
#[must_use]
pub fn create_progress_bar(len: u64) -> ProgressBar {
    let pb = ProgressBar::new(len);
    // Use unwrap_or_else to fall back to default style if template parsing fails
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} {msg}")
        .unwrap_or_else(|_| ProgressStyle::default_bar())
        .progress_chars("#>-");
    pb.set_style(style);
    pb
}

/// Manager for multi-stage progress bars.
///
/// `ProgressManager` coordinates multiple progress bars displayed simultaneously
/// using indicatif's `MultiProgress`. Each stage can have its own progress bar
/// with stage-specific styling.
///
/// # Example
///
/// ```no_run
/// use ruley::utils::progress::{ProgressManager, stages};
///
/// let mut manager = ProgressManager::new();
///
/// // Add stages
/// let _scanning = manager.add_stage(stages::SCANNING, 100);
///
/// // Update progress
/// manager.update(stages::SCANNING, 50, "src/lib.rs");
///
/// // Mark complete
/// manager.finish(stages::SCANNING, "Scanned 100 files");
/// ```
///
/// # TTY Detection
///
/// When stdout is not a TTY (e.g., piped output, CI environments), progress bars
/// are created as hidden bars that produce no output. This prevents garbled
/// terminal output in non-interactive environments.
pub struct ProgressManager {
    /// The underlying multi-progress container.
    multi: MultiProgress,
    /// Map of stage names to their progress bars.
    bars: HashMap<String, ProgressBar>,
    /// Whether stdout is a TTY (cached at construction time).
    is_tty: bool,
}

impl ProgressManager {
    /// Creates a new progress manager.
    ///
    /// Automatically detects whether stdout is a TTY and adjusts behavior accordingly.
    #[must_use]
    pub fn new() -> Self {
        let is_tty = Term::stdout().is_term();
        Self {
            multi: MultiProgress::new(),
            bars: HashMap::new(),
            is_tty,
        }
    }

    /// Adds a new progress stage with a stage-specific style.
    ///
    /// # Arguments
    ///
    /// * `name` - The stage name (use constants from [`stages`] module).
    /// * `total` - The total number of items to process (ignored for spinner stages).
    ///
    /// # Returns
    ///
    /// The created `ProgressBar`, allowing direct manipulation if needed.
    ///
    /// # Stage Styles
    ///
    /// - **scanning**: `"[{bar:40.cyan/blue}] {pos}/{len} Scanning files... {msg}"`
    /// - **compressing**: `"[{bar:40.cyan/blue}] {pos}/{len} Compressing... ({msg})"`
    /// - **analyzing**: `"{spinner:.green} Analyzing... {msg}"` (spinner, no progress)
    /// - **formatting**: `"[{bar:40.cyan/blue}] {pos}/{len} Generating {msg} format"`
    /// - **writing**: `"[{bar:40.cyan/blue}] {pos}/{len} Writing files... {msg}"`
    ///
    /// Unknown stage names use a generic bar style.
    #[must_use]
    pub fn add_stage(&mut self, name: &str, total: u64) -> ProgressBar {
        let pb = if self.is_tty {
            if name == stages::ANALYZING {
                // Use a spinner for the analyzing stage (indeterminate progress)
                let spinner = ProgressBar::new_spinner();
                spinner.enable_steady_tick(std::time::Duration::from_millis(100));
                self.multi.add(spinner)
            } else {
                let bar = ProgressBar::new(total);
                self.multi.add(bar)
            }
        } else {
            // In non-TTY mode, create hidden progress bars
            ProgressBar::hidden()
        };

        let style = Self::style_for_stage(name);
        pb.set_style(style);

        self.bars.insert(name.to_string(), pb.clone());
        pb
    }

    /// Updates the progress for a stage.
    ///
    /// # Arguments
    ///
    /// * `stage` - The stage name to update.
    /// * `current` - The current position (items processed).
    /// * `message` - A status message to display (e.g., current file name).
    ///
    /// # Note
    ///
    /// If the stage doesn't exist, this method does nothing (no error).
    pub fn update(&self, stage: &str, current: u64, message: &str) {
        if let Some(pb) = self.bars.get(stage) {
            pb.set_position(current);
            pb.set_message(message.to_string());
        }
    }

    /// Marks a stage as complete.
    ///
    /// # Arguments
    ///
    /// * `stage` - The stage name to finish.
    /// * `message` - A final message to display (e.g., summary).
    ///
    /// # Note
    ///
    /// If the stage doesn't exist, this method does nothing (no error).
    pub fn finish(&self, stage: &str, message: &str) {
        if let Some(pb) = self.bars.get(stage) {
            pb.set_message(message.to_string());
            pb.finish();
        }
    }

    /// Finishes a stage with a styled "done" message.
    ///
    /// Unlike [`finish`](Self::finish), this clears the progress bar and replaces
    /// it with a checkmark and the provided message.
    ///
    /// # Arguments
    ///
    /// * `stage` - The stage name to finish.
    /// * `message` - A completion message to display.
    pub fn finish_with_message(&self, stage: &str, message: &str) {
        if let Some(pb) = self.bars.get(stage) {
            pb.finish_with_message(message.to_string());
        }
    }

    /// Abandons a stage, clearing its progress bar.
    ///
    /// Use this when a stage is cancelled or encounters an error.
    ///
    /// # Arguments
    ///
    /// * `stage` - The stage name to abandon.
    pub fn abandon(&self, stage: &str) {
        if let Some(pb) = self.bars.get(stage) {
            pb.abandon();
        }
    }

    /// Returns the progress style for a given stage name.
    fn style_for_stage(name: &str) -> ProgressStyle {
        let template = match name {
            stages::SCANNING => "[{bar:40.cyan/blue}] {pos}/{len} Scanning files... {msg}",
            stages::COMPRESSING => "[{bar:40.cyan/blue}] {pos}/{len} Compressing... ({msg})",
            stages::ANALYZING => "{spinner:.green} Analyzing... {msg}",
            stages::FORMATTING => "[{bar:40.cyan/blue}] {pos}/{len} Generating {msg} format",
            stages::WRITING => "[{bar:40.cyan/blue}] {pos}/{len} Writing files... {msg}",
            // Default style for unknown stages
            _ => "[{bar:40.cyan/blue}] {pos}/{len} {msg}",
        };

        ProgressStyle::default_bar()
            .template(template)
            .unwrap_or_else(|_| ProgressStyle::default_bar())
            .progress_chars("#>-")
    }

    /// Returns the underlying `MultiProgress` for advanced usage.
    ///
    /// This is useful when you need to add custom progress bars that don't
    /// fit the standard stage model.
    #[must_use]
    pub fn multi_progress(&self) -> &MultiProgress {
        &self.multi
    }

    /// Returns whether the manager is running in TTY mode.
    ///
    /// When `false`, progress bars are hidden and produce no output.
    #[must_use]
    pub fn is_tty(&self) -> bool {
        self.is_tty
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ProgressManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressManager")
            .field("stages", &self.bars.keys().collect::<Vec<_>>())
            .field("is_tty", &self.is_tty)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_manager_add_stage() {
        let mut manager = ProgressManager::new();

        // Add multiple stages
        let _scanning = manager.add_stage(stages::SCANNING, 100);
        let _compressing = manager.add_stage(stages::COMPRESSING, 50);

        // Verify stages exist
        assert!(manager.bars.contains_key(stages::SCANNING));
        assert!(manager.bars.contains_key(stages::COMPRESSING));
        assert_eq!(manager.bars.len(), 2);
    }

    #[test]
    fn test_progress_manager_update_nonexistent_stage() {
        let manager = ProgressManager::new();

        // Should not panic when updating a stage that doesn't exist
        manager.update("nonexistent", 50, "test message");
    }

    #[test]
    fn test_progress_manager_finish_nonexistent_stage() {
        let manager = ProgressManager::new();

        // Should not panic when finishing a stage that doesn't exist
        manager.finish("nonexistent", "done");
    }

    #[test]
    fn test_stage_specific_styles() {
        // Verify each stage produces a valid style (no panic)
        let _ = ProgressManager::style_for_stage(stages::SCANNING);
        let _ = ProgressManager::style_for_stage(stages::COMPRESSING);
        let _ = ProgressManager::style_for_stage(stages::ANALYZING);
        let _ = ProgressManager::style_for_stage(stages::FORMATTING);
        let _ = ProgressManager::style_for_stage(stages::WRITING);
        let _ = ProgressManager::style_for_stage("unknown");
    }

    #[test]
    fn test_create_progress_bar_backwards_compat() {
        // Ensure the original function still works
        let pb = create_progress_bar(100);
        assert_eq!(pb.length(), Some(100));
    }

    #[test]
    fn test_progress_manager_default() {
        // Verify Default trait implementation works
        let manager = ProgressManager::default();
        assert!(manager.bars.is_empty());
    }
}
