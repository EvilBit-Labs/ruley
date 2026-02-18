// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Windsurf IDE output formatter.
//!
//! Generates .windsurfrules files for Windsurf IDE's AI assistant.
//! File is placed in the project root.

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

/// Formatter for Windsurf IDE rules.
pub struct WindsurfFormatter;

impl OutputFormatter for WindsurfFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        // Get the pre-formatted content for Windsurf format
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
        "windsurfrules"
    }

    fn default_filename(&self) -> &str {
        ""
    }
}
