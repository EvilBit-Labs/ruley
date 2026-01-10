use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct WindsurfFormatter;

impl OutputFormatter for WindsurfFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement Windsurf format
        todo!("Windsurf formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "windsurfrules"
    }
}
