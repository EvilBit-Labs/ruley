use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct AiderFormatter;

impl OutputFormatter for AiderFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement Aider format
        todo!("Aider formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "md"
    }
}
