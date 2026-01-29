//! Cursor IDE output formatter.
//!
//! Generates .mdc (Markdown Configuration) files for Cursor IDE's rules system.
//! Rules are placed in `.cursor/rules/` directory.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for Cursor IDE rules in .mdc format.
pub struct CursorFormatter;

impl OutputFormatter for CursorFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        // Get the pre-formatted content for Cursor format
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
        "mdc"
    }

    fn default_filename(&self) -> &str {
        "project"
    }

    fn default_directory(&self) -> &str {
        ".cursor/rules"
    }
}
