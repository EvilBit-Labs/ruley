use anyhow::Result;
use ruley::{cli, run};

#[tokio::main]
async fn main() {
    if let Err(e) = run_main().await {
        eprintln!("Error: {}", e);
        // Display the full error chain for debugging
        for cause in e.chain().skip(1) {
            eprintln!("  Caused by: {}", cause);
        }
        std::process::exit(1);
    }
}

async fn run_main() -> Result<()> {
    // Parse CLI arguments (includes env vars) and track which flags were explicitly provided
    let (args, presence) = cli::args::parse()?;

    // Load config from files + env vars (already merged)
    let config = cli::config::load(&args)?;

    // Merge configurations: CLI args override config files only when explicitly provided
    let merged_config = cli::config::merge_config(&args, config, &presence);

    // Initialize logging based on verbosity
    ruley::init_logging(merged_config.verbose);

    // Run the pipeline
    run(merged_config).await
}
