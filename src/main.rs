use ruley::{cli, run};

#[tokio::main]
async fn main() {
    // Parse CLI arguments (includes env vars) and track which flags were explicitly provided
    let (args, presence) = cli::args::parse();

    // Load config from files + env vars (already merged)
    let config = match cli::config::load(&args) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    };

    // Merge configurations: CLI args override config files only when explicitly provided
    let merged_config = cli::config::merge_config(&args, config, &presence);

    // Run the pipeline
    if let Err(e) = run(merged_config).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
