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
//! 6. **Validating** - Validation of generated outputs
//! 7. **Finalizing** - Post-processing and finalization
//! 8. **Writing** - Writing output files to disk
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
use chrono::Utc;
use cli::config::{ChunkingConfig, FinalizationConfig, ProvidersConfig, ValidationConfig};
use generator::rules::RuleType;
use llm::chunker::{Chunk, ChunkConfig};
use llm::client::LLMClient;
use llm::cost::{CostCalculator, CostTracker};
use llm::provider::LLMProvider;
#[cfg(feature = "anthropic")]
use llm::tokenizer::AnthropicTokenizer;
use llm::tokenizer::{TiktokenTokenizer, Tokenizer, TokenizerModel};
use std::collections::HashMap;
use std::path::PathBuf;
use utils::cache::TempFileManager;
use utils::finalization::FinalizationResult;
use utils::state::State;
use utils::validation::ValidationResult;

/// Initialize logging based on verbosity level.
/// This should be called once at application startup.
///
/// # Arguments
/// * `verbose` - Verbosity level (0 = INFO, 1 = DEBUG, 2+ = TRACE)
pub fn init_logging(verbose: u8) {
    let level = match verbose {
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
}

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
    pub rule_type: RuleType,
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
    /// Validation stage configuration
    pub validation: ValidationConfig,
    /// Finalization stage configuration
    pub finalization: FinalizationConfig,
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
    /// Stage 6: Validation of generated outputs
    Validating,
    /// Stage 7: Post-processing and finalization
    Finalizing,
    /// Stage 8: Writing output files to disk
    Writing,
    /// Stage 9: Reporting and summary generation
    Reporting,
    /// Stage 10: Cleanup of temporary files and resources
    Cleanup,
    /// Pipeline completed successfully
    Complete,
}

impl std::fmt::Display for PipelineStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Init => write!(f, "Init"),
            Self::Scanning => write!(f, "Scanning"),
            Self::Compressing => write!(f, "Compressing"),
            Self::Analyzing => write!(f, "Analyzing"),
            Self::Formatting => write!(f, "Formatting"),
            Self::Validating => write!(f, "Validating"),
            Self::Finalizing => write!(f, "Finalizing"),
            Self::Writing => write!(f, "Writing"),
            Self::Reporting => write!(f, "Reporting"),
            Self::Cleanup => write!(f, "Cleanup"),
            Self::Complete => write!(f, "Complete"),
        }
    }
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

    /// Returns the number of tracked temporary files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns true if there are no tracked temporary files
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Attempt to delete all tracked temporary files and clear the list.
    ///
    /// Continues on individual file deletion failures to ensure all files are attempted.
    /// All failures are logged with warnings. If any deletions fail, returns an error
    /// with the count of failed deletions.
    pub fn clear(&mut self) -> std::io::Result<()> {
        let mut failure_count = 0;
        let mut last_error = None;

        for path in &self.files {
            if path.exists()
                && let Err(e) = std::fs::remove_file(path)
            {
                tracing::warn!("Failed to delete temp file {}: {}", path.display(), e);
                failure_count += 1;
                last_error = Some(e);
            }
        }
        self.files.clear();

        match (failure_count, last_error) {
            (0, _) => Ok(()),
            (1, Some(e)) => Err(e),
            (n, Some(e)) => Err(std::io::Error::new(
                e.kind(),
                format!("Failed to delete {} temp files (last error: {})", n, e),
            )),
            (_, None) => Ok(()), // Should not happen
        }
    }
}

/// Stub for progress tracking functionality.
/// Will be fully implemented in Ticket 7 to provide real-time feedback
/// on pipeline execution across all stages.
#[derive(Debug, Default)]
pub struct ProgressTracker {
    /// Reserved field for future implementation (current tick count, etc.)
    _reserved: (),
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
    /// Compressed codebase data
    pub compressed_codebase: Option<packer::CompressedCodebase>,
    /// Analysis result from LLM (populated in Stage 4)
    pub analysis_result: Option<String>,
    /// Generated rules from analysis (populated in Stage 4)
    pub generated_rules: Option<generator::GeneratedRules>,
    /// Cost tracking for LLM operations
    pub cost_tracker: Option<CostTracker>,
    /// Cache manager for .ruley/ directory operations
    pub cache_manager: Option<TempFileManager>,
    /// Loaded state from previous runs (for user preferences and metrics)
    pub loaded_state: Option<State>,
    /// Validation results from Stage 6
    pub validation_results: Vec<ValidationResult>,
    /// Finalization result from Stage 7
    pub finalization_result: Option<FinalizationResult>,
}

impl PipelineContext {
    /// Create a new PipelineContext with the given merged configuration.
    /// Initializes with PipelineStage::Init and empty tracking structures.
    pub fn new(config: MergedConfig) -> Self {
        // Initialize progress manager if not in quiet mode
        let progress_manager = if !config.quiet {
            Some(ProgressManager::new())
        } else {
            None
        };

        Self {
            config,
            stage: PipelineStage::Init,
            temp_files: TempFileRefs::new(),
            progress: ProgressTracker::new(),
            compressed_codebase: None,
            analysis_result: None,
            generated_rules: None,
            cost_tracker: None,
            cache_manager: None,
            loaded_state: None,
            validation_results: Vec::new(),
            finalization_result: None,
        }
    }

    /// Transition to a new pipeline stage with logging.
    /// This is the recommended way to update the pipeline stage as it
    /// provides consistent logging for stage transitions.
    pub fn transition_to(&mut self, stage: PipelineStage) {
        self.stage = stage;
        tracing::info!("Pipeline stage: {}", stage);
    }
}

pub async fn run(config: MergedConfig) -> Result<()> {
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

    // Create cache manager
    let cache_manager = TempFileManager::new(&ctx.config.path)?;

    // Cleanup old temp files (24-hour threshold)
    let cleanup_result =
        cache_manager.cleanup_old_temp_files(std::time::Duration::from_secs(24 * 3600))?;
    if cleanup_result.deleted > 0 {
        tracing::info!("Cleaned up {} old temp files", cleanup_result.deleted);
    }

    // Ensure .ruley/ is in .gitignore
    utils::cache::ensure_gitignore_entry(&ctx.config.path)?;

    // Load previous state
    let loaded_state = utils::state::load_state(cache_manager.ruley_dir())?;
    if let Some(ref state) = loaded_state {
        tracing::debug!("Loaded previous state from {:?}", state.last_run);
    }

    ctx.cache_manager = Some(cache_manager);
    ctx.loaded_state = loaded_state;

    // Stage 2: Scanning
    ctx.transition_to(PipelineStage::Scanning);
    let file_entries = if let Some(_path) = &ctx.config.repomix_file {
        tracing::info!("Repomix file mode active, skipping scanning.");
        vec![] // Empty list, scanning is skipped
    } else {
        // Start scanning progress (spinner since we don't know total yet)
        if let Some(ref mut pm) = ctx.progress_manager {
            let _ = pm.add_stage(stages::SCANNING, 0);
            pm.update(stages::SCANNING, 0, "discovering files...");
        }

        let entries = packer::scan_files(&ctx.config.path, &ctx.config)
            .await
            .context("Failed to scan repository files")?;

        if let Some(ref pm) = ctx.progress_manager {
            pm.finish(
                stages::SCANNING,
                &format!("Scanned {} files", entries.len()),
            );
        }
        tracing::info!("Discovered {} files", entries.len());
        entries
    };

    // Write scanned files to cache (for debugging/recovery)
    if let Some(ref cache) = ctx.cache_manager {
        // Convert FileEntry to CachedFileEntry for serialization
        let cached_entries: Vec<utils::cache::CachedFileEntry> = file_entries
            .iter()
            .map(|e| utils::cache::CachedFileEntry {
                path: e.path.clone(),
                size: e.size,
                language: e.language.as_ref().map(|l| format!("{:?}", l)),
            })
            .collect();
        let path = cache.write_scanned_files(&cached_entries)?;
        ctx.temp_files.add(path);
        tracing::debug!("Cached scanned files list");
    }

    // Validate repomix file exists if specified
    if let Some(ref path) = ctx.config.repomix_file
        && !path.exists()
    {
        return Err(anyhow::anyhow!(
            "Repomix file does not exist: {}",
            path.display()
        ))
        .context("Failed to validate repomix file path");
    }

    // Warn about empty file lists (only in normal scan mode, not repomix mode)
    if file_entries.is_empty() && ctx.config.repomix_file.is_none() {
        tracing::warn!("No files found for processing, please check your include/exclude patterns");
    }

    // Stage 3: Compressing
    ctx.transition_to(PipelineStage::Compressing);

    let file_count = file_entries.len() as u64;
    if let Some(ref mut pm) = ctx.progress_manager {
        let _ = pm.add_stage(stages::COMPRESSING, file_count);
        pm.update(stages::COMPRESSING, 0, "processing...");
    }

    let compressed_codebase = if let Some(path) = &ctx.config.repomix_file {
        packer::parse_repomix(path.as_path())
            .await
            .context("Failed to parse repomix file")?
    } else {
        packer::compress_codebase(file_entries, &ctx.config)
            .await
            .context("Failed to compress codebase")?
    };

    if let Some(ref pm) = ctx.progress_manager {
        let ratio = compressed_codebase.metadata.compression_ratio;
        let msg = format!(
            "Compressed {} files ({:.0}% reduction)",
            compressed_codebase.metadata.total_files,
            (1.0 - ratio) * 100.0
        );
        pm.finish(stages::COMPRESSING, &msg);
    }

    ctx.compressed_codebase = Some(compressed_codebase);

    // Write compressed codebase summary to cache
    if let Some(ref cache) = ctx.cache_manager {
        if let Some(ref codebase) = ctx.compressed_codebase {
            // Create a summary string of the compressed codebase for caching
            let summary = format!(
                "Files: {}\nTotal size: {} bytes\nCompression ratio: {:.2}",
                codebase.metadata.total_files,
                codebase.metadata.total_compressed_size,
                codebase.metadata.compression_ratio
            );
            let path = cache.write_compressed_codebase(&summary)?;
            ctx.temp_files.add(path);
            tracing::debug!("Cached compressed codebase summary");
        }
    }

    // Check for dry-run mode (after scanning/compression so we can show file breakdown)
    if ctx.config.dry_run {
        if let Some(ref codebase) = ctx.compressed_codebase {
            // Try to get pricing from client, fall back to default pricing
            // (dry-run shouldn't require API keys)
            let pricing = match create_llm_client(&ctx.config) {
                Ok(client) => client.pricing(),
                Err(_) => get_default_pricing(&ctx.config.provider),
            };

            // Convert Vec<String> to &[String] for display function
            let formats: Vec<String> = ctx.config.format.clone();

            display_dry_run_summary(codebase, &formats, &ctx.config, &pricing)?;
        }
        return Ok(());
    }

    // Stage 4: Analyzing
    ctx.transition_to(PipelineStage::Analyzing);

    // Get the compressed codebase for analysis
    let codebase = ctx
        .compressed_codebase
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No compressed codebase available for analysis"))?;

    // Get the tokenizer for the provider
    let tokenizer = get_tokenizer(&ctx.config.provider, ctx.config.model.as_deref())?;

    // Calculate total tokens in the codebase
    let total_tokens = llm::tokenizer::calculate_tokens(codebase, tokenizer.as_ref());
    tracing::info!("Codebase contains {} tokens", total_tokens);

    // Get context limit for the provider
    let context_limit = get_context_limit(&ctx.config.provider, ctx.config.model.as_deref());

    // Determine chunk configuration
    let chunk_config = if let Some(ref chunking) = ctx.config.chunking {
        let chunk_size = chunking.chunk_size.unwrap_or(ctx.config.chunk_size);
        let overlap = chunking.overlap.unwrap_or(chunk_size / 10);
        ChunkConfig::new(chunk_size, overlap).context("Invalid chunking configuration")?
    } else {
        ChunkConfig::with_chunk_size(ctx.config.chunk_size)
            .context("Invalid chunk size configuration")?
    };

    // Chunk the codebase if needed
    let chunks = if total_tokens > context_limit {
        tracing::info!(
            "Codebase exceeds context limit ({} > {}), chunking required",
            total_tokens,
            context_limit
        );
        llm::chunker::chunk_codebase(codebase, &chunk_config, tokenizer.as_ref())
            .context("Failed to chunk codebase")?
    } else {
        vec![Chunk::from_codebase(codebase, tokenizer.as_ref())]
    };

    tracing::info!("Prepared {} chunk(s) for analysis", chunks.len());

    // Create LLM client (async for providers that fetch dynamic pricing)
    let client = create_llm_client(&ctx.config).await?;

    // Initialize cost tracker
    let pricing = client.pricing();
    let calculator = CostCalculator::new(pricing.clone());
    ctx.cost_tracker = Some(CostTracker::new(calculator.clone()));

    // Build the analysis prompt
    let prompt = generator::build_analysis_prompt(codebase, ctx.config.description.as_deref());

    // Show cost estimation and confirm (unless --no-confirm)
    if !ctx.config.no_confirm {
        // Display the tree-formatted cost estimate
        display_cost_estimate(
            codebase,
            &chunks,
            &ctx.config.format,
            &ctx.config.provider,
            &pricing,
            ctx.config.quiet,
        )?;

        // Prompt for confirmation
        let confirmed = prompt_confirmation("Proceed with LLM analysis?", true).await?;
        if !confirmed {
            tracing::info!("User cancelled operation");
            return Ok(());
        }
    } else if !ctx.config.quiet {
        // Just show the summary without confirmation
        display_cost_estimate(
            codebase,
            &chunks,
            &ctx.config.format,
            &ctx.config.provider,
            &pricing,
            false,
        )?;
    }

    // Start analyzing progress (spinner-based, indeterminate)
    if let Some(ref mut pm) = ctx.progress_manager {
        let _ = pm.add_stage(stages::ANALYZING, 0);
        pm.update(
            stages::ANALYZING,
            0,
            &format!("{} tokens sent", total_tokens),
        );
    }

    // Perform the analysis
    let analysis_result = perform_analysis(&mut ctx, &client, chunks, &prompt).await?;

    if let Some(ref pm) = ctx.progress_manager {
        pm.finish(stages::ANALYZING, "Analysis complete");
    }
    tracing::info!("Analysis complete ({} characters)", analysis_result.len());

    // Parse the analysis into GeneratedRules structure
    let generated_rules = generator::parse_analysis_response(
        &analysis_result,
        &ctx.config.provider,
        ctx.config.model.as_deref().unwrap_or("unknown"),
    )
    .context("Failed to parse analysis response")?;

    // Store the analysis result and generated rules for the next stage
    ctx.analysis_result = Some(analysis_result);
    ctx.generated_rules = Some(generated_rules);

    // Log cost summary if tracking
    if let Some(ref tracker) = ctx.cost_tracker {
        let summary = tracker.summary();
        tracing::info!(
            "LLM cost: ${:.4} ({} operations, {} input tokens, {} output tokens)",
            summary.total_cost,
            summary.operation_count,
            summary.total_input_tokens,
            summary.total_output_tokens
        );
    }

    // Stage 5: Formatting
    ctx.transition_to(PipelineStage::Formatting);

    // Get the analysis result for refinement
    let analysis = ctx
        .analysis_result
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No analysis result available for formatting"))?;

    // Get mutable reference to generated rules
    let rules = ctx
        .generated_rules
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("No generated rules available for formatting"))?;

    // Process each output format
    tracing::info!(
        "Generating format-specific rules for {} format(s)",
        ctx.config.format.len()
    );

    for format in &ctx.config.format {
        tracing::info!("Generating {} format rules", format);

        // Use machine-readable slug for prompt logic (always_apply computation)
        let rule_type_slug = ctx.config.rule_type.slug();

        // Build refinement prompt for this format
        let refinement_prompt =
            generator::build_refinement_prompt(analysis, format, Some(rule_type_slug));

        // Create messages for LLM call
        let messages = vec![llm::provider::Message {
            role: "user".to_string(),
            content: refinement_prompt.clone(),
        }];

        // Call LLM to generate format-specific rules
        let response = client
            .complete(&messages, &llm::provider::CompletionOptions::default())
            .await
            .with_context(|| format!("Failed to generate {} format rules", format))?;

        // Track cost using provider-reported token counts
        if let Some(ref mut tracker) = ctx.cost_tracker {
            tracker.add_operation(
                format!("format_refinement_{}", format),
                response.prompt_tokens,
                response.completion_tokens,
            );
        }

        // Create formatted rules and add to the collection
        let formatted_rules = generator::FormattedRules::with_rule_type(
            format,
            response.content,
            ctx.config.rule_type,
        );
        rules.add_format(formatted_rules);

        tracing::info!("Generated {} format rules successfully", format);
    }

    if let Some(ref pm) = ctx.progress_manager {
        pm.finish(
            stages::FORMATTING,
            &format!("Generated {} format(s)", ctx.config.format.len()),
        );
    }

    // Log final cost summary after all formats processed
    if let Some(ref tracker) = ctx.cost_tracker {
        let summary = tracker.summary();
        tracing::info!(
            "Total LLM cost after formatting: ${:.4} ({} operations, {} input tokens, {} output tokens)",
            summary.total_cost,
            summary.operation_count,
            summary.total_input_tokens,
            summary.total_output_tokens
        );
    }

    // Stage 6: Validating
    ctx.transition_to(PipelineStage::Validating);

    if ctx.config.validation.enabled {
        tracing::info!("Validating generated rules...");

        let codebase = ctx
            .compressed_codebase
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No compressed codebase available for validation"))?;

        let rules = ctx
            .generated_rules
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No generated rules available for validation"))?;

        let project_name = ctx
            .config
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project");

        let validation_results = utils::validation::validate_all_formats(
            rules,
            &ctx.config.format,
            &ctx.config.validation,
            codebase,
            project_name,
        )
        .context("Failed to validate generated rules")?;

        let has_failures = validation_results.iter().any(|r| !r.passed);

        if has_failures {
            utils::validation::display_validation_report(&validation_results, ctx.config.quiet);

            if ctx.config.validation.retry_on_failure {
                // Auto-fix: loop over attempts until validation passes or retries exhausted
                tracing::info!("Attempting auto-fix for validation failures...");

                let max_retries = ctx.config.validation.max_retries;
                let mut current_validation = validation_results;
                let mut total_refinement_cost = 0.0;

                for attempt in 1..=max_retries {
                    let rules_mut = ctx.generated_rules.as_mut().ok_or_else(|| {
                        anyhow::anyhow!("No generated rules available for refinement")
                    })?;

                    // Refine each failed format
                    for result in &current_validation {
                        if result.passed {
                            continue;
                        }

                        let current_content = rules_mut
                            .get_format(&result.format)
                            .map(|f| f.content.clone())
                            .unwrap_or_default();

                        let (fixed_content, refinement_result) = generator::refine_invalid_output(
                            &current_content,
                            &result.errors,
                            &result.format,
                            &client,
                            &mut ctx.cost_tracker,
                            attempt,
                            max_retries,
                        )
                        .await
                        .with_context(|| {
                            format!(
                                "Failed to refine {} format (attempt {})",
                                result.format, attempt
                            )
                        })?;

                        total_refinement_cost += refinement_result.total_cost;

                        tracing::info!(
                            "Refinement attempt {} for {}: cost ${:.4}",
                            attempt,
                            result.format,
                            refinement_result.total_cost
                        );

                        // Update the rules with fixed content
                        let rule_type = rules_mut
                            .get_format(&result.format)
                            .and_then(|f| f.rule_type);
                        let updated = generator::FormattedRules {
                            format: result.format.clone(),
                            content: fixed_content,
                            rule_type,
                        };
                        rules_mut.add_format(updated);
                    }

                    // Re-validate after this attempt
                    let rules = ctx.generated_rules.as_ref().ok_or_else(|| {
                        anyhow::anyhow!("No generated rules available for re-validation")
                    })?;

                    let revalidation_results = utils::validation::validate_all_formats(
                        rules,
                        &ctx.config.format,
                        &ctx.config.validation,
                        codebase,
                        project_name,
                    )
                    .context("Failed to re-validate after refinement")?;

                    let still_has_failures = revalidation_results.iter().any(|r| !r.passed);

                    if !still_has_failures {
                        tracing::info!(
                            "All formats passed validation after {} attempt(s) (total refinement cost: ${:.4})",
                            attempt,
                            total_refinement_cost
                        );
                        current_validation = revalidation_results;
                        break;
                    }

                    if attempt == max_retries {
                        // Retries exhausted
                        if !ctx.config.quiet {
                            utils::validation::display_validation_report(
                                &revalidation_results,
                                ctx.config.quiet,
                            );
                            println!();
                            println!(
                                "Auto-fix could not resolve all validation errors after {} attempt(s) (cost: ${:.4}).",
                                max_retries, total_refinement_cost
                            );
                        }

                        let choice =
                            prompt_validation_choice(ctx.config.quiet, ctx.config.no_confirm)
                                .await?;
                        match choice {
                            ValidationChoice::Cancel => {
                                return Err(anyhow::anyhow!(
                                    "Cancelled due to validation failures"
                                ));
                            }
                            ValidationChoice::WriteAnyway => {
                                tracing::warn!("Writing files despite validation failures");
                            }
                        }
                    }

                    current_validation = revalidation_results;
                }

                ctx.validation_results = current_validation;
            } else {
                // No auto-fix: prompt user
                let choice =
                    prompt_validation_choice(ctx.config.quiet, ctx.config.no_confirm).await?;
                match choice {
                    ValidationChoice::Cancel => {
                        return Err(anyhow::anyhow!("Cancelled due to validation failures"));
                    }
                    ValidationChoice::WriteAnyway => {
                        tracing::warn!("Writing files despite validation failures");
                    }
                }
                ctx.validation_results = validation_results;
            }
        } else {
            if !ctx.config.quiet {
                tracing::info!("All formats passed validation");
            }
            ctx.validation_results = validation_results;
        }
    } else {
        tracing::info!("Validation disabled, skipping");
    }

    // Stage 7: Finalizing
    ctx.transition_to(PipelineStage::Finalizing);

    if ctx.config.finalization.enabled {
        tracing::info!("Finalizing rules...");

        let rules = ctx
            .generated_rules
            .take()
            .ok_or_else(|| anyhow::anyhow!("No generated rules available for finalization"))?;

        let (finalized_rules, finalization_result) = utils::finalization::finalize_rules(
            rules,
            &ctx.config.finalization,
            &client,
            &mut ctx.cost_tracker,
            &ctx.config.path,
            &ctx.config.format,
            ctx.config.no_confirm,
            ctx.config.quiet,
        )
        .await
        .context("Failed to finalize rules")?;

        if finalization_result.metadata_injected {
            tracing::debug!("Metadata headers injected");
        }
        if finalization_result.deconflicted {
            tracing::info!("Rules deconflicted with existing rule files");
        }
        if finalization_result.optimizations.formatting_normalized {
            tracing::debug!("Formatting normalized");
        }

        ctx.generated_rules = Some(finalized_rules);
        ctx.finalization_result = Some(finalization_result);

        // Post-finalize smoke validation: re-validate with syntax + schema only
        if ctx.config.validation.enabled {
            let codebase = ctx.compressed_codebase.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No compressed codebase available for post-finalize validation")
            })?;
            let rules = ctx.generated_rules.as_ref().ok_or_else(|| {
                anyhow::anyhow!("No generated rules available for post-finalize validation")
            })?;
            let project_name = ctx
                .config
                .path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project");

            // Use a config that only checks syntax/schema and file paths
            let smoke_config = cli::config::ValidationConfig {
                enabled: true,
                retry_on_failure: false,
                max_retries: 0,
                semantic: cli::config::SemanticValidationConfig {
                    check_file_paths: true,
                    check_contradictions: true,
                    check_consistency: false,
                    check_reality: false,
                },
                format_overrides: cli::config::FormatValidationOverrides::default(),
            };

            let smoke_results = utils::validation::validate_all_formats(
                rules,
                &ctx.config.format,
                &smoke_config,
                codebase,
                project_name,
            )
            .context("Post-finalize smoke validation failed")?;

            let smoke_failures = smoke_results.iter().any(|r| !r.passed);
            if smoke_failures {
                tracing::warn!(
                    "Post-finalize validation detected issues (finalization may have introduced errors)"
                );
                if !ctx.config.quiet {
                    utils::validation::display_validation_report(&smoke_results, false);
                }
            }
        }
    } else {
        tracing::info!("Finalization disabled, skipping");
    }

    // Stage 8: Writing
    ctx.transition_to(PipelineStage::Writing);

    // Get the generated rules for writing
    let rules = ctx
        .generated_rules
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No generated rules available for writing"))?;

    // Determine project name from path
    let project_name = ctx
        .config
        .path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project");

    // Create write options (honor --output for single format)
    let mut output_paths = ctx.config.output_paths.clone();
    if let Some(ref output) = ctx.config.output {
        if ctx.config.format.len() != 1 {
            return Err(anyhow::anyhow!(
                "--output can only be used with a single format (got {} formats)",
                ctx.config.format.len()
            ));
        }
        output_paths.insert(
            ctx.config.format[0].clone(),
            output.to_string_lossy().into_owned(),
        );
    }

    let write_options = output::WriteOptions::new(&ctx.config.path)
        .with_output_paths(output_paths)
        .with_backups(true)
        .with_force(ctx.config.no_confirm);

    if let Some(ref mut pm) = ctx.progress_manager {
        let _ = pm.add_stage(stages::WRITING, ctx.config.format.len() as u64);
        pm.update(stages::WRITING, 0, "preparing...");
    }

    // Write output files (use spawn_blocking to avoid blocking the async runtime)
    let rules_clone = rules.clone();
    let formats_clone = ctx.config.format.clone();
    let project_name_owned = project_name.to_string();
    let results = tokio::task::spawn_blocking(move || {
        output::write_output(
            &rules_clone,
            &formats_clone,
            &project_name_owned,
            &write_options,
        )
    })
    .await
    .context("Write task panicked")?
    .context("Failed to write output files")?;

    if let Some(ref pm) = ctx.progress_manager {
        pm.finish(stages::WRITING, &format!("Wrote {} file(s)", results.len()));
    }

    // Report what was written
    for result in &results {
        if result.is_new {
            tracing::info!("Created {} at {}", result.format, result.path.display());
        } else if result.backup_created {
            tracing::info!(
                "Updated {} at {} (backup: {})",
                result.format,
                result.path.display(),
                result
                    .backup_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            );
        } else {
            tracing::info!("Overwrote {} at {}", result.format, result.path.display());
        }
    }

    if !ctx.config.quiet {
        println!();
        println!("Output Files Written");
        println!("====================");
        for result in &results {
            println!("  {} -> {}", result.format, result.path.display());
        }
    }

    // Stage 9: Reporting
    ctx.transition_to(PipelineStage::Reporting);

    // Calculate metrics for success summary
    let files_analyzed = ctx
        .compressed_codebase
        .as_ref()
        .map(|c| c.metadata.total_files)
        .unwrap_or(0);

    let tokens_processed = ctx
        .cost_tracker
        .as_ref()
        .map(|t| t.summary().total_input_tokens)
        .unwrap_or(0);

    let compression_ratio = ctx.compressed_codebase.as_ref().and_then(|c| {
        if c.metadata.compression_ratio < 1.0 {
            Some(c.metadata.compression_ratio)
        } else {
            None
        }
    });

    let actual_cost = ctx
        .cost_tracker
        .as_ref()
        .map(|t| t.summary().total_cost)
        .unwrap_or(0.0);

    let elapsed = ctx.start_time.elapsed();

    // Display success summary
    display_success_summary(
        &results,
        files_analyzed,
        tokens_processed,
        compression_ratio,
        actual_cost,
        elapsed,
        ctx.config.quiet,
    )?;

    // Stage 10: Cleanup
    ctx.transition_to(PipelineStage::Cleanup);

    // Save state and cleanup temp files
    if let Some(ref cache) = ctx.cache_manager {
        // Build state from context
        let state = utils::state::State {
            version: utils::state::CURRENT_STATE_VERSION.to_string(),
            last_run: Utc::now(),
            user_selections: utils::state::UserSelections::default(),
            output_files: results.iter().map(|r| r.path.clone()).collect(),
            cost_spent: ctx
                .cost_tracker
                .as_ref()
                .map(|t| t.summary().total_cost as f32)
                .unwrap_or(0.0),
            token_count: ctx
                .compressed_codebase
                .as_ref()
                .map(|c| c.metadata.total_original_size)
                .unwrap_or(0),
            compression_ratio: ctx
                .compressed_codebase
                .as_ref()
                .map(|c| c.metadata.compression_ratio)
                .unwrap_or(1.0),
        };

        // Save state
        utils::state::save_state(&state, cache.ruley_dir())?;
        tracing::info!("Saved state to .ruley/state.json");

        // Clean up temp files (preserve state.json)
        let cleanup_result = cache.cleanup_temp_files(true)?;
        if cleanup_result.deleted > 0 {
            tracing::debug!("Cleaned up {} temp files", cleanup_result.deleted);
        }
    }

    // Also call the existing cleanup_temp_files function for TempFileRefs
    cleanup_temp_files(&mut ctx).context("Failed to cleanup temporary files")?;

    // Pipeline Complete
    ctx.transition_to(PipelineStage::Complete);
    tracing::info!("Pipeline completed successfully");

    Ok(())
}

/// Cleanup temporary files created during pipeline execution.
fn cleanup_temp_files(ctx: &mut PipelineContext) -> Result<()> {
    let file_count = ctx.temp_files.len();
    ctx.temp_files
        .clear()
        .context("Failed to remove temporary files")?;

    tracing::debug!("Cleaned up {} temporary files", file_count);
    Ok(())
}

/// Get the appropriate tokenizer for the given provider.
///
/// Returns a boxed tokenizer that matches the provider's tokenization scheme.
///
/// # Arguments
///
/// * `provider` - The LLM provider name (e.g., "anthropic", "openai")
/// * `model` - Optional model name for more precise tokenizer selection
///
/// # Errors
///
/// Returns an error if the tokenizer cannot be created.
#[allow(unused_variables)]
fn get_tokenizer(provider: &str, model: Option<&str>) -> Result<Box<dyn Tokenizer>> {
    match provider.to_lowercase().as_str() {
        #[cfg(feature = "anthropic")]
        "anthropic" => Ok(Box::new(
            AnthropicTokenizer::new().context("Failed to create Anthropic tokenizer")?,
        )),
        #[cfg(feature = "openai")]
        "openai" => {
            let tokenizer_model = model
                .map(TokenizerModel::from_model_name)
                .unwrap_or(TokenizerModel::Gpt4o);
            Ok(Box::new(
                TiktokenTokenizer::new(tokenizer_model)
                    .context("Failed to create OpenAI tokenizer")?,
            ))
        }
        #[cfg(feature = "ollama")]
        "ollama" => {
            // Ollama uses tiktoken-compatible tokenization
            Ok(Box::new(
                TiktokenTokenizer::new(TokenizerModel::Gpt4o)
                    .context("Failed to create Ollama tokenizer")?,
            ))
        }
        #[cfg(feature = "openrouter")]
        "openrouter" => {
            // OpenRouter uses model-specific tokenization, default to cl100k_base
            Ok(Box::new(
                TiktokenTokenizer::new(TokenizerModel::Gpt4o)
                    .context("Failed to create OpenRouter tokenizer")?,
            ))
        }
        // Default to cl100k_base for other providers (reasonable approximation)
        _ => Ok(Box::new(
            TiktokenTokenizer::new(TokenizerModel::Gpt4)
                .context("Failed to create default tokenizer")?,
        )),
    }
}

/// Create an LLM client based on the configuration.
///
/// For OpenRouter, fetches dynamic model pricing from the API so that
/// cost estimation uses actual provider-supplied rates.
///
/// # Arguments
///
/// * `config` - The merged configuration containing provider settings
///
/// # Errors
///
/// Returns an error if the provider is not supported or cannot be initialized.
#[allow(unreachable_code, unused_variables)]
async fn create_llm_client(config: &MergedConfig) -> Result<LLMClient> {
    let provider: Box<dyn LLMProvider> = match config.provider.to_lowercase().as_str() {
        #[cfg(feature = "anthropic")]
        "anthropic" => {
            use llm::providers::anthropic::AnthropicProvider;

            let api_key = std::env::var("ANTHROPIC_API_KEY")
                .context("ANTHROPIC_API_KEY environment variable not set")?;

            let model = config
                .model
                .clone()
                .or_else(|| {
                    config
                        .providers
                        .anthropic
                        .as_ref()
                        .and_then(|p| p.model.clone())
                })
                .unwrap_or_else(|| "claude-sonnet-4-5-20250929".to_string());

            Box::new(
                AnthropicProvider::new(api_key, model)
                    .context("Failed to create Anthropic provider")?,
            )
        }
        #[cfg(feature = "openai")]
        "openai" => {
            use llm::providers::openai::OpenAIProvider;

            let api_key = std::env::var("OPENAI_API_KEY")
                .context("OPENAI_API_KEY environment variable not set")?;

            let model = config
                .model
                .clone()
                .or_else(|| {
                    config
                        .providers
                        .openai
                        .as_ref()
                        .and_then(|p| p.model.clone())
                })
                .unwrap_or_else(|| "gpt-4o".to_string());

            Box::new(
                OpenAIProvider::new(api_key, model).context("Failed to create OpenAI provider")?,
            )
        }
        #[cfg(feature = "ollama")]
        "ollama" => {
            use llm::providers::ollama::OllamaProvider;

            let host = std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string());

            let host_override = config
                .providers
                .ollama
                .as_ref()
                .and_then(|p| p.host.clone());

            let final_host = host_override.unwrap_or(host);

            let model = config
                .model
                .clone()
                .or_else(|| {
                    config
                        .providers
                        .ollama
                        .as_ref()
                        .and_then(|p| p.model.clone())
                })
                .unwrap_or_else(|| "llama3.1:70b".to_string());

            Box::new(
                OllamaProvider::new(final_host, model)
                    .context("Failed to create Ollama provider")?,
            )
        }
        #[cfg(feature = "openrouter")]
        "openrouter" => {
            use llm::providers::openrouter::OpenRouterProvider;

            let api_key = std::env::var("OPENROUTER_API_KEY")
                .context("OPENROUTER_API_KEY environment variable not set")?;

            let model = config
                .model
                .clone()
                .or_else(|| {
                    config
                        .providers
                        .openrouter
                        .as_ref()
                        .and_then(|p| p.model.clone())
                })
                .unwrap_or_else(|| "anthropic/claude-3.5-sonnet".to_string());

            let provider = OpenRouterProvider::new(api_key, model)
                .context("Failed to create OpenRouter provider")?;

            // Fetch dynamic pricing from OpenRouter's models API
            if let Err(e) = provider.fetch_model_pricing().await {
                tracing::warn!("Failed to fetch OpenRouter model pricing: {e}");
            }

            Box::new(provider)
        }
        provider => {
            return Err(anyhow::anyhow!(
                "Unsupported provider '{}'. Supported providers: anthropic, openai, ollama, openrouter",
                provider
            ));
        }
    };

    Ok(LLMClient::new(provider))
}

/// Get the context limit for the given provider and model.
///
/// Returns a reasonable default context limit for the provider.
fn get_context_limit(provider: &str, _model: Option<&str>) -> usize {
    match provider.to_lowercase().as_str() {
        "anthropic" => 200_000,  // Claude models support 200K context
        "openai" => 128_000,     // GPT-4o supports 128K context
        "ollama" => 100_000,     // Conservative default for local models
        "openrouter" => 128_000, // Varies by model, use conservative default
        _ => 100_000,            // Conservative default
    }
}

/// Get default pricing for a provider when API key is not available.
///
/// Used in dry-run mode to show cost estimates without requiring credentials.
/// These are approximate prices and may not reflect current API billing.
/// For accurate estimates, run without `--dry-run` with valid API credentials.
fn get_default_pricing(provider: &str) -> llm::provider::Pricing {
    use llm::provider::Pricing;

    match provider.to_lowercase().as_str() {
        "anthropic" => Pricing {
            input_per_1k: 0.003, // Claude Sonnet pricing
            output_per_1k: 0.015,
        },
        "openai" => Pricing {
            input_per_1k: 0.0025, // GPT-4o pricing
            output_per_1k: 0.01,
        },
        _ => Pricing {
            input_per_1k: 0.003, // Conservative default
            output_per_1k: 0.015,
        },
    }
}

/// User choices when validation fails.
enum ValidationChoice {
    /// Cancel the pipeline
    Cancel,
    /// Write files despite validation errors
    WriteAnyway,
}

/// Prompt the user for a choice when validation fails.
///
/// # Arguments
///
/// * `quiet` - Whether to suppress output
/// * `no_confirm` - Whether to skip confirmation (auto-proceed with write)
async fn prompt_validation_choice(quiet: bool, no_confirm: bool) -> Result<ValidationChoice> {
    if no_confirm || quiet {
        return Ok(ValidationChoice::WriteAnyway);
    }

    println!();
    println!("Options:");
    println!("  (c) Cancel - Exit without writing files");
    println!("  (w) Write anyway - Write files despite validation errors");
    println!();

    let mut stdout = tokio::io::stdout();
    stdout
        .write_all(b"Choice [c/w]: ")
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
        "w" | "write" => Ok(ValidationChoice::WriteAnyway),
        _ => Ok(ValidationChoice::Cancel),
    }
}

/// Perform LLM analysis on the codebase.
///
/// Handles both single-chunk and multi-chunk analysis paths.
///
/// # Arguments
///
/// * `ctx` - The pipeline context with compressed codebase
/// * `client` - The LLM client to use
/// * `chunks` - The chunks to analyze
/// * `prompt` - The analysis prompt
///
/// # Returns
///
/// The analysis result as a string.
async fn perform_analysis(
    ctx: &mut PipelineContext,
    client: &LLMClient,
    chunks: Vec<Chunk>,
    prompt: &str,
) -> Result<String> {
    let num_chunks = chunks.len();

    if num_chunks == 1 {
        tracing::info!("Analyzing codebase (single chunk, no merge required)");
        let result = llm::analysis::analyze_chunked_with_results(chunks, prompt, client)
            .await
            .context("Failed to analyze codebase")?;

        // Track the operation cost using provider-reported token counts
        if let Some(ref mut tracker) = ctx.cost_tracker {
            let total_prompt: usize = result.chunk_results.iter().map(|r| r.prompt_tokens).sum();
            let total_completion: usize = result
                .chunk_results
                .iter()
                .map(|r| r.completion_tokens)
                .sum();
            tracker.add_operation("analysis", total_prompt, total_completion);
        }

        Ok(result.merged_analysis)
    } else {
        tracing::info!(
            "Analyzing codebase in {} chunks with merge step",
            num_chunks
        );
        let result = llm::analysis::analyze_chunked_with_results(chunks, prompt, client)
            .await
            .context("Failed to analyze chunked codebase")?;

        // Track cost using provider-reported token counts (includes all chunks + merge)
        if let Some(ref mut tracker) = ctx.cost_tracker {
            let total_prompt: usize = result.chunk_results.iter().map(|r| r.prompt_tokens).sum();
            let total_completion: usize = result
                .chunk_results
                .iter()
                .map(|r| r.completion_tokens)
                .sum();
            // Add merge step tokens if present
            let merge_prompt = result.merge_prompt_tokens;
            let merge_completion = result.merge_completion_tokens;
            tracker.add_operation(
                "chunked_analysis",
                total_prompt + merge_prompt,
                total_completion + merge_completion,
            );
        }

        tracing::info!("Chunk analysis and merge completed");

        Ok(result.merged_analysis)
    }
}
