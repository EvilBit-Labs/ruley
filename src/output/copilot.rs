// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! GitHub Copilot output formatter.
//!
//! Generates copilot-instructions.md files for GitHub Copilot.
//! File is placed in the `.github/` directory.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for GitHub Copilot instructions.
pub struct CopilotFormatter;

impl OutputFormatter for CopilotFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        // Get the pre-formatted content for Copilot format
        rules
            .get_format(&metadata.format)
            .map(|r| r.content.clone())
            .ok_or_else(|| {
                RuleyError::OutputFormat(format!(
                    "No rules generated for format '{}'. Available formats: {:?}",
                    metadata.format,
                    rules.formats().collect::<Vec<_>>()
                ))
            })
    }

    fn extension(&self) -> &str {
        "md"
    }

    fn default_filename(&self) -> &str {
        "copilot-instructions"
    }

    fn default_directory(&self) -> &str {
        ".github"
    }
}
