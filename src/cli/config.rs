//! Configuration management using the `config` crate for hierarchical discovery and merging.
//!
//! ## Configuration Sources (in precedence order, highest to lowest):
//! 1. **CLI flags** - Highest precedence (passed separately to application logic)
//! 2. **Environment variables** - Middle precedence (via `RULEY_*` prefix)
//! 3. **Config files** - Lowest precedence
//!
//! ## Config File Discovery (in merge order, later overrides earlier):
//! The `config` crate loads and merges configuration files in this order:
//! 1. `~/.config/ruley/config.toml` (user config directory - lowest precedence)
//! 2. `ruley.toml` in git repository root (walking up from current directory)
//! 3. `./ruley.toml` in current directory (highest precedence among fallback files)
//! 4. Explicit `--config` path (if provided and exists - overrides all above)
//!
//! ## Usage:
//! ```rust
//! use ruley::cli::{args, config};
//!
//! let (args, presence) = args::parse();
//! let file_config = config::load(&args)?;
//! let merged = config::merge_config(&args, file_config, &presence);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::cli::args::{Args, ArgsPresence};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for content chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Maximum tokens per chunk
    pub chunk_size: Option<usize>,
    /// Token overlap between chunks (for future use)
    pub overlap: Option<usize>,
}

/// Root configuration structure loaded from config files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub include: IncludeConfig,
    #[serde(default)]
    pub exclude: ExcludeConfig,
    #[serde(default)]
    pub providers: ProvidersConfig,
    pub chunking: Option<ChunkingConfig>,
}

/// General application settings.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GeneralConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub model: Option<String>,
    #[serde(default)]
    pub format: Vec<String>,
    #[serde(default)]
    pub compress: bool,
    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
    #[serde(default)]
    pub no_confirm: bool,
    #[serde(default = "default_rule_type")]
    pub rule_type: String,
}

fn default_chunk_size() -> usize {
    100000
}

fn default_provider() -> String {
    "anthropic".to_string()
}

fn default_rule_type() -> String {
    "agent".to_string()
}

/// Output format and path configuration.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub paths: std::collections::HashMap<String, String>,
}

/// File inclusion patterns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IncludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// File exclusion patterns.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcludeConfig {
    #[serde(default)]
    pub patterns: Vec<String>,
}

/// LLM provider configurations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProvidersConfig {
    pub anthropic: Option<ProviderConfig>,
    pub openai: Option<ProviderConfig>,
    pub ollama: Option<OllamaConfig>,
    pub openrouter: Option<ProviderConfig>,
}

/// Configuration for a single LLM provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
}

/// Ollama-specific configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: Option<String>,
    pub model: Option<String>,
}

fn discover_config_paths(explicit_path: &PathBuf) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // User config (lowest precedence)
    if let Some(user_config) = get_user_config_path() {
        paths.push(user_config);
    }

    // Git root config
    if let Some(git_root) = find_git_root() {
        let git_config = git_root.join("ruley.toml");
        if git_config.exists() {
            paths.push(git_config);
        }
    }

    // Current directory config
    let current_dir_config = PathBuf::from("ruley.toml");
    if current_dir_config.exists() {
        paths.push(current_dir_config);
    }

    // Explicit --config path (highest precedence)
    if explicit_path != &PathBuf::from("ruley.toml") && explicit_path.exists() {
        paths.push(explicit_path.clone());
    }

    paths
}

fn find_git_root() -> Option<PathBuf> {
    git2::Repository::discover(".")
        .ok()
        .and_then(|repo| repo.workdir().map(|p| p.to_path_buf()))
}

fn get_user_config_path() -> Option<PathBuf> {
    dirs::config_dir()
        .map(|config_dir| config_dir.join("ruley").join("config.toml"))
        .filter(|path| path.exists())
}

/// Load configuration from discovered config files and environment variables.
pub fn load(args: &Args) -> Result<Config> {
    let mut builder = config::Config::builder();

    for config_path in discover_config_paths(&args.config) {
        builder = builder.add_source(config::File::from(config_path));
    }

    builder = builder.add_source(
        config::Environment::with_prefix("RULEY")
            .separator("_")
            .try_parsing(true),
    );

    let settings = builder.build().context("Failed to build configuration")?;

    settings
        .try_deserialize()
        .context("Failed to deserialize configuration")
}

/// Merge CLI arguments with loaded configuration to create final merged config.
/// CLI arguments have highest precedence when explicitly provided, followed by
/// environment variables (already merged into config), then config files.
///
/// The `presence` parameter indicates which CLI arguments were explicitly provided
/// on the command line, allowing us to distinguish between CLI defaults and user intent.
pub fn merge_config(args: &Args, config: Config, presence: &ArgsPresence) -> crate::MergedConfig {
    // Provider: use CLI if explicitly provided, otherwise fall back to config
    let provider = if presence.provider {
        args.provider.clone()
    } else if !config.general.provider.is_empty() {
        config.general.provider.clone()
    } else {
        args.provider.clone() // Fall back to CLI default
    };

    // Format: use CLI if explicitly provided, otherwise fall back to config
    let format: Vec<String> = if presence.format {
        args.format.iter().map(|f| f.as_str().to_string()).collect()
    } else if !config.general.format.is_empty() {
        config.general.format.clone()
    } else if !config.output.formats.is_empty() {
        config.output.formats.clone()
    } else {
        // Fall back to CLI default
        args.format.iter().map(|f| f.as_str().to_string()).collect()
    };

    // Rule type: use CLI if explicitly provided, otherwise fall back to config
    let rule_type = if presence.rule_type {
        args.rule_type.clone()
    } else if !config.general.rule_type.is_empty() {
        config.general.rule_type.clone()
    } else {
        args.rule_type.clone() // Fall back to CLI default
    };

    // Compress: use CLI if explicitly provided, otherwise fall back to config
    let compress = if presence.compress {
        args.compress
    } else {
        config.general.compress
    };

    // Chunk size: use CLI if explicitly provided, otherwise fall back to config
    let chunk_size = if presence.chunk_size {
        args.chunk_size
    } else if config.general.chunk_size != default_chunk_size() {
        config.general.chunk_size
    } else if let Some(ref chunking) = config.chunking {
        chunking.chunk_size.unwrap_or(args.chunk_size)
    } else {
        args.chunk_size // Fall back to CLI default
    };

    // No confirm: use CLI if explicitly provided, otherwise fall back to config
    let no_confirm = if presence.no_confirm {
        args.no_confirm
    } else {
        config.general.no_confirm
    };

    // Path: always use CLI value (no config file equivalent)
    let path = args.path.clone();

    // Output: always use CLI value (no config file equivalent)
    let output = args.output.clone();

    // Repomix file: always use CLI value (no config file equivalent)
    let repomix_file = args.repomix_file.clone();

    // Merge include patterns: CLI args take precedence, then config file
    let include = if args.include.is_empty() {
        config.include.patterns
    } else {
        args.include.clone()
    };

    // Merge exclude patterns: CLI args take precedence, then config file
    let exclude = if args.exclude.is_empty() {
        config.exclude.patterns
    } else {
        args.exclude.clone()
    };

    // Create output_paths HashMap from config.output.paths
    let output_paths = config.output.paths;

    crate::MergedConfig {
        provider,
        model: args.model.clone().or(config.general.model),
        format,
        output,
        repomix_file,
        path,
        description: args.description.clone(),
        rule_type,
        include,
        exclude,
        compress,
        chunk_size,
        no_confirm,
        dry_run: args.dry_run,
        verbose: args.verbose,
        quiet: args.quiet,
        chunking: config.chunking,
        output_paths,
        providers: config.providers,
    }
}
