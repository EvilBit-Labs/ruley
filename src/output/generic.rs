use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct GenericFormatter;

impl OutputFormatter for GenericFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement generic AI context format
        todo!("Generic formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "md"
    }
}
