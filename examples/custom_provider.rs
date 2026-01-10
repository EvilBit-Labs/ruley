//! Example of using ruley with a custom LLM provider.
//!
//! This demonstrates how to configure ruley to use different LLM backends
//! such as Ollama for local inference or OpenRouter for model routing.
//!
//! Run with: `cargo run --example custom_provider`

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt().with_env_filter("debug").init();

    tracing::info!("Custom provider example");

    // Example: Configure Ollama provider for local inference
    //
    // let config = ruley::Config::builder()
    //     .provider(Provider::Ollama {
    //         base_url: "http://localhost:11434".into(),
    //         model: "codellama:13b".into(),
    //     })
    //     .build()?;

    // Example: Configure OpenRouter for model routing
    //
    // let config = ruley::Config::builder()
    //     .provider(Provider::OpenRouter {
    //         api_key: std::env::var("OPENROUTER_API_KEY")?,
    //         model: "anthropic/claude-3-sonnet".into(),
    //     })
    //     .build()?;

    tracing::info!("Provider configuration example completed");

    Ok(())
}
