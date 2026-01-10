use crate::cli::args::Args;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: Option<GeneralConfig>,
    pub output: Option<OutputConfig>,
    pub include: Option<IncludeConfig>,
    pub exclude: Option<ExcludeConfig>,
    pub providers: Option<ProvidersConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub format: Option<String>,
    pub compress: Option<bool>,
    pub chunk_size: Option<usize>,
    pub no_confirm: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub formats: Option<Vec<String>>,
    pub paths: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludeConfig {
    pub patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExcludeConfig {
    pub patterns: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub anthropic: Option<ProviderConfig>,
    pub openai: Option<ProviderConfig>,
    pub ollama: Option<OllamaConfig>,
    pub openrouter: Option<ProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub model: Option<String>,
    pub max_tokens: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    pub host: Option<String>,
    pub model: Option<String>,
}

pub fn load(args: &Args) -> Result<Config> {
    let config_path = &args.config; // Already PathBuf, no conversion needed

    if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config {
            general: None,
            output: None,
            include: None,
            exclude: None,
            providers: None,
        })
    }
}
