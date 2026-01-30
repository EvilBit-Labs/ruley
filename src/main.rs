use anyhow::Result;
use ruley::utils::error::{RuleyError, format_error};
use ruley::{cli, run};

#[tokio::main]
async fn main() {
    // Try to determine verbose mode early for better error formatting
    // Default to false for early errors (before config is parsed)
    let verbose = std::env::args().any(|arg| arg == "-v" || arg == "--verbose");

    if let Err(e) = run_main().await {
        display_error(&e, verbose);
        std::process::exit(1);
    }
}

/// Display an error with contextual formatting.
///
/// Tries to downcast to `RuleyError` for rich formatting, falls back to
/// anyhow's error chain display for other errors.
fn display_error(error: &anyhow::Error, verbose: bool) {
    // Try to downcast to RuleyError for rich formatting
    if let Some(ruley_error) = error.downcast_ref::<RuleyError>() {
        eprintln!("{}", format_error(ruley_error, verbose));
    } else {
        // Fall back to formatted anyhow display
        eprintln!("\n\u{26a0} Error: {}", error);

        // Display the full error chain
        let causes: Vec<_> = error.chain().skip(1).collect();
        if !causes.is_empty() {
            eprintln!("\nCaused by:");
            for (i, cause) in causes.iter().enumerate() {
                let prefix = if i == causes.len() - 1 {
                    "\u{2514}\u{2500}"
                } else {
                    "\u{251c}\u{2500}"
                };
                eprintln!("{} {}", prefix, cause);
            }
        }

        if verbose {
            // Show backtrace in verbose mode if available
            let backtrace = error.backtrace();
            if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
                eprintln!("\nBacktrace:\n{}", backtrace);
            }
        }
    }

    eprintln!();
    eprintln!("Temp files preserved in .ruley/ for debugging");
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
