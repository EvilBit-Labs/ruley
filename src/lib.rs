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
use llm::chunker::{Chunk, ChunkConfig};
use llm::client::LLMClient;
use llm::cost::{CostCalculator, CostTracker};
use llm::provider::LLMProvider;
#[cfg(feature = "anthropic")]
use llm::tokenizer::AnthropicTokenizer;
use llm::tokenizer::{TiktokenTokenizer, Tokenizer, TokenizerModel};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

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

    /// Returns the number of tracked temporary files
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns true if there are no tracked temporary files
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
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
    /// Cost tracking for LLM operations
    pub cost_tracker: Option<CostTracker>,
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
            compressed_codebase: None,
            analysis_result: None,
            cost_tracker: None,
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
        if !ctx.config.quiet {
            display_dry_run_config(&ctx.config);
        }
        return Ok(());
    }

    // Stage 2: Scanning
    ctx.transition_to(PipelineStage::Scanning);
    let file_entries = if let Some(_path) = &ctx.config.repomix_file {
        tracing::info!("Repomix file mode active, skipping scanning.");
        vec![] // Empty list, scanning is skipped
    } else {
        let entries = packer::scan_files(&ctx.config.path, &ctx.config)
            .await
            .context("Failed to scan repository files")?;
        tracing::info!("Discovered {} files", entries.len());
        entries
    };

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
    let compressed_codebase = if let Some(path) = &ctx.config.repomix_file {
        packer::parse_repomix(path.as_path())
            .await
            .context("Failed to parse repomix file")?
    } else {
        packer::compress_codebase(file_entries, &ctx.config)
            .await
            .context("Failed to compress codebase")?
    };

    ctx.compressed_codebase = Some(compressed_codebase);

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

    // Create LLM client
    let client = create_llm_client(&ctx.config)?;

    // Initialize cost tracker
    let pricing = client.pricing();
    let calculator = CostCalculator::new(pricing);
    ctx.cost_tracker = Some(CostTracker::new(calculator.clone()));

    // Build the analysis prompt (needed for accurate cost estimation)
    let prompt = build_analysis_prompt(&ctx.config.rule_type, ctx.config.description.as_deref());
    let prompt_tokens = tokenizer.count_tokens(&prompt);

    // Estimate cost
    // For multi-chunk, estimate includes: N chunk analyses + 1 merge
    let estimated_output_tokens = chunks.len() * 4096; // Approximate output per chunk
    let total_chunk_tokens: usize = chunks.iter().map(|c| c.token_count).sum();
    // Include prompt tokens: for multi-chunk, prompt is sent with each chunk
    let estimated_input_tokens = if chunks.len() == 1 {
        total_chunk_tokens + prompt_tokens
    } else {
        total_chunk_tokens + (prompt_tokens * chunks.len())
    };
    let cost_estimate = calculator.estimate_cost(estimated_input_tokens, estimated_output_tokens);

    // Show cost estimation and confirm (unless --no-confirm)
    if !ctx.config.no_confirm {
        let confirmed = confirm_cost(&cost_estimate, chunks.len(), ctx.config.quiet).await?;
        if !confirmed {
            tracing::info!("User cancelled operation");
            return Ok(());
        }
    } else if !ctx.config.quiet {
        println!(
            "Estimated cost: ${:.4} ({} input tokens, ~{} output tokens)",
            cost_estimate.total_cost, cost_estimate.input_tokens, cost_estimate.output_tokens
        );
    }

    // Perform the analysis
    let analysis_result = perform_analysis(&mut ctx, &client, chunks, &prompt).await?;

    tracing::info!("Analysis complete ({} characters)", analysis_result.len());

    // Store the analysis result for the next stage
    ctx.analysis_result = Some(analysis_result);

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
    let file_count = ctx.temp_files.len();
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
        // Default to cl100k_base for other providers (reasonable approximation)
        _ => Ok(Box::new(
            TiktokenTokenizer::new(TokenizerModel::Gpt4)
                .context("Failed to create default tokenizer")?,
        )),
    }
}

/// Create an LLM client based on the configuration.
///
/// # Arguments
///
/// * `config` - The merged configuration containing provider settings
///
/// # Errors
///
/// Returns an error if the provider is not supported or cannot be initialized.
#[allow(unreachable_code, unused_variables)]
fn create_llm_client(config: &MergedConfig) -> Result<LLMClient> {
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
        provider => {
            return Err(anyhow::anyhow!(
                "Unsupported provider '{}'. Supported providers: anthropic, openai",
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
        "anthropic" => 200_000, // Claude models support 200K context
        "openai" => 128_000,    // GPT-4o supports 128K context
        _ => 100_000,           // Conservative default
    }
}

/// Display cost estimation and prompt for confirmation.
///
/// # Arguments
///
/// * `estimate` - The cost estimate to display
/// * `num_chunks` - Number of chunks that will be processed
/// * `quiet` - Whether to suppress output
///
/// # Returns
///
/// `true` if the user confirms, `false` otherwise.
async fn confirm_cost(
    estimate: &llm::cost::CostEstimate,
    num_chunks: usize,
    quiet: bool,
) -> Result<bool> {
    if quiet {
        return Ok(true);
    }

    println!();
    println!("Cost Estimation");
    println!("===============");
    println!(
        "Input tokens:  {:>10} (${:.4})",
        estimate.input_tokens, estimate.input_cost
    );
    println!(
        "Output tokens: {:>10} (${:.4}) [estimated]",
        estimate.output_tokens, estimate.output_cost
    );
    println!(
        "Total tokens:  {:>10}",
        estimate.input_tokens + estimate.output_tokens
    );
    println!("----------------------------");
    println!("Estimated cost: ${:.4}", estimate.total_cost);
    if num_chunks > 1 {
        println!("Chunks to process: {}", num_chunks);
    }
    println!();

    let mut stdout = tokio::io::stdout();
    stdout
        .write_all(b"Proceed with LLM analysis? [y/N] ")
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

    let confirmed = matches!(input.trim().to_lowercase().as_str(), "y" | "yes");
    Ok(confirmed)
}

/// Build the analysis prompt from the rule type and optional description.
fn build_analysis_prompt(rule_type: &str, description: Option<&str>) -> String {
    let base = generator::prompts::base_prompt();

    let mut prompt = base.to_string();

    if let Some(desc) = description {
        prompt.push_str("\n\nAdditional context from user:\n");
        prompt.push_str(desc);
    }

    prompt.push_str(&format!(
        "\n\nGenerate rules appropriate for a '{}' rule type.",
        rule_type
    ));

    prompt
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

    // Calculate total input tokens from all chunks (includes actual codebase content)
    let total_input_tokens: usize = chunks.iter().map(|c| c.token_count).sum();

    if num_chunks == 1 {
        tracing::info!("Analyzing codebase (single chunk, no merge required)");
        let result = llm::analysis::analyze_chunked(chunks, prompt, client)
            .await
            .context("Failed to analyze codebase")?;

        // Track the operation cost
        if let Some(ref mut tracker) = ctx.cost_tracker {
            let tokenizer = get_tokenizer(&ctx.config.provider, ctx.config.model.as_deref())?;
            let output_tokens = tokenizer.count_tokens(&result);
            // Include prompt tokens plus the codebase content tokens
            let prompt_tokens = tokenizer.count_tokens(prompt);
            tracker.add_operation(
                "analysis",
                total_input_tokens + prompt_tokens,
                output_tokens,
            );
        }

        Ok(result)
    } else {
        tracing::info!(
            "Analyzing codebase in {} chunks with merge step",
            num_chunks
        );
        let result = llm::analysis::analyze_chunked(chunks, prompt, client)
            .await
            .context("Failed to analyze chunked codebase")?;

        // Track cost for chunked analysis
        // Each chunk is analyzed separately, then merged
        if let Some(ref mut tracker) = ctx.cost_tracker {
            let tokenizer = get_tokenizer(&ctx.config.provider, ctx.config.model.as_deref())?;
            let output_tokens = tokenizer.count_tokens(&result);
            let prompt_tokens = tokenizer.count_tokens(prompt);
            // Total input = all chunk tokens + prompt overhead per chunk + merge input
            // Simplification: count total input + prompt per chunk
            let total_with_prompts = total_input_tokens + (prompt_tokens * num_chunks);
            tracker.add_operation("chunked_analysis", total_with_prompts, output_tokens);
        }

        tracing::info!("Chunk analysis and merge completed");

        Ok(result)
    }
}
