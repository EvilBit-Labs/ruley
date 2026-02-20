// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Validation module for generated rule outputs.
//!
//! Provides three-layer validation (syntax, schema, semantic) for each output format.
//! Validators check rendered formatter output (bytes-to-write) before finalization.
//!
//! # Validation Layers
//!
//! 1. **Syntax**: Ensures content parses correctly (JSON, Markdown, YAML frontmatter)
//! 2. **Schema**: Checks required sections and structure per format
//! 3. **Semantic**: Validates file paths exist, detects contradictions, checks consistency

use crate::cli::config::SemanticValidationConfig;
use crate::packer::CompressedCodebase;
use anyhow::Result;
use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

/// Regex for extracting file paths from rule content.
static FILE_PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?:^|\s|`|"|')([a-zA-Z0-9_./\-]+\.[a-zA-Z0-9]+)(?:\s|`|"|'|$|[,;:\)])"#)
        .expect("file path regex is invalid")
});

/// Regex for detecting contradictory spacing rules.
static TABS_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\buse\s+tabs\b").expect("tabs regex is invalid"));

static SPACES_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\buse\s+spaces\b").expect("spaces regex is invalid"));

/// Regex for extracting indentation width (e.g., "2 spaces", "4-space indent").
static INDENT_WIDTH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(\d+)[\s-]*(?:space|indent)").expect("indent width regex is invalid")
});

/// Regex for detecting naming convention references.
static CAMEL_CASE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:use\s+)?camel\s*case\b").expect("camelCase regex is invalid")
});

static SNAKE_CASE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:use\s+)?snake\s*[_\s]?case\b")
        .expect("snake_case regex is invalid")
});

/// Regex for detecting line length limits.
static LINE_LENGTH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(?:max(?:imum)?|line)\s*(?:line\s*)?(?:length|width|chars?)\s*(?:of\s*|:\s*|=\s*)?(\d+)")
        .expect("line length regex is invalid")
});

/// Regex for detecting semicolon usage rules.
static SEMICOLONS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:always\s+use|require|use)\s+semicolons?\b")
        .expect("semicolons regex is invalid")
});

static NO_SEMICOLONS_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:no|omit|avoid|don'?t\s+use)\s+semicolons?\b")
        .expect("no semicolons regex is invalid")
});

/// Regex for detecting quote style rules.
static SINGLE_QUOTES_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:use\s+)?single\s+quotes?\b")
        .expect("single quotes regex is invalid")
});

static DOUBLE_QUOTES_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:use\s+)?double\s+quotes?\b")
        .expect("double quotes regex is invalid")
});

/// Identifies which validation layer produced an error or warning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLayer {
    /// Content parsing validation (JSON, Markdown, YAML)
    Syntax,
    /// Required sections and structure validation
    Schema,
    /// File path existence, contradictions, consistency, reality checks
    Semantic,
}

impl fmt::Display for ValidationLayer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax => write!(f, "Syntax"),
            Self::Schema => write!(f, "Schema"),
            Self::Semantic => write!(f, "Semantic"),
        }
    }
}

/// A blocking validation error for a specific format.
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Which validation layer produced this error
    pub layer: ValidationLayer,
    /// Human-readable error description
    pub message: String,
    /// Optional location in the content (e.g., line number)
    pub location: Option<String>,
    /// Optional suggestion for fixing the error
    pub suggestion: Option<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.layer, self.message)?;
        if let Some(ref loc) = self.location {
            write!(f, " at {}", loc)?;
        }
        if let Some(ref sug) = self.suggestion {
            write!(f, "\n      Suggestion: {}", sug)?;
        }
        Ok(())
    }
}

/// A non-blocking validation warning for a specific format.
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Which validation layer produced this warning
    pub layer: ValidationLayer,
    /// Human-readable warning description
    pub message: String,
    /// Optional location in the content
    pub location: Option<String>,
    /// Optional suggestion for improvement
    pub suggestion: Option<String>,
}

/// Result of validating a single format's output.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// The format that was validated
    pub format: String,
    /// Whether validation passed (no errors)
    pub passed: bool,
    /// Blocking errors that prevent output
    pub errors: Vec<ValidationError>,
    /// Non-blocking warnings
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationResult {
    /// Create a passing validation result.
    #[cfg(test)]
    fn pass(format: &str) -> Self {
        Self {
            format: format.to_string(),
            passed: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a result from collected errors and warnings.
    fn from_checks(
        format: &str,
        errors: Vec<ValidationError>,
        warnings: Vec<ValidationWarning>,
    ) -> Self {
        Self {
            format: format.to_string(),
            passed: errors.is_empty(),
            errors,
            warnings,
        }
    }
}

/// Trait for format-specific validators.
///
/// Each format implements this trait to provide syntax, schema, and semantic validation.
pub trait FormatValidator: Send + Sync {
    /// Validate the rendered content for this format.
    ///
    /// # Arguments
    ///
    /// * `content` - The rendered output content to validate
    /// * `config` - Semantic validation configuration (which checks to run)
    /// * `codebase` - The compressed codebase for file path validation
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult>;
}

// ============================================================================
// Format-specific validators
// ============================================================================

/// Validator for Cursor .mdc format.
pub struct CursorValidator;

impl FormatValidator for CursorValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Syntax: Check for valid Markdown structure
        validate_markdown_syntax(content, &mut errors);

        // Schema: Cursor rules should have frontmatter-like structure
        if let Some(after_prefix) = content.strip_prefix("---") {
            // Has YAML frontmatter - validate it
            if let Some(end) = after_prefix.find("---") {
                let frontmatter = &after_prefix[..end];
                if !frontmatter.contains("description") {
                    warnings.push(ValidationWarning {
                        layer: ValidationLayer::Schema,
                        message: "Cursor rule frontmatter missing 'description' field".to_string(),
                        location: Some("line 1".to_string()),
                        suggestion: Some(
                            "Add a 'description' field to the frontmatter".to_string(),
                        ),
                    });
                }
            } else {
                errors.push(ValidationError {
                    layer: ValidationLayer::Syntax,
                    message: "Unclosed YAML frontmatter (missing closing ---)".to_string(),
                    location: Some("line 1".to_string()),
                    suggestion: Some("Add closing --- after frontmatter".to_string()),
                });
            }
        }

        // Semantic checks
        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("cursor", errors, warnings))
    }
}

/// Validator for Claude CLAUDE.md format.
pub struct ClaudeValidator;

impl FormatValidator for ClaudeValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Syntax: Valid Markdown
        validate_markdown_syntax(content, &mut errors);

        // Schema: Claude format should have key sections
        let content_lower = content.to_lowercase();
        let expected_sections = ["# ", "## "];
        let has_any_heading = expected_sections.iter().any(|s| content_lower.contains(s));
        if !has_any_heading {
            errors.push(ValidationError {
                layer: ValidationLayer::Schema,
                message: "Claude rules missing Markdown headings".to_string(),
                location: None,
                suggestion: Some("Add section headings using # or ## syntax".to_string()),
            });
        }

        // Semantic checks
        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("claude", errors, warnings))
    }
}

/// Validator for GitHub Copilot format.
pub struct CopilotValidator;

impl FormatValidator for CopilotValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Syntax: Valid Markdown
        validate_markdown_syntax(content, &mut errors);

        // Schema: Copilot instructions should have content
        if content.trim().is_empty() {
            errors.push(ValidationError {
                layer: ValidationLayer::Schema,
                message: "Copilot instructions file is empty".to_string(),
                location: None,
                suggestion: Some("Add coding instructions for Copilot".to_string()),
            });
        }

        // Semantic checks
        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("copilot", errors, warnings))
    }
}

/// Validator for Windsurf format.
pub struct WindsurfValidator;

impl FormatValidator for WindsurfValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        validate_markdown_syntax(content, &mut errors);

        if content.trim().is_empty() {
            errors.push(ValidationError {
                layer: ValidationLayer::Schema,
                message: "Windsurf rules file is empty".to_string(),
                location: None,
                suggestion: Some("Add coding rules for Windsurf".to_string()),
            });
        }

        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("windsurf", errors, warnings))
    }
}

/// Validator for Aider format.
pub struct AiderValidator;

impl FormatValidator for AiderValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        validate_markdown_syntax(content, &mut errors);

        if content.trim().is_empty() {
            errors.push(ValidationError {
                layer: ValidationLayer::Schema,
                message: "Aider conventions file is empty".to_string(),
                location: None,
                suggestion: Some("Add coding conventions for Aider".to_string()),
            });
        }

        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("aider", errors, warnings))
    }
}

/// Validator for generic Markdown format.
pub struct GenericValidator;

impl FormatValidator for GenericValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        validate_markdown_syntax(content, &mut errors);

        if content.trim().is_empty() {
            errors.push(ValidationError {
                layer: ValidationLayer::Schema,
                message: "Generic rules file is empty".to_string(),
                location: None,
                suggestion: Some("Add coding rules content".to_string()),
            });
        }

        validate_semantic(content, config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("generic", errors, warnings))
    }
}

/// Validator for JSON format.
pub struct JsonValidator;

impl FormatValidator for JsonValidator {
    fn validate(
        &self,
        content: &str,
        config: &SemanticValidationConfig,
        codebase: &CompressedCodebase,
    ) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Syntax: Must be valid JSON
        if let Err(e) = serde_json::from_str::<serde_json::Value>(content) {
            errors.push(ValidationError {
                layer: ValidationLayer::Syntax,
                message: format!("Invalid JSON: {}", e),
                location: Some(format!("line {}", e.line())),
                suggestion: Some("Fix JSON syntax errors".to_string()),
            });
            // If JSON is invalid, skip further checks
            return Ok(ValidationResult::from_checks("json", errors, warnings));
        }

        // Schema: Should be a non-empty object or array
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(content) {
            match &value {
                serde_json::Value::Object(map) if map.is_empty() => {
                    errors.push(ValidationError {
                        layer: ValidationLayer::Schema,
                        message: "JSON output is an empty object".to_string(),
                        location: None,
                        suggestion: Some("Ensure rules content is generated".to_string()),
                    });
                }
                serde_json::Value::Null => {
                    errors.push(ValidationError {
                        layer: ValidationLayer::Schema,
                        message: "JSON output is null".to_string(),
                        location: None,
                        suggestion: Some("Ensure rules content is generated".to_string()),
                    });
                }
                _ => {}
            }
        }

        // Semantic checks (skip file path checks for JSON by default)
        let json_config = SemanticValidationConfig {
            check_file_paths: config.check_file_paths,
            check_contradictions: config.check_contradictions,
            check_consistency: false, // JSON doesn't need cross-format consistency
            check_reality: config.check_reality,
        };
        validate_semantic(content, &json_config, codebase, &mut errors, &mut warnings);

        Ok(ValidationResult::from_checks("json", errors, warnings))
    }
}

// ============================================================================
// Shared validation helpers
// ============================================================================

/// Basic Markdown syntax validation.
fn validate_markdown_syntax(content: &str, errors: &mut Vec<ValidationError>) {
    if content.trim().is_empty() {
        errors.push(ValidationError {
            layer: ValidationLayer::Syntax,
            message: "Content is empty".to_string(),
            location: None,
            suggestion: Some("Ensure content was generated".to_string()),
        });
        return;
    }

    // Check for unclosed code blocks
    let triple_backtick_count = content.matches("```").count();
    if !triple_backtick_count.is_multiple_of(2) {
        errors.push(ValidationError {
            layer: ValidationLayer::Syntax,
            message: "Unclosed code block (odd number of ``` delimiters)".to_string(),
            location: None,
            suggestion: Some("Add closing ``` to unclosed code blocks".to_string()),
        });
    }
}

/// Shared semantic validation across all formats.
fn validate_semantic(
    content: &str,
    config: &SemanticValidationConfig,
    codebase: &CompressedCodebase,
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    // Check file paths exist in codebase
    if config.check_file_paths {
        let referenced_paths = extract_file_paths(content);
        for path in &referenced_paths {
            if !matches_any_file(path, codebase) {
                warnings.push(ValidationWarning {
                    layer: ValidationLayer::Semantic,
                    message: format!("File path \"{}\" not found in codebase", path),
                    location: None,
                    suggestion: Some("Remove this reference or fix the file path".to_string()),
                });
            }
        }
    }

    // Check for contradictions
    if config.check_contradictions {
        let contradictions = detect_contradictions(content);
        for contradiction in contradictions {
            errors.push(ValidationError {
                layer: ValidationLayer::Semantic,
                message: contradiction,
                location: None,
                suggestion: Some("Resolve the contradictory rules".to_string()),
            });
        }
    }

    // Reality check: verify rules reference actual languages/frameworks
    if config.check_reality {
        validate_reality(content, codebase, warnings);
    }
}

/// Extract file paths from rule content.
fn extract_file_paths(content: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for cap in FILE_PATH_RE.captures_iter(content) {
        let path = cap[1].to_string();
        // Filter out common false positives
        if !is_common_non_path(&path) && path.contains('/') {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

/// Check if a string is a common non-path that looks like a file path.
fn is_common_non_path(s: &str) -> bool {
    let lower = s.to_lowercase();
    // URLs, version strings, etc.
    lower.starts_with("http")
        || lower.starts_with("ftp")
        || lower.contains("://")
        || lower.starts_with("v0.")
        || lower.starts_with("v1.")
        || lower.starts_with("v2.")
        || lower.ends_with(".com")
        || lower.ends_with(".org")
        || lower.ends_with(".io")
        || lower.ends_with(".net")
}

/// Check if a path matches any file in the compressed codebase.
fn matches_any_file(path: &str, codebase: &CompressedCodebase) -> bool {
    codebase.files.iter().any(|f| {
        let file_path = f.path.to_string_lossy();
        // Exact match or suffix match (rules may reference relative paths)
        file_path == path || file_path.ends_with(path) || path.ends_with(&*file_path)
    })
}

/// Detect contradictory rules in content.
fn detect_contradictions(content: &str) -> Vec<String> {
    let mut contradictions = Vec::new();

    // Check tabs vs spaces contradiction
    let has_tabs = TABS_RE.is_match(content);
    let has_spaces = SPACES_RE.is_match(content);
    if has_tabs && has_spaces {
        contradictions.push(
            "Contradictory indentation rules: both \"use tabs\" and \"use spaces\" found"
                .to_string(),
        );
    }

    contradictions
}

/// Key conventions extracted from a format's rendered content.
///
/// Used for cross-format consistency checking to ensure core conventions
/// do not conflict or go missing across formats.
#[derive(Debug, Clone, Default)]
struct ExtractedConventions {
    /// "tabs" or "spaces" if detected
    indentation_style: Option<String>,
    /// Indentation width (e.g., 2, 4) if detected
    indent_width: Option<u32>,
    /// Naming conventions detected (e.g., "camelCase", "snake_case")
    naming_conventions: Vec<String>,
    /// Max line length if detected
    line_length: Option<u32>,
    /// "use" or "no" for semicolons
    semicolons: Option<String>,
    /// "single" or "double" for quote style
    quote_style: Option<String>,
}

/// Extract key conventions from rendered format content.
fn extract_conventions(content: &str) -> ExtractedConventions {
    let mut conventions = ExtractedConventions::default();

    // Indentation style
    if TABS_RE.is_match(content) {
        conventions.indentation_style = Some("tabs".to_string());
    } else if SPACES_RE.is_match(content) {
        conventions.indentation_style = Some("spaces".to_string());
    }

    // Indent width
    if let Some(cap) = INDENT_WIDTH_RE.captures(content)
        && let Ok(width) = cap[1].parse::<u32>()
    {
        conventions.indent_width = Some(width);
    }

    // Naming conventions
    if CAMEL_CASE_RE.is_match(content) {
        conventions.naming_conventions.push("camelCase".to_string());
    }
    if SNAKE_CASE_RE.is_match(content) {
        conventions
            .naming_conventions
            .push("snake_case".to_string());
    }

    // Line length
    if let Some(cap) = LINE_LENGTH_RE.captures(content)
        && let Ok(length) = cap[1].parse::<u32>()
        && (40..=500).contains(&length)
    {
        conventions.line_length = Some(length);
    }

    // Semicolons
    if SEMICOLONS_RE.is_match(content) {
        conventions.semicolons = Some("use".to_string());
    } else if NO_SEMICOLONS_RE.is_match(content) {
        conventions.semicolons = Some("no".to_string());
    }

    // Quote style
    if SINGLE_QUOTES_RE.is_match(content) {
        conventions.quote_style = Some("single".to_string());
    } else if DOUBLE_QUOTES_RE.is_match(content) {
        conventions.quote_style = Some("double".to_string());
    }

    conventions
}

/// Validate cross-format consistency of core conventions.
///
/// Compares key conventions extracted from all rendered format outputs
/// to ensure they do not conflict or go missing. Reports conflicts as
/// blocking errors and missing conventions as warnings.
fn validate_cross_format_consistency(
    all_rendered_outputs: &HashMap<String, String>,
    errors: &mut Vec<ValidationError>,
) {
    if all_rendered_outputs.len() < 2 {
        return; // Need at least 2 formats to check consistency
    }

    // Extract conventions from each format
    let conventions_by_format: HashMap<&str, ExtractedConventions> = all_rendered_outputs
        .iter()
        .map(|(format, content)| (format.as_str(), extract_conventions(content)))
        .collect();

    // Collect all detected values per convention for conflict detection
    check_convention_conflict(
        &conventions_by_format,
        "indentation style",
        |c| c.indentation_style.clone(),
        errors,
    );

    check_convention_conflict(
        &conventions_by_format,
        "indent width",
        |c| c.indent_width.map(|w| w.to_string()),
        errors,
    );

    check_convention_conflict(
        &conventions_by_format,
        "semicolons",
        |c| c.semicolons.clone(),
        errors,
    );

    check_convention_conflict(
        &conventions_by_format,
        "quote style",
        |c| c.quote_style.clone(),
        errors,
    );

    check_convention_conflict(
        &conventions_by_format,
        "line length",
        |c| c.line_length.map(|l| l.to_string()),
        errors,
    );

    // Check for missing conventions: if a majority of formats specify a convention,
    // formats missing it are flagged
    check_convention_missing(
        &conventions_by_format,
        "indentation style",
        |c| c.indentation_style.is_some(),
        errors,
    );

    check_convention_missing(
        &conventions_by_format,
        "naming conventions",
        |c| !c.naming_conventions.is_empty(),
        errors,
    );
}

/// Check if a specific convention conflicts across formats.
fn check_convention_conflict<F>(
    conventions_by_format: &HashMap<&str, ExtractedConventions>,
    convention_name: &str,
    extractor: F,
    errors: &mut Vec<ValidationError>,
) where
    F: Fn(&ExtractedConventions) -> Option<String>,
{
    let mut values: HashMap<String, Vec<String>> = HashMap::new();
    for (format, conventions) in conventions_by_format {
        if let Some(value) = extractor(conventions) {
            values.entry(value).or_default().push((*format).to_string());
        }
    }

    if values.len() > 1 {
        let conflict_details: Vec<String> = values
            .iter()
            .map(|(value, formats)| format!("{} in [{}]", value, formats.join(", ")))
            .collect();

        errors.push(ValidationError {
            layer: ValidationLayer::Semantic,
            message: format!(
                "Cross-format conflict for {}: {}",
                convention_name,
                conflict_details.join(" vs ")
            ),
            location: None,
            suggestion: Some(format!(
                "Ensure all formats use the same {} convention",
                convention_name
            )),
        });
    }
}

/// Check if a convention is missing from some formats when present in a majority.
fn check_convention_missing(
    conventions_by_format: &HashMap<&str, ExtractedConventions>,
    convention_name: &str,
    has_convention: fn(&ExtractedConventions) -> bool,
    errors: &mut Vec<ValidationError>,
) {
    let total = conventions_by_format.len();
    let with_convention: Vec<&&str> = conventions_by_format
        .iter()
        .filter(|(_, c)| has_convention(c))
        .map(|(f, _)| f)
        .collect();

    let without_convention: Vec<&&str> = conventions_by_format
        .iter()
        .filter(|(_, c)| !has_convention(c))
        .map(|(f, _)| f)
        .collect();

    // Flag missing if majority (>50%) have it but some don't
    if with_convention.len() > total / 2 && !without_convention.is_empty() {
        let missing_formats: Vec<String> =
            without_convention.iter().map(|f| f.to_string()).collect();
        errors.push(ValidationError {
            layer: ValidationLayer::Semantic,
            message: format!(
                "Cross-format inconsistency: {} specified in most formats but missing from [{}]",
                convention_name,
                missing_formats.join(", ")
            ),
            location: None,
            suggestion: Some(format!(
                "Add {} convention to all format outputs for consistency",
                convention_name
            )),
        });
    }
}

/// Reality check: verify rules reference actual languages/frameworks in codebase.
fn validate_reality(
    content: &str,
    codebase: &CompressedCodebase,
    warnings: &mut Vec<ValidationWarning>,
) {
    let content_lower = content.to_lowercase();

    // Collect languages present in the codebase
    let codebase_extensions: Vec<String> = codebase
        .files
        .iter()
        .filter_map(|f| {
            f.path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
        })
        .collect();

    // Check for language references that don't match codebase
    let language_checks = [
        ("typescript", &["ts", "tsx"] as &[&str]),
        ("javascript", &["js", "jsx"]),
        ("python", &["py"]),
        ("rust", &["rs"]),
        ("go", &["go"]),
        ("java", &["java"]),
        ("ruby", &["rb"]),
        ("swift", &["swift"]),
        ("kotlin", &["kt", "kts"]),
        ("c#", &["cs"]),
        ("php", &["php"]),
    ];

    for (language, extensions) in &language_checks {
        // Only warn if the rules prominently reference a language not in the codebase
        // (checking for the word appearing as a prominent reference, not just in passing)
        let pattern = format!(r"\b{}\b", regex::escape(language));
        if let Ok(re) = regex::Regex::new(&pattern) {
            let match_count = re.find_iter(&content_lower).count();
            // Only flag if referenced multiple times (suggests it's a key language in the rules)
            if match_count >= 3
                && !extensions
                    .iter()
                    .any(|ext| codebase_extensions.contains(&ext.to_string()))
            {
                warnings.push(ValidationWarning {
                    layer: ValidationLayer::Semantic,
                    message: format!(
                        "Rules prominently reference \"{}\" but no {} files found in codebase",
                        language,
                        extensions.join("/")
                    ),
                    location: None,
                    suggestion: Some(
                        "Verify that language references match the actual codebase".to_string(),
                    ),
                });
            }
        }
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Get the appropriate validator for a given format name.
pub fn get_validator(format: &str) -> Result<Box<dyn FormatValidator>> {
    match format.to_lowercase().as_str() {
        "cursor" => Ok(Box::new(CursorValidator)),
        "claude" => Ok(Box::new(ClaudeValidator)),
        "copilot" => Ok(Box::new(CopilotValidator)),
        "windsurf" => Ok(Box::new(WindsurfValidator)),
        "aider" => Ok(Box::new(AiderValidator)),
        "generic" => Ok(Box::new(GenericValidator)),
        "json" => Ok(Box::new(JsonValidator)),
        _ => Err(anyhow::anyhow!("Unknown format for validation: {}", format)),
    }
}

/// Validate all requested formats and return results.
///
/// Renders the output for each format using the formatter, then validates the rendered content.
/// When `check_consistency` is enabled for any format, also performs cross-format consistency
/// checks to ensure core conventions do not conflict or go missing across formats.
pub fn validate_all_formats(
    rules: &crate::generator::GeneratedRules,
    formats: &[String],
    config: &crate::cli::config::ValidationConfig,
    codebase: &CompressedCodebase,
    project_name: &str,
) -> Result<Vec<ValidationResult>> {
    let mut results = Vec::new();
    let mut rendered_outputs: HashMap<String, String> = HashMap::new();

    // Phase 1: Render and validate each format individually
    for format in formats {
        // Get the formatter to render output
        let formatter = crate::output::get_formatter(format)?;
        let metadata = crate::output::Metadata {
            project_name: project_name.to_string(),
            format: format.clone(),
        };

        // Render the output (what would be written to disk)
        let rendered = match formatter.format(rules, &metadata) {
            Ok(content) => content,
            Err(e) => {
                results.push(ValidationResult::from_checks(
                    format,
                    vec![ValidationError {
                        layer: ValidationLayer::Syntax,
                        message: format!("Failed to render {} format: {}", format, e),
                        location: None,
                        suggestion: Some(
                            "Check that format-specific rules were generated".to_string(),
                        ),
                    }],
                    Vec::new(),
                ));
                continue;
            }
        };

        // Store rendered output for cross-format checks
        rendered_outputs.insert(format.clone(), rendered.clone());

        // Get the validator and validate
        let validator = get_validator(format)?;
        let semantic_config = config.semantic_for_format(format);
        let result = validator.validate(&rendered, semantic_config, codebase)?;
        results.push(result);
    }

    // Phase 2: Cross-format consistency check
    let consistency_enabled = formats
        .iter()
        .any(|f| config.semantic_for_format(f).check_consistency);

    if consistency_enabled && rendered_outputs.len() >= 2 {
        let mut cross_format_errors = Vec::new();
        validate_cross_format_consistency(&rendered_outputs, &mut cross_format_errors);

        if !cross_format_errors.is_empty() {
            results.push(ValidationResult::from_checks(
                "cross-format",
                cross_format_errors,
                Vec::new(),
            ));
        }
    }

    Ok(results)
}

/// Display a formatted validation report to the user.
pub fn display_validation_report(results: &[ValidationResult], quiet: bool) {
    if quiet {
        return;
    }

    let failed_count = results.iter().filter(|r| !r.passed).count();
    let total = results.len();

    println!();
    println!("Validation Report");
    println!("=================");

    for result in results {
        let status = if result.passed { "PASSED" } else { "FAILED" };
        println!();
        println!("{} ({}):", result.format, status);

        // Display errors
        for error in &result.errors {
            println!("  \u{2717} {}", error);
        }

        // Display warnings
        for warning in &result.warnings {
            println!("  \u{26a0} [{}] {}", warning.layer, warning.message);
            if let Some(ref sug) = warning.suggestion {
                println!("      Suggestion: {}", sug);
            }
        }

        if result.errors.is_empty() && result.warnings.is_empty() {
            println!("  \u{2713} All checks passed");
        }
    }

    println!();
    if failed_count > 0 {
        println!(
            "Summary: {} of {} format(s) failed validation",
            failed_count, total
        );
    } else {
        println!("Summary: All {} format(s) passed validation", total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packer::{CodebaseMetadata, CompressedCodebase, CompressedFile, CompressionMethod};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_codebase() -> CompressedCodebase {
        CompressedCodebase {
            files: vec![
                CompressedFile {
                    path: PathBuf::from("src/main.rs"),
                    original_content: String::new(),
                    compressed_content: String::new(),
                    compression_method: CompressionMethod::None,
                    original_size: 100,
                    compressed_size: 100,
                    language: None,
                },
                CompressedFile {
                    path: PathBuf::from("src/lib.rs"),
                    original_content: String::new(),
                    compressed_content: String::new(),
                    compression_method: CompressionMethod::None,
                    original_size: 200,
                    compressed_size: 200,
                    language: None,
                },
            ],
            metadata: CodebaseMetadata {
                total_files: 2,
                total_original_size: 300,
                total_compressed_size: 300,
                languages: HashMap::new(),
                compression_ratio: 1.0,
            },
        }
    }

    fn default_semantic_config() -> SemanticValidationConfig {
        SemanticValidationConfig::default()
    }

    #[test]
    fn test_extract_file_paths() {
        let content = "Use `src/main.rs` and `src/lib.rs` for entry points.";
        let paths = extract_file_paths(content);
        assert!(paths.contains(&"src/main.rs".to_string()));
        assert!(paths.contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn test_extract_file_paths_ignores_urls() {
        let content = "Visit https://example.com for more info about src/main.rs.";
        let paths = extract_file_paths(content);
        assert!(!paths.iter().any(|p| p.contains("example.com")));
    }

    #[test]
    fn test_detect_contradictions_tabs_vs_spaces() {
        let content = "Always use tabs for indentation. Use spaces for alignment.";
        let contradictions = detect_contradictions(content);
        assert_eq!(contradictions.len(), 1);
        assert!(contradictions[0].contains("indentation"));
    }

    #[test]
    fn test_detect_contradictions_none() {
        let content = "Use spaces for indentation. Use 4-space indent width.";
        let contradictions = detect_contradictions(content);
        assert!(contradictions.is_empty());
    }

    #[test]
    fn test_matches_any_file() {
        let codebase = test_codebase();
        assert!(matches_any_file("src/main.rs", &codebase));
        assert!(!matches_any_file("src/missing.rs", &codebase));
    }

    #[test]
    fn test_json_validator_valid() {
        let validator = JsonValidator;
        let content = r#"{"rules": [{"name": "test"}]}"#;
        let config = default_semantic_config();
        let codebase = test_codebase();

        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_json_validator_invalid_syntax() {
        let validator = JsonValidator;
        let content = r#"{"rules": [}"#;
        let config = default_semantic_config();
        let codebase = test_codebase();

        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Syntax)
        );
    }

    #[test]
    fn test_claude_validator_missing_headings() {
        let validator = ClaudeValidator;
        let content = "Just some text without any headings at all.";
        let config = default_semantic_config();
        let codebase = test_codebase();

        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Schema)
        );
    }

    #[test]
    fn test_claude_validator_with_headings() {
        let validator = ClaudeValidator;
        let content = "# Project Rules\n\n## Coding Standards\n\nUse consistent formatting.";
        let config = default_semantic_config();
        let codebase = test_codebase();

        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_markdown_unclosed_code_block() {
        let validator = GenericValidator;
        let content = "# Rules\n\n```rust\nfn main() {}\n";
        let config = default_semantic_config();
        let codebase = test_codebase();

        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("code block"))
        );
    }

    #[test]
    fn test_validation_result_pass() {
        let result = ValidationResult::pass("cursor");
        assert!(result.passed);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_is_common_non_path() {
        assert!(is_common_non_path("https://example.com"));
        assert!(is_common_non_path("v1.0.0"));
        assert!(!is_common_non_path("src/main.rs"));
    }

    #[test]
    fn test_extract_conventions_indentation() {
        let content = "Always use spaces for indentation with 4 space indent.";
        let conventions = extract_conventions(content);
        assert_eq!(conventions.indentation_style.as_deref(), Some("spaces"));
        assert_eq!(conventions.indent_width, Some(4));
    }

    #[test]
    fn test_extract_conventions_naming() {
        let content = "Use camelCase for variables and snake_case for file names.";
        let conventions = extract_conventions(content);
        assert!(
            conventions
                .naming_conventions
                .contains(&"camelCase".to_string())
        );
        assert!(
            conventions
                .naming_conventions
                .contains(&"snake_case".to_string())
        );
    }

    #[test]
    fn test_extract_conventions_semicolons() {
        let content = "Always use semicolons at the end of statements.";
        let conventions = extract_conventions(content);
        assert_eq!(conventions.semicolons.as_deref(), Some("use"));
    }

    #[test]
    fn test_cross_format_consistency_conflict() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "cursor".to_string(),
            "Use tabs for indentation.".to_string(),
        );
        outputs.insert(
            "claude".to_string(),
            "Use spaces for indentation.".to_string(),
        );

        let mut errors = Vec::new();
        validate_cross_format_consistency(&outputs, &mut errors);

        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("Cross-format conflict")
                    && e.message.contains("indentation style"))
        );
    }

    #[test]
    fn test_cross_format_consistency_no_conflict() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "cursor".to_string(),
            "Use spaces for indentation.".to_string(),
        );
        outputs.insert(
            "claude".to_string(),
            "Use spaces for indentation.".to_string(),
        );

        let mut errors = Vec::new();
        validate_cross_format_consistency(&outputs, &mut errors);

        // No conflict errors for indentation
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("indentation style") && e.message.contains("conflict"))
        );
    }

    #[test]
    fn test_cross_format_consistency_missing() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "cursor".to_string(),
            "Use spaces for indentation.".to_string(),
        );
        outputs.insert(
            "claude".to_string(),
            "Use spaces for indentation.".to_string(),
        );
        outputs.insert("copilot".to_string(), "Write clean code.".to_string());

        let mut errors = Vec::new();
        validate_cross_format_consistency(&outputs, &mut errors);

        assert!(errors.iter().any(|e| e.message.contains("missing from")));
    }

    #[test]
    fn test_cross_format_consistency_single_format() {
        let mut outputs = HashMap::new();
        outputs.insert("cursor".to_string(), "Use tabs.".to_string());

        let mut errors = Vec::new();
        validate_cross_format_consistency(&outputs, &mut errors);

        // No checks for single format
        assert!(errors.is_empty());
    }
}
