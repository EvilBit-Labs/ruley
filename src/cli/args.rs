use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "lowercase")]
pub enum OutputFormat {
    Cursor,
    Claude,
    Copilot,
    Windsurf,
    Aider,
    Generic,
    Json,
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

    /// LLM provider
    #[arg(short, long, default_value = "anthropic", env = "RULEY_PROVIDER")]
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
    #[arg(long, default_value = "agent", env = "RULEY_RULE_TYPE")]
    pub rule_type: String,

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

pub fn parse() -> Args {
    Args::parse()
}
