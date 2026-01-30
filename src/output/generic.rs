//! Generic AI assistant output formatter.
//!
//! Generates AI_RULES.md files for universal AI assistant use.
//! File is placed in the project root.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for generic AI assistant rules.
pub struct GenericFormatter;

impl OutputFormatter for GenericFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        // Get the pre-formatted content for generic format
        rules
            .get_format(&metadata.format)
            .map(|r| r.content.clone())
            .ok_or_else(|| {
                RuleyError::OutputFormat(format!(
                    "No rules generated for format '{}'. Available formats: {:?}",
                    metadata.format,
                    rules.formats().collect::<Vec<_>>()
                ))
            })
    }

    fn extension(&self) -> &str {
        "md"
    }

    fn default_filename(&self) -> &str {
        "AI_RULES"
    }
}
