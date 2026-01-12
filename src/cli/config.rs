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
//! ```rust,no_run
//! use ruley::cli::{args, config};
//!
//! let (args, presence) = args::parse()?;
//! let file_config = config::load(&args)?;
//! let merged = config::merge_config(&args, file_config, &presence);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use crate::cli::args::{Args, ArgsPresence};
use crate::utils::error::RuleyError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Configuration for content chunking.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChunkingConfig {
    /// Maximum tokens per chunk
    pub chunk_size: Option<usize>,
    /// Token overlap between chunks (for future use)
    pub overlap: Option<usize>,
}

/// Root configuration structure loaded from config files.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

/// Discover configuration file paths in order of precedence.
fn discover_config_paths(explicit_path: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut canonical_paths = Vec::new();

    // Helper to add a path if it doesn't already exist (by canonical path)
    let mut add_if_unique = |path: PathBuf| {
        if !path.exists() {
            return;
        }
        // Use canonical path for duplicate detection to handle symlinks and relative paths
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
        if !canonical_paths.contains(&canonical) {
            canonical_paths.push(canonical);
            paths.push(path);
        }
    };

    // User config (lowest precedence)
    if let Some(user_config) = get_user_config_path() {
        add_if_unique(user_config);
    }

    // Git root config
    if let Some(git_root) = find_git_root() {
        let git_config = git_root.join("ruley.toml");
        add_if_unique(git_config);
    }

    // Current directory config
    let current_dir_config = PathBuf::from("ruley.toml");
    add_if_unique(current_dir_config);

    // Explicit --config path (highest precedence)
    add_if_unique(explicit_path.to_path_buf());

    paths
}

fn find_git_root() -> Option<PathBuf> {
    match git2::Repository::discover(".") {
        Ok(repo) => repo.workdir().map(|p| p.to_path_buf()),
        Err(e) => {
            tracing::debug!("Failed to discover git repository: {}", e);
            None
        }
    }
}

fn get_user_config_path() -> Option<PathBuf> {
    dirs::config_dir()
        .map(|config_dir| config_dir.join("ruley").join("config.toml"))
        .filter(|path| path.exists())
}

/// Load configuration from discovered config files and environment variables.
pub fn load(args: &Args) -> Result<Config, RuleyError> {
    let mut builder = config::Config::builder();

    for config_path in discover_config_paths(&args.config) {
        builder = builder.add_source(config::File::from(config_path));
    }

    builder = builder.add_source(
        config::Environment::with_prefix("RULEY")
            .separator("_")
            .try_parsing(true),
    );

    let settings = builder
        .build()
        .map_err(|e| RuleyError::Config(format!("Failed to build configuration: {e}")))?;

    settings
        .try_deserialize()
        .map_err(|e| RuleyError::Config(format!("Failed to deserialize configuration: {e}")))
}

/// Merge CLI arguments with loaded configuration to create final merged config.
/// CLI arguments have highest precedence when explicitly provided, followed by
/// environment variables (already merged into config), then config files.
///
/// The `presence` parameter indicates which CLI arguments were explicitly provided
/// on the command line, allowing us to distinguish between CLI defaults and user intent.
pub fn merge_config(args: &Args, config: Config, presence: &ArgsPresence) -> crate::MergedConfig {
    // Provider: CLI explicit > config (config always has a value due to default)
    let provider = if presence.provider {
        args.provider.clone()
    } else {
        config.general.provider.clone()
    };

    // Format: CLI explicit > general.format > output.formats > CLI default
    let format: Vec<String> = if presence.format {
        args.format.iter().map(|f| f.as_str().to_string()).collect()
    } else {
        first_non_empty(&[&config.general.format, &config.output.formats])
            .unwrap_or_else(|| args.format.iter().map(|f| f.as_str().to_string()).collect())
    };

    // Rule type: CLI explicit > config (config always has a value due to default)
    let rule_type = if presence.rule_type {
        args.rule_type.clone()
    } else {
        config.general.rule_type.clone()
    };

    // Compress: CLI explicit > config
    let compress = if presence.compress {
        args.compress
    } else {
        config.general.compress
    };

    // Chunk size: CLI explicit > general.chunk_size (if non-default) > chunking.chunk_size > CLI default
    let chunk_size = if presence.chunk_size {
        args.chunk_size
    } else if config.general.chunk_size != default_chunk_size() {
        config.general.chunk_size
    } else {
        config
            .chunking
            .as_ref()
            .and_then(|c| c.chunk_size)
            .unwrap_or(args.chunk_size)
    };

    // No confirm: CLI explicit > config
    let no_confirm = if presence.no_confirm {
        args.no_confirm
    } else {
        config.general.no_confirm
    };

    // Include/exclude: CLI non-empty > config
    let include = if args.include.is_empty() {
        config.include.patterns
    } else {
        args.include.clone()
    };

    let exclude = if args.exclude.is_empty() {
        config.exclude.patterns
    } else {
        args.exclude.clone()
    };

    crate::MergedConfig {
        provider,
        model: args.model.clone().or(config.general.model),
        format,
        output: args.output.clone(),
        repomix_file: args.repomix_file.clone(),
        path: args.path.clone(),
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
        output_paths: config.output.paths,
        providers: config.providers,
    }
}

/// Returns the first non-empty vector from the slice, or None if all are empty.
fn first_non_empty(vecs: &[&Vec<String>]) -> Option<Vec<String>> {
    vecs.iter().find(|v| !v.is_empty()).map(|v| (*v).clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::{Args, ArgsPresence, OutputFormat};
    use std::collections::HashMap;

    // Note: Discovery tests that change current directory were removed because
    // they are flaky in parallel test execution. The discovery logic is tested
    // through integration tests instead.

    mod merging {
        use super::*;

        fn create_test_config() -> Config {
            Config {
                general: GeneralConfig {
                    provider: "openai".to_string(),
                    model: Some("gpt-4o".to_string()),
                    format: vec!["cursor".to_string(), "claude".to_string()],
                    compress: true,
                    chunk_size: 50000,
                    no_confirm: false,
                    rule_type: "manual".to_string(),
                },
                output: OutputConfig {
                    formats: vec!["copilot".to_string()],
                    paths: {
                        let mut map = HashMap::new();
                        map.insert("cursor".to_string(), ".cursor/rules/rules.mdc".to_string());
                        map
                    },
                },
                include: IncludeConfig {
                    patterns: vec!["**/*.rs".to_string()],
                },
                exclude: ExcludeConfig {
                    patterns: vec!["**/target/**".to_string()],
                },
                providers: ProvidersConfig::default(),
                chunking: Some(ChunkingConfig {
                    chunk_size: Some(75000),
                    overlap: None,
                }),
            }
        }

        fn create_test_args() -> Args {
            Args {
                path: PathBuf::from("."),
                provider: "anthropic".to_string(),
                model: Some("claude-sonnet-4".to_string()),
                output: None,
                repomix_file: None,
                format: vec![OutputFormat::Copilot, OutputFormat::Windsurf],
                description: None,
                rule_type: "agent".to_string(),
                config: PathBuf::from("ruley.toml"),
                include: vec!["**/*.ts".to_string()],
                exclude: vec!["**/node_modules/**".to_string()],
                compress: false,
                chunk_size: 100000,
                no_confirm: true,
                dry_run: false,
                verbose: 0,
                quiet: false,
            }
        }

        fn create_test_presence() -> ArgsPresence {
            ArgsPresence {
                provider: true,
                format: true,
                rule_type: true,
                compress: true,
                chunk_size: true,
                no_confirm: true,
            }
        }

        #[test]
        fn test_merge_config_cli_explicit() {
            let config = create_test_config();
            let args = create_test_args();
            let presence = create_test_presence();

            let merged = merge_config(&args, config, &presence);

            // CLI values should win when explicitly provided
            assert_eq!(merged.provider, "anthropic");
            assert_eq!(merged.format, vec!["copilot", "windsurf"]);
            assert_eq!(merged.rule_type, "agent");
            assert!(!merged.compress);
            assert_eq!(merged.chunk_size, 100000);
            assert!(merged.no_confirm);
        }

        #[test]
        fn test_merge_config_cli_defaults() {
            let config = create_test_config();
            let args = create_test_args();
            let presence = ArgsPresence::default(); // No CLI flags explicitly provided

            let merged = merge_config(&args, config, &presence);

            // Config file values should be used when CLI uses defaults
            assert_eq!(merged.provider, "openai");
            assert_eq!(merged.format, vec!["cursor", "claude"]);
            assert_eq!(merged.rule_type, "manual");
            assert!(merged.compress);
            assert_eq!(merged.chunk_size, 50000);
            assert!(!merged.no_confirm);
        }

        #[test]
        fn test_merge_config_format_precedence() {
            let config = Config {
                general: GeneralConfig {
                    format: vec!["cursor".to_string()],
                    ..Default::default()
                },
                output: OutputConfig {
                    formats: vec!["claude".to_string()],
                    ..Default::default()
                },
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let args = Args {
                format: vec![OutputFormat::Copilot],
                ..create_test_args()
            };

            // When CLI format is explicitly provided
            let presence = ArgsPresence {
                format: true,
                ..Default::default()
            };
            let merged = merge_config(&args, config.clone(), &presence);
            assert_eq!(merged.format, vec!["copilot"]);

            // When CLI format is not provided, use general.format
            let presence = ArgsPresence {
                format: false,
                ..Default::default()
            };
            let merged = merge_config(&args, config.clone(), &presence);
            assert_eq!(merged.format, vec!["cursor"]);
        }

        #[test]
        fn test_merge_config_chunk_size_precedence() {
            let config = Config {
                general: GeneralConfig {
                    chunk_size: 50000,
                    ..Default::default()
                },
                output: OutputConfig::default(),
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: Some(ChunkingConfig {
                    chunk_size: Some(75000),
                    overlap: None,
                }),
            };

            let args = Args {
                chunk_size: 100000,
                ..create_test_args()
            };

            // CLI chunk_size explicitly provided
            let presence = ArgsPresence {
                chunk_size: true,
                ..Default::default()
            };
            let merged = merge_config(&args, config.clone(), &presence);
            assert_eq!(merged.chunk_size, 100000);

            // CLI chunk_size not provided, use general.chunk_size
            let presence = ArgsPresence {
                chunk_size: false,
                ..Default::default()
            };
            let merged = merge_config(&args, config.clone(), &presence);
            assert_eq!(merged.chunk_size, 50000);

            // If general.chunk_size is default, use chunking.chunk_size
            let config = Config {
                general: GeneralConfig {
                    chunk_size: default_chunk_size(), // default value
                    ..Default::default()
                },
                output: OutputConfig::default(),
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: Some(ChunkingConfig {
                    chunk_size: Some(75000),
                    overlap: None,
                }),
            };
            let presence = ArgsPresence {
                chunk_size: false,
                ..Default::default()
            };
            let merged = merge_config(&args, config, &presence);
            assert_eq!(merged.chunk_size, 75000);
        }

        #[test]
        fn test_merge_config_include_exclude() {
            let config = Config {
                general: GeneralConfig::default(),
                output: OutputConfig::default(),
                include: IncludeConfig {
                    patterns: vec!["**/*.rs".to_string()],
                },
                exclude: ExcludeConfig {
                    patterns: vec!["**/target/**".to_string()],
                },
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let args = Args {
                include: vec!["**/*.ts".to_string()],
                exclude: vec!["**/node_modules/**".to_string()],
                ..create_test_args()
            };

            let merged = merge_config(&args, config, &ArgsPresence::default());

            // CLI args should override config file
            assert_eq!(merged.include, vec!["**/*.ts"]);
            assert_eq!(merged.exclude, vec!["**/node_modules/**"]);
        }

        #[test]
        fn test_merge_config_include_exclude_empty_cli() {
            let config = Config {
                general: GeneralConfig::default(),
                output: OutputConfig::default(),
                include: IncludeConfig {
                    patterns: vec!["**/*.rs".to_string()],
                },
                exclude: ExcludeConfig {
                    patterns: vec!["**/target/**".to_string()],
                },
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let args = Args {
                include: vec![],
                exclude: vec![],
                ..create_test_args()
            };

            let merged = merge_config(&args, config, &ArgsPresence::default());

            // Config file values should be used when CLI is empty
            assert_eq!(merged.include, vec!["**/*.rs"]);
            assert_eq!(merged.exclude, vec!["**/target/**"]);
        }

        #[test]
        fn test_merge_config_output_paths() {
            let config = Config {
                general: GeneralConfig::default(),
                output: OutputConfig {
                    paths: {
                        let mut map = HashMap::new();
                        map.insert("cursor".to_string(), ".cursor/rules/rules.mdc".to_string());
                        map.insert("claude".to_string(), "CLAUDE.md".to_string());
                        map
                    },
                    formats: vec![],
                },
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let merged = merge_config(&create_test_args(), config, &ArgsPresence::default());

            assert_eq!(merged.output_paths.len(), 2);
            assert_eq!(
                merged.output_paths.get("cursor"),
                Some(&".cursor/rules/rules.mdc".to_string())
            );
            assert_eq!(
                merged.output_paths.get("claude"),
                Some(&"CLAUDE.md".to_string())
            );
        }

        #[test]
        fn test_merge_config_model_precedence() {
            let config = Config {
                general: GeneralConfig {
                    model: Some("gpt-4o".to_string()),
                    ..Default::default()
                },
                output: OutputConfig::default(),
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let args = Args {
                model: Some("claude-sonnet-4".to_string()),
                ..create_test_args()
            };

            let merged = merge_config(&args, config, &ArgsPresence::default());

            // CLI model should override config model
            assert_eq!(merged.model, Some("claude-sonnet-4".to_string()));
        }

        #[test]
        fn test_merge_config_model_from_config() {
            let config = Config {
                general: GeneralConfig {
                    model: Some("gpt-4o".to_string()),
                    ..Default::default()
                },
                output: OutputConfig::default(),
                include: IncludeConfig::default(),
                exclude: ExcludeConfig::default(),
                providers: ProvidersConfig::default(),
                chunking: None,
            };

            let args = Args {
                model: None,
                ..create_test_args()
            };

            let merged = merge_config(&args, config, &ArgsPresence::default());

            // Config model should be used when CLI model is None
            assert_eq!(merged.model, Some("gpt-4o".to_string()));
        }
    }
}
