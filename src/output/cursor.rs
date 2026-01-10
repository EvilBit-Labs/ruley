use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct CursorFormatter;

impl OutputFormatter for CursorFormatter {
    fn format(&self, _rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        // TODO: Implement Cursor .mdc format
        todo!("Cursor formatter not yet implemented")
    }

    fn extension(&self) -> &str {
        "mdc"
    }
}
