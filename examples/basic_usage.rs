//! Basic usage example for ruley library.
//!
//! This example demonstrates how to use ruley programmatically
//! to analyze a codebase and generate AI IDE rules.
//!
//! Run with: `cargo run --example basic_usage`

use anyhow::Result;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    tracing::info!("Starting ruley example...");

    // Example: Define the target directory to analyze
    let target_dir = PathBuf::from(".");

    tracing::info!("Analyzing directory: {:?}", target_dir);

    // TODO: Add actual ruley library usage once the public API is defined
    //
    // Example workflow:
    // 1. Pack the codebase
    // 2. Send to LLM for analysis
    // 3. Generate rules for target IDE
    //
    // let config = ruley::Config::builder()
    //     .directory(target_dir)
    //     .output_format(OutputFormat::Cursor)
    //     .provider(Provider::Anthropic)
    //     .build()?;
    //
    // let rules = ruley::generate_rules(config).await?;
    // rules.write_to_directory(".cursor/rules")?;

    tracing::info!("Example completed successfully!");

    Ok(())
}
