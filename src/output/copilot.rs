use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct CopilotFormatter;

impl OutputFormatter for CopilotFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement GitHub Copilot format
        todo!("Copilot formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "md"
    }
}
