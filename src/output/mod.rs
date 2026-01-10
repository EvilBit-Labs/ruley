pub mod aider;
pub mod claude;
pub mod copilot;
pub mod cursor;
pub mod generic;
pub mod json;
pub mod windsurf;

use crate::generator::rules::GeneratedRules;
use crate::utils::error::RuleyError;

pub trait OutputFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError>;
    fn extension(&self) -> &str;
}

pub struct Metadata {
    pub project_name: String,
    pub format: String,
}
