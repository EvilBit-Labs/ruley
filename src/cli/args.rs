use crate::generator::rules::RuleType;
use clap::{ArgMatches, CommandFactory, FromArgMatches, Parser, ValueEnum};
use std::path::PathBuf;

/// Supported LLM provider names for CLI validation.
const SUPPORTED_PROVIDERS: [&str; 4] = ["anthropic", "openai", "ollama", "openrouter"];

/// Supported conflict resolution strategies for CLI validation.
const SUPPORTED_CONFLICT_STRATEGIES: [&str; 4] = ["prompt", "overwrite", "skip", "smart-merge"];

/// Supported output formats for generated rules.
/// Each format corresponds to a specific AI IDE tool or configuration style.
#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Cursor IDE rules format (.mdc files)
    Cursor,
    /// Claude Code format (CLAUDE.md)
    Claude,
    /// GitHub Copilot format
    Copilot,
    /// Windsurf IDE format
    Windsurf,
    /// Aider format
    Aider,
    /// Generic markdown format
    Generic,
    /// JSON format for programmatic use
    Json,
}

/// Tracks which CLI arguments were explicitly provided by the user.
/// Used to determine whether to use CLI defaults or fall back to config file values.
#[derive(Debug, Clone, Default)]
pub struct ArgsPresence {
    /// Whether --provider was explicitly provided
    pub provider: bool,
    /// Whether --format was explicitly provided
    pub format: bool,
    /// Whether --rule-type was explicitly provided
    pub rule_type: bool,
    /// Whether --compress was explicitly provided
    pub compress: bool,
    /// Whether --chunk-size was explicitly provided
    pub chunk_size: bool,
    /// Whether --no-confirm was explicitly provided
    pub no_confirm: bool,
    /// Whether --retry-on-validation-failure was explicitly provided
    pub retry_on_validation_failure: bool,
    /// Whether --no-deconflict was explicitly provided
    pub no_deconflict: bool,
    /// Whether --no-semantic-validation was explicitly provided
    pub no_semantic_validation: bool,
    /// Whether --on-conflict was explicitly provided
    pub on_conflict: bool,
}

impl ArgsPresence {
    /// Determine which arguments were explicitly provided from clap's ArgMatches.
    pub fn from_matches(matches: &ArgMatches) -> Self {
        Self {
            provider: is_from_cli(matches, "provider"),
            format: is_from_cli(matches, "format"),
            rule_type: is_from_cli(matches, "rule_type"),
            compress: is_from_cli(matches, "compress"),
            chunk_size: is_from_cli(matches, "chunk_size"),
            no_confirm: is_from_cli(matches, "no_confirm"),
            retry_on_validation_failure: is_from_cli(matches, "retry_on_validation_failure"),
            no_deconflict: is_from_cli(matches, "no_deconflict"),
            no_semantic_validation: is_from_cli(matches, "no_semantic_validation"),
            on_conflict: is_from_cli(matches, "on_conflict"),
        }
    }
}

/// Check if an argument was explicitly provided on the command line (not from env or default).
fn is_from_cli(matches: &ArgMatches, name: &str) -> bool {
    matches.value_source(name) == Some(clap::parser::ValueSource::CommandLine)
}

/// CLI argument parsing with environment variable support.
///
/// Environment variables follow the pattern `RULEY_*` and are overridden by CLI flags.
/// Example: `RULEY_PROVIDER=openai` is overridden by `--provider anthropic`.
#[derive(Parser, Debug)]
#[command(name = "ruley")]
#[command(about = "Make your codebase ruley - generate AI IDE rules from codebases")]
#[command(version)]
pub struct Args {
    /// Path to repository (local path or remote URL)
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// LLM provider (anthropic, openai, ollama, openrouter)
    #[arg(
        short,
        long,
        default_value = "anthropic",
        env = "RULEY_PROVIDER",
        value_parser = clap::builder::PossibleValuesParser::new(SUPPORTED_PROVIDERS)
    )]
    pub provider: String,

    /// Model to use
    #[arg(short, long, env = "RULEY_MODEL")]
    pub model: Option<String>,

    /// Output file path
    #[arg(short, long, env = "RULEY_OUTPUT")]
    pub output: Option<PathBuf>,

    /// Path to existing repomix file for input
    #[arg(long, env = "RULEY_REPOMIX_FILE")]
    pub repomix_file: Option<PathBuf>,

    /// Output format(s), comma-separated
    #[arg(
        short,
        long,
        value_delimiter = ',',
        default_value = "cursor",
        env = "RULEY_FORMAT"
    )]
    pub format: Vec<OutputFormat>,

    /// Focus area for rule generation
    #[arg(long, env = "RULEY_DESCRIPTION")]
    pub description: Option<String>,

    /// Cursor rule type
    #[arg(long, default_value = "auto", env = "RULEY_RULE_TYPE")]
    pub rule_type: RuleType,

    /// Config file path
    #[arg(short, long, default_value = "ruley.toml", env = "RULEY_CONFIG")]
    pub config: PathBuf,

    /// Include only matching files (repeatable)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude matching files (repeatable)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Enable tree-sitter compression
    #[arg(long, env = "RULEY_COMPRESS")]
    pub compress: bool,

    /// Max tokens per LLM chunk
    #[arg(long, default_value_t = 100000, env = "RULEY_CHUNK_SIZE")]
    pub chunk_size: usize,

    /// Skip cost confirmation prompt
    #[arg(long, env = "RULEY_NO_CONFIRM")]
    pub no_confirm: bool,

    /// Show what would be processed without calling LLM
    #[arg(long, env = "RULEY_DRY_RUN")]
    pub dry_run: bool,

    /// Automatically retry with LLM fix when validation fails
    #[arg(long)]
    pub retry_on_validation_failure: bool,

    /// Disable LLM-based deconfliction with existing rule files
    #[arg(long)]
    pub no_deconflict: bool,

    /// Disable all semantic validation checks
    #[arg(long)]
    pub no_semantic_validation: bool,

    /// Conflict resolution strategy when output files exist (prompt, overwrite, skip, smart-merge)
    #[arg(
        long,
        env = "RULEY_ON_CONFLICT",
        value_parser = clap::builder::PossibleValuesParser::new(SUPPORTED_CONFLICT_STRATEGIES)
    )]
    pub on_conflict: Option<String>,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short)]
    pub quiet: bool,
}

impl OutputFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutputFormat::Cursor => "cursor",
            OutputFormat::Claude => "claude",
            OutputFormat::Copilot => "copilot",
            OutputFormat::Windsurf => "windsurf",
            OutputFormat::Aider => "aider",
            OutputFormat::Generic => "generic",
            OutputFormat::Json => "json",
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Parse CLI arguments and return both the parsed args and presence flags.
/// The presence flags indicate which arguments were explicitly provided on the command line.
///
/// # Errors
/// Returns a clap error if argument parsing fails.
pub fn parse() -> Result<(Args, ArgsPresence), clap::Error> {
    let matches = Args::command().get_matches();
    let presence = ArgsPresence::from_matches(&matches);
    let args = Args::from_arg_matches(&matches)?;
    Ok((args, presence))
}
