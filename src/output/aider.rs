//! Aider output formatter.
//!
//! Generates CONVENTIONS.md files for Aider's conventions system.
//! File is placed in the project root.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for Aider conventions.
pub struct AiderFormatter;

impl OutputFormatter for AiderFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        // Get the pre-formatted content for Aider format
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
        "CONVENTIONS"
    }
}
