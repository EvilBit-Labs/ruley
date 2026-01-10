use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        serde_json::to_string_pretty(rules).map_err(|e| RuleyError::OutputFormat(e.to_string()))
    }

    fn extension(&self) -> &str {
        "json"
    }
}
