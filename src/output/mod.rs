//! Output formatters for generating IDE-specific rule files.
//!
//! This module provides the `OutputFormatter` trait and implementations for
//! various AI IDE tools:
//!
//! - **Cursor**: .mdc files in `.cursor/rules/`
//! - **Claude**: CLAUDE.md in project root
//! - **Copilot**: .github/copilot-instructions.md
//! - **Windsurf**: .windsurfrules in project root
//! - **Aider**: CONVENTIONS.md in project root
//! - **Generic**: AI_RULES.md for universal use
//! - **JSON**: Structured JSON output
//!
//! # Example
//!
//! ```ignore
//! use ruley::output::{OutputFormatter, CursorFormatter, Metadata};
//!
//! let formatter = CursorFormatter;
//! let metadata = Metadata {
//!     project_name: "my-project".to_string(),
//!     format: "cursor".to_string(),
//! };
//! let content = formatter.format(&rules, &metadata)?;
//! ```

pub mod aider;
pub mod claude;
pub mod copilot;
pub mod cursor;
pub mod generic;
pub mod json;
pub mod windsurf;
mod writer;

pub use aider::AiderFormatter;
pub use claude::ClaudeFormatter;
pub use copilot::CopilotFormatter;
pub use cursor::CursorFormatter;
pub use generic::GenericFormatter;
pub use json::JsonFormatter;
pub use windsurf::WindsurfFormatter;
pub use writer::{ConflictStrategy, OutputResult, WriteOptions, write_output};

use crate::generator::rules::GeneratedRules;
use crate::utils::error::RuleyError;

/// Trait for formatting generated rules into specific IDE formats.
pub trait OutputFormatter {
    /// Format the generated rules into the target format.
    ///
    /// # Arguments
    ///
    /// * `rules` - The generated rules containing format-specific content
    /// * `metadata` - Metadata about the output (project name, format)
    ///
    /// # Returns
    ///
    /// The formatted content as a string, ready to be written to a file.
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError>;

    /// Get the file extension for this format (without the leading dot).
    fn extension(&self) -> &str;

    /// Get the default filename (without extension) for this format.
    fn default_filename(&self) -> &str {
        "rules"
    }

    /// Get the default directory for this format, relative to project root.
    ///
    /// Returns an empty string if the file should be in the project root.
    fn default_directory(&self) -> &str {
        ""
    }
}

/// Metadata about the output being generated.
#[derive(Debug, Clone)]
pub struct Metadata {
    /// Name of the project
    pub project_name: String,
    /// Format being generated (e.g., "cursor", "claude")
    pub format: String,
}

/// Get the appropriate formatter for a given format name.
///
/// # Arguments
///
/// * `format` - The format name (e.g., "cursor", "claude", "copilot")
///
/// # Returns
///
/// A boxed formatter implementing `OutputFormatter`, or an error if the format is unknown.
pub fn get_formatter(format: &str) -> Result<Box<dyn OutputFormatter>, RuleyError> {
    match format.to_lowercase().as_str() {
        "cursor" => Ok(Box::new(CursorFormatter)),
        "claude" => Ok(Box::new(ClaudeFormatter)),
        "copilot" => Ok(Box::new(CopilotFormatter)),
        "windsurf" => Ok(Box::new(WindsurfFormatter)),
        "aider" => Ok(Box::new(AiderFormatter)),
        "generic" => Ok(Box::new(GenericFormatter)),
        "json" => Ok(Box::new(JsonFormatter)),
        _ => Err(RuleyError::invalid_format(format)),
    }
}
