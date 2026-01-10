use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "ruley")]
#[command(about = "Make your codebase ruley - generate AI IDE rules from codebases")]
#[command(version)]
pub struct Args {
    /// Path to repository (local path or remote URL)
    #[arg(default_value = ".")]
    pub path: String,

    /// LLM provider
    #[arg(short, long, default_value = "anthropic")]
    pub provider: String,

    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,

    /// Output format(s), comma-separated
    #[arg(short, long, default_value = "cursor")]
    pub format: String,

    /// Focus area for rule generation
    #[arg(long)]
    pub description: Option<String>,

    /// Cursor rule type
    #[arg(long, default_value = "agent")]
    pub rule_type: String,

    /// Config file path
    #[arg(short, long, default_value = "ruley.toml")]
    pub config: String,

    /// Include only matching files (repeatable)
    #[arg(long)]
    pub include: Vec<String>,

    /// Exclude matching files (repeatable)
    #[arg(long)]
    pub exclude: Vec<String>,

    /// Enable tree-sitter compression
    #[arg(long)]
    pub compress: bool,

    /// Max tokens per LLM chunk
    #[arg(long, default_value_t = 100000)]
    pub chunk_size: usize,

    /// Skip cost confirmation prompt
    #[arg(long)]
    pub no_confirm: bool,

    /// Show what would be processed without calling LLM
    #[arg(long)]
    pub dry_run: bool,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-essential output
    #[arg(short)]
    pub quiet: bool,
}

pub fn parse() -> Args {
    Args::parse()
}
