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
//! 7. **Complete** - Pipeline completed successfully
//!
//! ## Architecture
//!
//! The `Context` struct carries state through all pipeline stages, containing:
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

use anyhow::Result;
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
    /// Verbosity level (0-3)
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
    /// Initial setup and configuration validation
    Init,
    /// Repository scanning and file discovery
    Scanning,
    /// Optional tree-sitter compression of source files
    Compressing,
    /// LLM analysis and rule generation
    Analyzing,
    /// Converting LLM output to target formats
    Formatting,
    /// Writing output files to disk
    Writing,
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

    /// Attempt to delete all tracked temporary files and clear the list
    pub fn clear(&mut self) -> std::io::Result<()> {
        for path in &self.files {
            if path.exists() {
                std::fs::remove_file(path)?;
            }
        }
        self.files.clear();
        Ok(())
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

/// Context carries state through all pipeline stages.
/// This is the primary state container passed through all pipeline operations,
/// containing the resolved configuration, current execution stage, temporary
/// file tracking, and progress information.
#[derive(Debug)]
pub struct Context {
    /// Final resolved configuration from all sources
    pub config: MergedConfig,
    /// Current pipeline execution stage
    pub stage: PipelineStage,
    /// Temporary file tracking for cleanup
    pub temp_files: TempFileRefs,
    /// Progress tracking (stub for now)
    pub progress: ProgressTracker,
}

impl Context {
    /// Create a new Context with the given merged configuration.
    /// Initializes with PipelineStage::Init and empty tracking structures.
    pub fn new(config: MergedConfig) -> Self {
        Self {
            config,
            stage: PipelineStage::Init,
            temp_files: TempFileRefs::new(),
            progress: ProgressTracker::new(),
        }
    }

    /// Update the current pipeline stage
    pub fn set_stage(&mut self, stage: PipelineStage) {
        self.stage = stage;
    }
}

pub async fn run() -> Result<()> {
    let args = cli::args::parse();
    let _config = cli::config::load(&args)?;

    // TODO: Implement orchestrator
    tracing::info!("ruley initialized");

    Ok(())
}
