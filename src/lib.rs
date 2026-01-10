pub mod cli;
pub mod generator;
pub mod llm;
pub mod output;
pub mod packer;
pub mod utils;

use anyhow::Result;

pub async fn run() -> Result<()> {
    let args = cli::args::parse();
    let _config = cli::config::load(&args)?;

    // TODO: Implement orchestrator
    tracing::info!("ruley initialized");

    Ok(())
}
