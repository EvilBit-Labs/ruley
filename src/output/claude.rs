use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct ClaudeFormatter;

impl OutputFormatter for ClaudeFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement Claude Code format
        todo!("Claude formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "md"
    }
}
