//! # ruley Pipeline Infrastructure
//!
//! ruley implements a comprehensive 10-stage pipeline architecture for generating
//! AI IDE rules from codebases:
//!
//! 1. **Init** - Configuration validation and setup
//! 2. **Scanning** - Repository scanning and file discovery
//! 3. **Compressing** - Optional tree-sitter compression of source files
//! 4. **Analyzing** - LLM analysis and rule generation
//! 5. **Formatting** - Converting LLM output to target formats
//! 6. **Writing** - Writing output files to disk
//! 7. **Validating** - Validation of generated outputs
//! 8. **Finalizing** - Post-processing and finalization
//! 9. **Reporting** - Reporting and summary generation
//! 10. **Cleanup** - Cleanup of temporary files and resources
//!
//! After all stages complete, the pipeline transitions to the **Complete** terminal state.
//!
//! ## Architecture
//!
//! The `PipelineContext` struct carries state through all pipeline stages, containing:
//! - `config: MergedConfig` - Final resolved configuration from all sources
//! - `stage: PipelineStage` - Current pipeline execution stage
//! - `temp_files: TempFileRefs` - Temporary file tracking for cleanup
//! - `progress: ProgressTracker` - Progress tracking (stub for now)
//!
//! Configuration follows hierarchical precedence:
//! 1. User config (~/.config/ruley/config.toml)
//! 2. Git root (ruley.toml)
//! 3. Current directory (ruley.toml)
//! 4. Explicit --config path
//! 5. Environment variables (RULEY_*)
//! 6. CLI flags (highest precedence)
//!
//! The `MergedConfig` struct represents the final resolved configuration after
//! merging all sources (CLI flags override env vars override config files).

pub mod cli;
pub mod generator;
pub mod llm;
pub mod output;
pub mod packer;
pub mod utils;

use anyhow::{Context, Result};
use cli::config::{ChunkingConfig, ProvidersConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Final resolved configuration after merging all sources (CLI, env, config files).
/// This struct represents the single source of truth for all configuration values
/// used throughout the pipeline execution.
#[derive(Debug, Clone)]
pub struct MergedConfig {
    /// LLM provider (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model name (optional, provider may have default)
    pub model: Option<String>,
    /// Output formats (e.g., ["cursor", "claude"])
    pub format: Vec<String>,
    /// Output file path (optional)
    pub output: Option<PathBuf>,
    /// Path to repomix file (optional)
    pub repomix_file: Option<PathBuf>,
    /// Repository path to process
    pub path: PathBuf,
    /// Description for rule generation (optional)
    pub description: Option<String>,
    /// Rule type to generate
    pub rule_type: String,
    /// File include patterns
    pub include: Vec<String>,
    /// File exclude patterns
    pub exclude: Vec<String>,
    /// Enable tree-sitter compression
    pub compress: bool,
    /// Maximum chunk size for processing
    pub chunk_size: usize,
    /// Skip cost confirmation prompt
    pub no_confirm: bool,
    /// Dry run mode (show what would be processed)
    pub dry_run: bool,
    /// Verbosity level (0 = INFO, 1 = DEBUG, 2+ = TRACE)
    pub verbose: u8,
    /// Quiet mode (suppress all output)
    pub quiet: bool,
    /// Optional chunking configuration from config file
    pub chunking: Option<ChunkingConfig>,
    /// Output path mappings by format
    pub output_paths: HashMap<String, String>,
    /// Provider-specific configurations
    pub providers: ProvidersConfig,
}

/// Tracks the current stage of pipeline execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineStage {
    /// Stage 1: Initial setup and configuration validation
    Init,
    /// Stage 2: Repository scanning and file discovery
    Scanning,
    /// Stage 3: Optional tree-sitter compression of source files
    Compressing,
    /// Stage 4: LLM analysis and rule generation
    Analyzing,
    /// Stage 5: Converting LLM output to target formats
    Formatting,
    /// Stage 6: Writing output files to disk
    Writing,
    /// Stage 7: Validation of generated outputs
    Validating,
    /// Stage 8: Post-processing and finalization
    Finalizing,
    /// Stage 9: Reporting and summary generation
    Reporting,
    /// Stage 10: Cleanup of temporary files and resources
    Cleanup,
    /// Pipeline completed successfully
    Complete,
}

/// Tracks temporary files created during pipeline execution for cleanup.
/// Used for cleanup in error paths and stage 10 completion.
#[derive(Debug, Default)]
pub struct TempFileRefs {
    /// Paths of temporary files to clean up
    files: Vec<PathBuf>,
}

impl TempFileRefs {
    /// Create a new empty TempFileRefs instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a temporary file for tracking and cleanup
    pub fn add(&mut self, path: PathBuf) {
        self.files.push(path);
    }

    /// Attempt to delete all tracked temporary files and clear the list.
    /// Continues on individual file deletion failures to ensure all files are attempted.
    /// Returns the last error encountered, if any.
    pub fn clear(&mut self) -> std::io::Result<()> {
        let mut last_error = None;
        for path in &self.files {
            if path.exists()
                && let Err(e) = std::fs::remove_file(path)
            {
                tracing::warn!("Failed to delete temp file {}: {}", path.display(), e);
                last_error = Some(e);
            }
        }
        self.files.clear();
        last_error.map_or(Ok(()), Err)
    }
}

/// Stub for progress tracking functionality.
/// Will be fully implemented in Ticket 7 to provide real-time feedback
/// on pipeline execution across all stages.
#[derive(Debug, Default)]
pub struct ProgressTracker {
    /// Placeholder field for future implementation
    _placeholder: (),
}

impl ProgressTracker {
    /// Create a new ProgressTracker instance
    pub fn new() -> Self {
        Self::default()
    }
}

/// PipelineContext carries state through all pipeline stages.
/// This is the primary state container passed through all pipeline operations,
/// containing the resolved configuration, current execution stage, temporary
/// file tracking, and progress information.
#[derive(Debug)]
pub struct PipelineContext {
    /// Final resolved configuration from all sources
    pub config: MergedConfig,
    /// Current pipeline execution stage
    pub stage: PipelineStage,
    /// Temporary file tracking for cleanup
    pub temp_files: TempFileRefs,
    /// Progress tracking (stub for now)
    pub progress: ProgressTracker,
}

impl PipelineContext {
    /// Create a new PipelineContext with the given merged configuration.
    /// Initializes with PipelineStage::Init and empty tracking structures.
    pub fn new(config: MergedConfig) -> Self {
        Self {
            config,
            stage: PipelineStage::Init,
            temp_files: TempFileRefs::new(),
            progress: ProgressTracker::new(),
        }
    }

    /// Transition to a new pipeline stage with logging.
    /// This is the recommended way to update the pipeline stage as it
    /// provides consistent logging for stage transitions.
    pub fn transition_to(&mut self, stage: PipelineStage) {
        self.stage = stage;
        tracing::info!("Pipeline stage: {:?}", stage);
    }
}

pub async fn run(config: MergedConfig) -> Result<()> {
    // Initialize logging based on verbosity level
    let level = match config.verbose {
        0 => tracing::Level::INFO,
        1 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    // Use try_init() to gracefully handle cases where logging is already initialized
    // (e.g., in tests or when the library is used multiple times)
    let _ = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .without_time()
        .try_init();

    // Log version and configuration summary
    tracing::info!("ruley v{} starting", env!("CARGO_PKG_VERSION"));
    tracing::debug!(
        "Configuration: provider={}, model={:?}, format={:?}, compress={}, chunk_size={}",
        config.provider,
        config.model,
        config.format,
        config.compress,
        config.chunk_size
    );

    // Initialize context
    let mut ctx = PipelineContext::new(config);

    // Stage 1: Init (Configuration Validation)
    ctx.transition_to(PipelineStage::Init);

    // Validate repository path exists
    if !ctx.config.path.exists() {
        return Err(anyhow::anyhow!(
            "Repository path does not exist: {}",
            ctx.config.path.display()
        ))
        .context("Failed to validate repository path");
    }

    // Check for dry-run mode
    if ctx.config.dry_run {
        display_dry_run_config(&ctx.config);
        return Ok(());
    }

    // Stage 2: Scanning
    ctx.transition_to(PipelineStage::Scanning);
    // TODO: Implement repository scanning

    // Stage 3: Compressing
    ctx.transition_to(PipelineStage::Compressing);
    // TODO: Implement tree-sitter compression

    // Stage 4: Analyzing
    ctx.transition_to(PipelineStage::Analyzing);
    // TODO: Implement LLM analysis

    // Stage 5: Formatting
    ctx.transition_to(PipelineStage::Formatting);
    // TODO: Implement output formatting

    // Stage 6: Writing
    ctx.transition_to(PipelineStage::Writing);
    // TODO: Implement file writing

    // Stage 7: Validating
    ctx.transition_to(PipelineStage::Validating);
    // TODO: Implement output validation

    // Stage 8: Finalizing
    ctx.transition_to(PipelineStage::Finalizing);
    // TODO: Implement post-processing and finalization

    // Stage 9: Reporting
    ctx.transition_to(PipelineStage::Reporting);
    // TODO: Implement reporting and summary generation

    // Stage 10: Cleanup
    ctx.transition_to(PipelineStage::Cleanup);
    cleanup_temp_files(&mut ctx).context("Failed to cleanup temporary files")?;

    // Pipeline Complete
    ctx.transition_to(PipelineStage::Complete);
    tracing::info!("Pipeline completed successfully");

    Ok(())
}

/// Cleanup temporary files created during pipeline execution.
fn cleanup_temp_files(ctx: &mut PipelineContext) -> Result<()> {
    let file_count = ctx.temp_files.files.len();
    ctx.temp_files
        .clear()
        .context("Failed to remove temporary files")?;

    tracing::debug!("Cleaned up {} temporary files", file_count);
    Ok(())
}

/// Display configuration summary for dry-run mode.
fn display_dry_run_config(config: &MergedConfig) {
    let include_str = if config.include.is_empty() {
        "none".to_string()
    } else {
        config.include.join(", ")
    };

    let exclude_str = if config.exclude.is_empty() {
        "none".to_string()
    } else {
        config.exclude.join(", ")
    };

    println!("Dry Run Mode - Configuration Summary");
    println!("=====================================");
    println!("Provider:     {}", config.provider);
    println!(
        "Model:        {}",
        config.model.as_deref().unwrap_or("default")
    );
    println!("Format:       {}", config.format.join(", "));
    println!("Path:         {}", config.path.display());
    println!("Compress:     {}", config.compress);
    println!("Chunk Size:   {}", config.chunk_size);
    println!("Include:      {}", include_str);
    println!("Exclude:      {}", exclude_str);
    println!("No Confirm:   {}", config.no_confirm);
    println!();
    println!("No LLM calls will be made.");
}
