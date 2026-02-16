//! Rule structures and parsing for generated AI IDE rules.
//!
//! This module provides:
//! - `GeneratedRules`: Container for analysis results and format-specific rules
//! - `FormattedRules`: Format-specific rule content
//! - `GenerationMetadata`: Metadata about the generation process
//! - `parse_analysis_response`: Parser for LLM analysis responses
//!
//! # Example
//!
//! ```ignore
//! use ruley::generator::rules::{parse_analysis_response, RuleType};
//!
//! let response = llm_client.complete(&prompt).await?;
//! let rules = parse_analysis_response(&response.content, "anthropic", "claude-3-opus")?;
//! ```

use crate::utils::error::RuleyError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rule application type for Cursor format.
///
/// Determines how and when rules are applied during AI assistance.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
#[value(rename_all = "lowercase")]
pub enum RuleType {
    /// Always apply these rules to all interactions
    #[serde(alias = "AlwaysApply", alias = "always_apply")]
    #[value(name = "always", help = "Always Apply")]
    Always,
    /// Apply rules intelligently based on context
    #[default]
    #[serde(alias = "ApplyIntelligently", alias = "auto")]
    #[value(name = "auto", help = "Apply Intelligently")]
    Auto,
    /// Apply only to files matching specific patterns
    #[serde(alias = "ApplyToSpecificFiles", alias = "specific")]
    #[value(name = "files", help = "Apply to Specific Files")]
    Files,
    /// Apply only when manually invoked
    #[serde(alias = "ApplyManually", alias = "apply_manually")]
    #[value(name = "manual", help = "Apply Manually")]
    Manual,
}

impl std::str::FromStr for RuleType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "always" | "alwaysapply" | "always_apply" => Ok(Self::Always),
            "auto" | "intelligent" | "applyintelligently" => Ok(Self::Auto),
            "files" | "specific" | "applytospecificfiles" => Ok(Self::Files),
            "manual" | "applymanually" => Ok(Self::Manual),
            _ => Err(format!("unknown rule type: '{}'", s)),
        }
    }
}

impl RuleType {
    /// Human-friendly label for use in prompts and output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "Always Apply",
            Self::Auto => "Apply Intelligently",
            Self::Files => "Apply to Specific Files",
            Self::Manual => "Apply Manually",
        }
    }

    /// Machine-readable slug for use in config values and prompt logic.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Auto => "auto",
            Self::Files => "files",
            Self::Manual => "manual",
        }
    }
}

/// Format-specific rules content.
///
/// Contains the formatted rules for a specific output format
/// (e.g., Cursor .mdc, Claude CLAUDE.md, Copilot instructions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedRules {
    /// The format identifier (e.g., "cursor", "claude", "copilot")
    pub format: String,
    /// The formatted rules content ready for output
    pub content: String,
    /// Optional rule type for formats that support it
    pub rule_type: Option<RuleType>,
}

impl FormattedRules {
    /// Create new formatted rules.
    pub fn new(format: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            format: format.into(),
            content: content.into(),
            rule_type: None,
        }
    }

    /// Create formatted rules with a specific rule type.
    pub fn with_rule_type(
        format: impl Into<String>,
        content: impl Into<String>,
        rule_type: RuleType,
    ) -> Self {
        Self {
            format: format.into(),
            content: content.into(),
            rule_type: Some(rule_type),
        }
    }
}

/// Metadata about the rule generation process.
///
/// Captures information about when, how, and at what cost
/// the rules were generated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Timestamp of generation (ISO 8601 format)
    pub timestamp: String,
    /// LLM provider used (e.g., "anthropic", "openai")
    pub provider: String,
    /// Model name used (e.g., "claude-3-opus", "gpt-4o")
    pub model: String,
    /// Number of input tokens used
    pub input_tokens: usize,
    /// Number of output tokens generated
    pub output_tokens: usize,
    /// Total cost in USD
    pub cost: f64,
}

impl GenerationMetadata {
    /// Create new metadata with current timestamp.
    pub fn new(provider: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            provider: provider.into(),
            model: model.into(),
            input_tokens: 0,
            output_tokens: 0,
            cost: 0.0,
        }
    }

    /// Update token counts and cost.
    pub fn with_usage(mut self, input_tokens: usize, output_tokens: usize, cost: f64) -> Self {
        self.input_tokens = input_tokens;
        self.output_tokens = output_tokens;
        self.cost = cost;
        self
    }
}

impl Default for GenerationMetadata {
    fn default() -> Self {
        Self::new("unknown", "unknown")
    }
}

/// Container for generated rules and metadata.
///
/// This is the primary output structure from the rule generation pipeline.
/// It contains the raw analysis, format-specific rules, and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedRules {
    /// Raw analysis output from the LLM
    pub analysis: String,
    /// Format-specific rules indexed by format name
    pub rules_by_format: HashMap<String, FormattedRules>,
    /// Metadata about the generation process
    pub metadata: GenerationMetadata,
}

impl GeneratedRules {
    /// Create new generated rules from an analysis response.
    pub fn new(analysis: impl Into<String>) -> Self {
        Self {
            analysis: analysis.into(),
            rules_by_format: HashMap::new(),
            metadata: GenerationMetadata::default(),
        }
    }

    /// Create generated rules with metadata.
    pub fn with_metadata(analysis: impl Into<String>, metadata: GenerationMetadata) -> Self {
        Self {
            analysis: analysis.into(),
            rules_by_format: HashMap::new(),
            metadata,
        }
    }

    /// Add formatted rules for a specific format.
    pub fn add_format(&mut self, rules: FormattedRules) {
        self.rules_by_format.insert(rules.format.clone(), rules);
    }

    /// Get rules for a specific format.
    pub fn get_format(&self, format: &str) -> Option<&FormattedRules> {
        self.rules_by_format.get(format)
    }

    /// Check if rules exist for a specific format.
    pub fn has_format(&self, format: &str) -> bool {
        self.rules_by_format.contains_key(format)
    }

    /// Get all format names that have rules.
    ///
    /// Returns an iterator to avoid allocation. Callers can collect if needed.
    pub fn formats(&self) -> impl Iterator<Item = &str> {
        self.rules_by_format.keys().map(String::as_str)
    }
}

/// Parse an LLM analysis response into a GeneratedRules structure.
///
/// This function takes the raw LLM response and creates a `GeneratedRules`
/// structure. The analysis is stored as-is, and format-specific rules
/// can be added later through refinement prompts.
///
/// # Arguments
///
/// * `response` - The raw LLM response text
/// * `provider` - The LLM provider name (e.g., "anthropic")
/// * `model` - The model name (e.g., "claude-3-opus")
///
/// # Returns
///
/// A `GeneratedRules` structure with the analysis and metadata.
///
/// # Errors
///
/// Currently this function always succeeds, returning the raw response.
/// The `Result` return type is retained for future compatibility when
/// structured JSON parsing may be added, which could fail on malformed
/// responses. This design allows callers to use `?` operator consistently
/// and makes the API forward-compatible.
///
/// # Example
///
/// ```ignore
/// let rules = parse_analysis_response(&response, "anthropic", "claude-3-opus")?;
/// println!("Analysis: {}", rules.analysis);
/// ```
pub fn parse_analysis_response(
    response: &str,
    provider: &str,
    model: &str,
) -> Result<GeneratedRules, RuleyError> {
    // Create metadata
    let metadata = GenerationMetadata::new(provider, model);

    // Create rules structure with the analysis
    let rules = GeneratedRules::with_metadata(response, metadata);

    Ok(rules)
}

/// Get the default rule type for a given format.
///
/// Different formats have different default behaviors:
/// - Cursor: ApplyIntelligently (context-aware)
/// - Claude: AlwaysApply (project-wide instructions)
/// - Copilot: ApplyIntelligently (code completion context)
/// - Windsurf: ApplyIntelligently (context-aware)
/// - Aider: ApplyIntelligently (code-focused)
/// - Generic: ApplyIntelligently (universal default)
pub fn get_default_rule_type(format: &str) -> RuleType {
    match format.to_lowercase().as_str() {
        "cursor" => RuleType::Auto,
        "claude" => RuleType::Always,
        "copilot" => RuleType::Auto,
        "windsurf" => RuleType::Auto,
        "aider" => RuleType::Auto,
        "generic" => RuleType::Auto,
        _ => RuleType::Auto,
    }
}

// ============================================================================
// Legacy structures for backwards compatibility
// These match the original structure and can be used for structured output parsing
// ============================================================================

/// Project information extracted from analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub description: String,
}

/// Technology stack information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechStack {
    pub language: Option<String>,
    pub framework: Option<String>,
    pub build_tool: Option<String>,
}

/// A coding convention rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Convention {
    pub category: String,
    pub rule: String,
    pub rationale: Option<String>,
    pub examples: Vec<Example>,
}

/// An important file in the codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyFile {
    pub path: String,
    pub description: String,
}

/// Architecture description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureInfo {
    pub description: String,
}

/// A common development task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub steps: Vec<String>,
}

/// An anti-pattern to avoid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Antipattern {
    pub description: String,
    pub example: Option<String>,
}

/// A code example (valid or invalid).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    pub description: String,
    pub code: String,
    pub is_valid: bool,
}

/// Structured rules from LLM analysis (legacy format).
///
/// This structure is used when the LLM returns structured JSON output.
/// For most use cases, prefer `GeneratedRules` with format-specific rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredRules {
    pub project: ProjectInfo,
    pub tech_stack: TechStack,
    pub conventions: Vec<Convention>,
    pub key_files: Vec<KeyFile>,
    pub architecture: ArchitectureInfo,
    pub tasks: Vec<Task>,
    pub antipatterns: Vec<Antipattern>,
    pub examples: Vec<Example>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_type_from_str() {
        assert_eq!("always".parse::<RuleType>().unwrap(), RuleType::Always);
        assert_eq!("auto".parse::<RuleType>().unwrap(), RuleType::Auto);
        assert_eq!("files".parse::<RuleType>().unwrap(), RuleType::Files);
        assert_eq!("manual".parse::<RuleType>().unwrap(), RuleType::Manual);
        // Unknown values return an error
        assert!("unknown".parse::<RuleType>().is_err());
        assert!("invalid".parse::<RuleType>().is_err());
    }

    #[test]
    fn test_rule_type_as_str() {
        assert_eq!(RuleType::Always.as_str(), "Always Apply");
        assert_eq!(RuleType::Auto.as_str(), "Apply Intelligently");
    }

    #[test]
    fn test_rule_type_slug() {
        assert_eq!(RuleType::Always.slug(), "always");
        assert_eq!(RuleType::Auto.slug(), "auto");
        assert_eq!(RuleType::Files.slug(), "files");
        assert_eq!(RuleType::Manual.slug(), "manual");
    }

    #[test]
    fn test_rule_type_serde_roundtrip() {
        // Serialize uses lowercase due to rename_all
        let json = serde_json::to_string(&RuleType::Always).unwrap();
        assert_eq!(json, "\"always\"");

        // Deserialize lowercase strings
        let rt: RuleType = serde_json::from_str("\"always\"").unwrap();
        assert_eq!(rt, RuleType::Always);
        let rt: RuleType = serde_json::from_str("\"auto\"").unwrap();
        assert_eq!(rt, RuleType::Auto);
        let rt: RuleType = serde_json::from_str("\"manual\"").unwrap();
        assert_eq!(rt, RuleType::Manual);
        let rt: RuleType = serde_json::from_str("\"files\"").unwrap();
        assert_eq!(rt, RuleType::Files);
    }

    #[test]
    fn test_formatted_rules_new() {
        let rules = FormattedRules::new("cursor", "# Rules content");
        assert_eq!(rules.format, "cursor");
        assert_eq!(rules.content, "# Rules content");
        assert!(rules.rule_type.is_none());
    }

    #[test]
    fn test_formatted_rules_with_rule_type() {
        let rules = FormattedRules::with_rule_type("cursor", "# Rules", RuleType::Always);
        assert_eq!(rules.rule_type, Some(RuleType::Always));
    }

    #[test]
    fn test_generation_metadata_new() {
        let metadata = GenerationMetadata::new("anthropic", "claude-3-opus");
        assert_eq!(metadata.provider, "anthropic");
        assert_eq!(metadata.model, "claude-3-opus");
        assert!(!metadata.timestamp.is_empty());
    }

    #[test]
    fn test_generation_metadata_with_usage() {
        let metadata = GenerationMetadata::new("openai", "gpt-4o").with_usage(1000, 500, 0.05);
        assert_eq!(metadata.input_tokens, 1000);
        assert_eq!(metadata.output_tokens, 500);
        assert!((metadata.cost - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generated_rules_new() {
        let rules = GeneratedRules::new("Test analysis");
        assert_eq!(rules.analysis, "Test analysis");
        assert!(rules.rules_by_format.is_empty());
    }

    #[test]
    fn test_generated_rules_add_format() {
        let mut rules = GeneratedRules::new("Test analysis");
        rules.add_format(FormattedRules::new("cursor", "# Cursor rules"));
        rules.add_format(FormattedRules::new("claude", "# Claude rules"));

        assert!(rules.has_format("cursor"));
        assert!(rules.has_format("claude"));
        assert!(!rules.has_format("copilot"));
        assert_eq!(rules.formats().count(), 2);
    }

    #[test]
    fn test_generated_rules_get_format() {
        let mut rules = GeneratedRules::new("Test");
        rules.add_format(FormattedRules::new("cursor", "# Content"));

        let cursor_rules = rules.get_format("cursor");
        assert!(cursor_rules.is_some());
        assert_eq!(cursor_rules.unwrap().content, "# Content");

        assert!(rules.get_format("copilot").is_none());
    }

    #[test]
    fn test_parse_analysis_response() {
        let response = "This is a test analysis of the codebase.";
        let result = parse_analysis_response(response, "anthropic", "claude-3-opus");

        assert!(result.is_ok());
        let rules = result.unwrap();
        assert_eq!(rules.analysis, response);
        assert_eq!(rules.metadata.provider, "anthropic");
        assert_eq!(rules.metadata.model, "claude-3-opus");
    }

    #[test]
    fn test_get_default_rule_type() {
        assert_eq!(get_default_rule_type("cursor"), RuleType::Auto);
        assert_eq!(get_default_rule_type("claude"), RuleType::Always);
        assert_eq!(get_default_rule_type("copilot"), RuleType::Auto);
        assert_eq!(get_default_rule_type("windsurf"), RuleType::Auto);
        assert_eq!(get_default_rule_type("aider"), RuleType::Auto);
        assert_eq!(get_default_rule_type("generic"), RuleType::Auto);
    }
}
