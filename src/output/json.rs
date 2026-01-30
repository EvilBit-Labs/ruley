//! JSON output formatter.
//!
//! Generates structured JSON output of the generated rules.
//! Useful for programmatic consumption or further processing.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for JSON output.
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        serde_json::to_string_pretty(rules).map_err(|e| RuleyError::OutputFormat(e.to_string()))
    }

    fn extension(&self) -> &str {
        "json"
    }

    fn default_filename(&self) -> &str {
        "ruley-output"
    }
}
