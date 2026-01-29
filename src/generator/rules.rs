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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RuleType {
    /// Always apply these rules to all interactions
    AlwaysApply,
    /// Apply rules intelligently based on context
    #[default]
    ApplyIntelligently,
    /// Apply only to files matching specific patterns
    ApplyToSpecificFiles,
    /// Apply only when manually invoked
    ApplyManually,
}

impl std::str::FromStr for RuleType {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "always" | "alwaysapply" | "always_apply" => Self::AlwaysApply,
            "auto" | "intelligent" | "applyintelligently" => Self::ApplyIntelligently,
            "files" | "specific" | "applytospecificfiles" => Self::ApplyToSpecificFiles,
            "manual" | "applymanually" => Self::ApplyManually,
            _ => Self::ApplyIntelligently, // Default
        })
    }
}

impl RuleType {
    /// Convert to string for use in prompts and output.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlwaysApply => "Always Apply",
            Self::ApplyIntelligently => "Apply Intelligently",
            Self::ApplyToSpecificFiles => "Apply to Specific Files",
            Self::ApplyManually => "Apply Manually",
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
    pub fn formats(&self) -> Vec<&String> {
        self.rules_by_format.keys().collect()
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
/// This function is designed to be graceful - it will return the raw
/// response even if structured parsing fails.
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
pub fn get_default_rule_type(format: &str) -> RuleType {
    match format.to_lowercase().as_str() {
        "cursor" => RuleType::ApplyIntelligently,
        "claude" => RuleType::AlwaysApply,
        "copilot" => RuleType::ApplyIntelligently,
        _ => RuleType::ApplyIntelligently,
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
        assert_eq!("always".parse::<RuleType>().unwrap(), RuleType::AlwaysApply);
        assert_eq!(
            "auto".parse::<RuleType>().unwrap(),
            RuleType::ApplyIntelligently
        );
        assert_eq!(
            "files".parse::<RuleType>().unwrap(),
            RuleType::ApplyToSpecificFiles
        );
        assert_eq!(
            "manual".parse::<RuleType>().unwrap(),
            RuleType::ApplyManually
        );
        assert_eq!(
            "unknown".parse::<RuleType>().unwrap(),
            RuleType::ApplyIntelligently
        );
    }

    #[test]
    fn test_rule_type_as_str() {
        assert_eq!(RuleType::AlwaysApply.as_str(), "Always Apply");
        assert_eq!(RuleType::ApplyIntelligently.as_str(), "Apply Intelligently");
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
        let rules = FormattedRules::with_rule_type("cursor", "# Rules", RuleType::AlwaysApply);
        assert_eq!(rules.rule_type, Some(RuleType::AlwaysApply));
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
        assert_eq!(rules.formats().len(), 2);
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
        assert_eq!(
            get_default_rule_type("cursor"),
            RuleType::ApplyIntelligently
        );
        assert_eq!(get_default_rule_type("claude"), RuleType::AlwaysApply);
        assert_eq!(
            get_default_rule_type("copilot"),
            RuleType::ApplyIntelligently
        );
    }
}
