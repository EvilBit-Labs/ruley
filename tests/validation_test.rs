// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for the three-layer validation system.
//!
//! Tests syntax, schema, and semantic validation across all output formats.
//! Uses real validators with representative content samples.

use ruley::cli::config::{FormatValidationOverrides, SemanticValidationConfig, ValidationConfig};
use ruley::packer::{CodebaseMetadata, CompressedCodebase, CompressedFile, CompressionMethod};
use ruley::utils::validation::{ValidationLayer, get_validator};
use std::collections::HashMap;
use std::path::PathBuf;

/// Creates a test codebase with Rust source files for validation.
fn test_codebase() -> CompressedCodebase {
    CompressedCodebase {
        files: vec![
            CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() {}".to_string(),
                compressed_content: "fn main() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 12,
                compressed_size: 12,
                language: None,
            },
            CompressedFile {
                path: PathBuf::from("src/lib.rs"),
                original_content: "pub fn greet() {}".to_string(),
                compressed_content: "pub fn greet() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 17,
                compressed_size: 17,
                language: None,
            },
            CompressedFile {
                path: PathBuf::from("src/utils.rs"),
                original_content: String::new(),
                compressed_content: String::new(),
                compression_method: CompressionMethod::None,
                original_size: 0,
                compressed_size: 0,
                language: None,
            },
        ],
        metadata: CodebaseMetadata {
            total_files: 3,
            total_original_size: 29,
            total_compressed_size: 29,
            languages: HashMap::new(),
            compression_ratio: 1.0,
        },
    }
}

fn default_config() -> SemanticValidationConfig {
    SemanticValidationConfig::default()
}

mod syntax_validation {
    use super::*;

    /// Test that empty content fails syntax validation.
    #[test]
    fn test_empty_content_fails_syntax() {
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Syntax)
        );
    }

    /// Test that unclosed code blocks are detected.
    #[test]
    fn test_unclosed_code_block_detected() {
        let content = "# Rules\n\n```rust\nfn main() {}\n";
        let validator = get_validator("generic").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(content, &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("code block")),
            "Should detect unclosed code block"
        );
    }

    /// Test that properly closed code blocks pass.
    #[test]
    fn test_closed_code_blocks_pass() {
        let content = "# Rules\n\n```rust\nfn main() {}\n```\n";
        let validator = get_validator("generic").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(content, &default_config(), &codebase)
            .unwrap();
        assert!(
            !result
                .errors
                .iter()
                .any(|e| e.message.contains("code block")),
            "Closed code blocks should not trigger error"
        );
    }

    /// Test JSON syntax validation with invalid JSON.
    #[test]
    fn test_invalid_json_fails_syntax() {
        let validator = get_validator("json").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(r#"{"rules": [}"#, &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Syntax)
        );
    }

    /// Test valid JSON passes syntax validation.
    #[test]
    fn test_valid_json_passes_syntax() {
        let validator = get_validator("json").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(r#"{"rules": ["use spaces"]}"#, &default_config(), &codebase)
            .unwrap();
        assert!(result.passed);
    }

    /// Test empty content fails syntax for all seven formats.
    #[test]
    fn test_empty_content_fails_all_formats() {
        let formats = [
            "cursor", "claude", "copilot", "windsurf", "aider", "generic", "json",
        ];
        let codebase = test_codebase();
        let config = default_config();

        for format in &formats {
            let validator = get_validator(format).unwrap();
            let result = validator.validate("", &config, &codebase).unwrap();
            assert!(
                !result.passed,
                "Empty content should fail syntax for format: {}",
                format
            );
            assert!(
                result
                    .errors
                    .iter()
                    .any(|e| e.layer == ValidationLayer::Syntax),
                "Empty content should produce Syntax error for format: {}",
                format
            );
        }
    }

    /// Test unclosed code blocks fail for all Markdown-based formats.
    #[test]
    fn test_unclosed_code_block_all_markdown_formats() {
        let markdown_formats = [
            "cursor", "claude", "copilot", "windsurf", "aider", "generic",
        ];
        let content = "# Rules\n\n```python\ndef foo():\n    pass\n";
        let codebase = test_codebase();
        let config = default_config();

        for format in &markdown_formats {
            let validator = get_validator(format).unwrap();
            let result = validator.validate(content, &config, &codebase).unwrap();
            assert!(
                result
                    .errors
                    .iter()
                    .any(|e| e.message.contains("code block")),
                "Unclosed code block should be detected for format: {}",
                format
            );
        }
    }

    /// Test JSON null value fails schema.
    #[test]
    fn test_json_null_fails_schema() {
        let validator = get_validator("json").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("null", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Schema)
        );
    }
}

mod schema_validation {
    use super::*;

    /// Test that Claude format requires section headings.
    #[test]
    fn test_claude_requires_headings() {
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(
                "Just plain text without any headings.",
                &default_config(),
                &codebase,
            )
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Schema),
            "Claude format should require headings"
        );
    }

    /// Test that Claude format with headings passes schema validation.
    #[test]
    fn test_claude_with_headings_passes() {
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(
                "# Project Rules\n\n## Coding Standards\n\nUse consistent formatting.",
                &default_config(),
                &codebase,
            )
            .unwrap();
        assert!(result.passed);
    }

    /// Test Cursor format with unclosed frontmatter.
    #[test]
    fn test_cursor_unclosed_frontmatter() {
        let content = "---\ndescription: test\n\n# Rules\n\nSome content";
        let validator = get_validator("cursor").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(content, &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.message.contains("frontmatter")),
            "Should detect unclosed frontmatter"
        );
    }

    /// Test Cursor format with valid frontmatter.
    #[test]
    fn test_cursor_valid_frontmatter() {
        let content =
            "---\ndescription: Project rules\nalwaysApply: true\n---\n\n# Rules\n\nUse spaces.";
        let validator = get_validator("cursor").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate(content, &default_config(), &codebase)
            .unwrap();
        assert!(
            result.passed,
            "Valid Cursor frontmatter should pass: {:?}",
            result.errors
        );
    }

    /// Test that empty JSON object fails schema.
    #[test]
    fn test_json_empty_object_fails() {
        let validator = get_validator("json").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("{}", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Schema)
        );
    }

    /// Test that copilot format rejects empty content.
    #[test]
    fn test_copilot_empty_content_fails() {
        let validator = get_validator("copilot").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("   ", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
    }

    /// Test schema validation for all seven formats with valid content.
    #[test]
    fn test_all_seven_formats_schema_pass() {
        let codebase = test_codebase();
        let config = default_config();

        let valid_content = [
            (
                "cursor",
                "---\ndescription: Rules\nalwaysApply: true\n---\n\n# Rules\n\nUse spaces.\n",
            ),
            ("claude", "# Project Rules\n\n## Standards\n\nUse spaces.\n"),
            (
                "copilot",
                "# Copilot Instructions\n\nUse consistent naming.\n",
            ),
            ("windsurf", "# Windsurf Rules\n\nFollow conventions.\n"),
            ("aider", "# Conventions\n\nUse consistent formatting.\n"),
            ("generic", "# AI Rules\n\nUse proper indentation.\n"),
            ("json", r#"{"rules": ["Use consistent formatting"]}"#),
        ];

        for (format, content) in &valid_content {
            let validator = get_validator(format).unwrap();
            let result = validator.validate(content, &config, &codebase).unwrap();
            assert!(
                result.passed,
                "Valid content should pass schema for format '{}': {:?}",
                format, result.errors
            );
        }
    }

    /// Test Windsurf empty content fails schema.
    #[test]
    fn test_windsurf_empty_content_fails() {
        let validator = get_validator("windsurf").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("   ", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
    }

    /// Test Aider empty content fails schema.
    #[test]
    fn test_aider_empty_content_fails() {
        let validator = get_validator("aider").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("  ", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
    }

    /// Test Generic empty content fails schema.
    #[test]
    fn test_generic_empty_content_fails() {
        let validator = get_validator("generic").unwrap();
        let codebase = test_codebase();
        let result = validator
            .validate("  ", &default_config(), &codebase)
            .unwrap();
        assert!(!result.passed);
    }
}

mod semantic_validation {
    use super::*;

    /// Test contradiction detection: tabs vs spaces.
    #[test]
    fn test_detects_tabs_vs_spaces_contradiction() {
        let content = "# Rules\n\nUse tabs for indentation.\nAlways use spaces for alignment.";
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Semantic),
            "Should detect tabs vs spaces contradiction"
        );
    }

    /// Test no contradiction when only spaces are mentioned.
    #[test]
    fn test_no_contradiction_single_style() {
        let content = "# Rules\n\nUse spaces for indentation. Use 4 space indent width.";
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(
            !result.errors.iter().any(|e| {
                e.layer == ValidationLayer::Semantic && e.message.contains("Contradictory")
            }),
            "Should not detect contradictions with consistent style"
        );
    }

    /// Test file path validation against codebase.
    #[test]
    fn test_file_path_warnings() {
        let content = "# Rules\n\n## File References\n\nSee `src/main.rs` and `src/nonexistent.rs` for details.";
        let config = SemanticValidationConfig {
            check_file_paths: true,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator.validate(content, &config, &codebase).unwrap();

        // src/nonexistent.rs should trigger a warning
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.message.contains("nonexistent.rs")),
            "Should warn about non-existent file path"
        );
    }

    /// Test that existing file paths don't trigger warnings.
    #[test]
    fn test_existing_file_paths_no_warning() {
        let content = "# Rules\n\n## Files\n\nSee `src/main.rs` for entry point.";
        let config = SemanticValidationConfig {
            check_file_paths: true,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("claude").unwrap();
        let codebase = test_codebase();
        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(
            !result
                .warnings
                .iter()
                .any(|w| w.message.contains("main.rs")),
            "Existing file paths should not trigger warnings"
        );
    }

    /// Test get_validator for all supported formats.
    #[test]
    fn test_get_validator_all_formats() {
        let formats = [
            "cursor", "claude", "copilot", "windsurf", "aider", "generic", "json",
        ];
        for format in &formats {
            let result = get_validator(format);
            assert!(
                result.is_ok(),
                "Should get validator for format: {}",
                format
            );
        }
    }

    /// Test get_validator for unknown format.
    #[test]
    fn test_get_validator_unknown_format() {
        let result = get_validator("nonexistent");
        assert!(result.is_err());
    }

    /// Test file path existence check for all seven formats.
    #[test]
    fn test_file_path_existence_all_formats() {
        let config = SemanticValidationConfig {
            check_file_paths: true,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let codebase = test_codebase();
        let content_with_bad_path = [
            (
                "claude",
                "# Rules\n\n## Refs\n\nSee `src/missing.rs` for details.",
            ),
            (
                "copilot",
                "# Instructions\n\nSee `src/missing.rs` for details.",
            ),
            ("windsurf", "# Rules\n\nSee `src/missing.rs` for details."),
            (
                "aider",
                "# Conventions\n\nSee `src/missing.rs` for details.",
            ),
            ("generic", "# Rules\n\nSee `src/missing.rs` for details."),
            (
                "cursor",
                "---\ndescription: Rules\n---\n\n# Rules\n\nSee `src/missing.rs`.",
            ),
        ];

        for (format, content) in &content_with_bad_path {
            let validator = get_validator(format).unwrap();
            let result = validator.validate(content, &config, &codebase).unwrap();
            assert!(
                result
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("missing.rs")),
                "File path warning should be raised for format: {}",
                format
            );
        }
    }

    /// Test contradiction detection across formats.
    #[test]
    fn test_contradiction_detection_across_formats() {
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };
        let codebase = test_codebase();
        let contradictory =
            "# Rules\n\nUse tabs for indentation.\nAlways use spaces for indentation.";

        let formats = ["claude", "copilot", "windsurf", "aider", "generic"];
        for format in &formats {
            let validator = get_validator(format).unwrap();
            let result = validator
                .validate(contradictory, &config, &codebase)
                .unwrap();
            assert!(
                result
                    .errors
                    .iter()
                    .any(|e| e.layer == ValidationLayer::Semantic),
                "Contradiction should be detected for format: {}",
                format
            );
        }
    }

    /// Test reality/tooling alignment: warns when rules reference a language not in codebase.
    #[test]
    fn test_reality_alignment_warns_on_missing_language() {
        let config = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: false,
            check_reality: true,
        };
        // Codebase has only .rs files
        let codebase = test_codebase();
        // Content prominently references typescript (3+ times triggers warning)
        let content = "# Rules\n\n## TypeScript Standards\n\nAll typescript code must follow typescript conventions. Use typescript strict mode.";
        let validator = get_validator("claude").unwrap();
        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(
            result.warnings.iter().any(|w| {
                w.layer == ValidationLayer::Semantic && w.message.contains("typescript")
            }),
            "Should warn about typescript not being in codebase: {:?}",
            result.warnings
        );
    }

    /// Test reality check does NOT warn when referenced language is in codebase.
    #[test]
    fn test_reality_alignment_no_warn_for_present_language() {
        let config = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: false,
            check_reality: true,
        };
        let codebase = test_codebase(); // Has .rs files
        let content = "# Rules\n\n## Rust Standards\n\nAll rust code must follow rust conventions. Use rust 2024 edition.";
        let validator = get_validator("claude").unwrap();
        let result = validator.validate(content, &config, &codebase).unwrap();
        assert!(
            !result
                .warnings
                .iter()
                .any(|w| { w.layer == ValidationLayer::Semantic && w.message.contains("rust") }),
            "Should not warn about rust when .rs files exist: {:?}",
            result.warnings
        );
    }
}

mod semantic_config_toggles {
    use super::*;

    /// Test that disabling check_contradictions suppresses contradiction errors.
    #[test]
    fn test_disable_contradictions_check() {
        let config = SemanticValidationConfig {
            check_contradictions: false,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };
        let codebase = test_codebase();
        let contradictory =
            "# Rules\n\nUse tabs for indentation.\nAlways use spaces for indentation.";
        let validator = get_validator("claude").unwrap();
        let result = validator
            .validate(contradictory, &config, &codebase)
            .unwrap();

        assert!(
            !result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Semantic),
            "Contradictions should not be flagged when check_contradictions is false"
        );
    }

    /// Test that disabling check_file_paths suppresses file path warnings.
    #[test]
    fn test_disable_file_paths_check() {
        let config = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let codebase = test_codebase();
        let content = "# Rules\n\n## Refs\n\nSee `src/missing.rs` and `src/gone.rs` for details.";
        let validator = get_validator("claude").unwrap();
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            !result
                .warnings
                .iter()
                .any(|w| w.message.contains("missing.rs")),
            "File path warnings should not appear when check_file_paths is false"
        );
    }

    /// Test that disabling check_reality suppresses reality warnings.
    #[test]
    fn test_disable_reality_check() {
        let config = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let codebase = test_codebase();
        let content = "# Rules\n\n## Python Standards\n\nAll python code must follow python conventions. Use python type hints.";
        let validator = get_validator("claude").unwrap();
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            !result
                .warnings
                .iter()
                .any(|w| { w.layer == ValidationLayer::Semantic && w.message.contains("python") }),
            "Reality warnings should not appear when check_reality is false"
        );
    }

    /// Test that each check can be independently enabled.
    #[test]
    fn test_independent_check_enablement() {
        let codebase = test_codebase();
        let content = "# Rules\n\nUse tabs for indentation.\nUse spaces for alignment.\nSee `src/missing.rs`.";

        // Only contradictions enabled
        let config_contradictions = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("claude").unwrap();
        let result = validator
            .validate(content, &config_contradictions, &codebase)
            .unwrap();
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Semantic),
            "Only contradiction check should fire"
        );
        assert!(
            result.warnings.is_empty(),
            "No file path warnings when check_file_paths is false"
        );

        // Only file paths enabled
        let config_paths = SemanticValidationConfig {
            check_contradictions: false,
            check_file_paths: true,
            check_consistency: false,
            check_reality: false,
        };
        let result2 = validator
            .validate(content, &config_paths, &codebase)
            .unwrap();
        assert!(
            !result2
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Semantic),
            "No contradiction errors when check_contradictions is false"
        );
        assert!(
            result2
                .warnings
                .iter()
                .any(|w| w.message.contains("missing.rs")),
            "File path warnings should fire when enabled"
        );
    }
}

mod per_format_overrides {
    use super::*;

    /// Test per-format override disables file path checks for JSON.
    #[test]
    fn test_json_override_skips_file_paths() {
        let json_override = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: true,
            check_consistency: false,
            check_reality: false,
        };
        let overrides = FormatValidationOverrides {
            json: Some(json_override),
            ..Default::default()
        };

        let config = ValidationConfig {
            enabled: true,
            retry_on_failure: false,
            max_retries: 3,
            semantic: SemanticValidationConfig::default(),
            format_overrides: overrides,
        };

        // JSON format should use the override
        let json_semantic = config.semantic_for_format("json");
        assert!(
            !json_semantic.check_file_paths,
            "JSON should skip file path checks"
        );

        // Other formats should use the global default
        let claude_semantic = config.semantic_for_format("claude");
        assert!(
            claude_semantic.check_file_paths,
            "Claude should use global default"
        );
    }

    /// Test per-format override is honored during validation.
    #[test]
    fn test_format_override_honored_in_validation() {
        let codebase = test_codebase();
        let content = r#"{"rules": ["See src/missing.rs for details"]}"#;

        // With file path checks enabled (global)
        let config_enabled = SemanticValidationConfig {
            check_file_paths: true,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let validator = get_validator("json").unwrap();
        let result_enabled = validator
            .validate(content, &config_enabled, &codebase)
            .unwrap();

        // With file path checks disabled (override for JSON)
        let config_disabled = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };
        let result_disabled = validator
            .validate(content, &config_disabled, &codebase)
            .unwrap();

        // Enabled config should have file path warnings; disabled should not
        let has_path_warning_enabled = result_enabled
            .warnings
            .iter()
            .any(|w| w.message.contains("missing.rs"));
        let has_path_warning_disabled = result_disabled
            .warnings
            .iter()
            .any(|w| w.message.contains("missing.rs"));

        assert!(
            has_path_warning_enabled,
            "File path warning should appear when enabled"
        );
        assert!(
            !has_path_warning_disabled,
            "File path warning should not appear when disabled"
        );
    }

    /// Test semantic_for_format returns override when present, global otherwise.
    #[test]
    fn test_semantic_for_format_fallback() {
        let cursor_override = SemanticValidationConfig {
            check_file_paths: false,
            check_contradictions: false,
            check_consistency: true,
            check_reality: false,
        };

        let config = ValidationConfig {
            enabled: true,
            retry_on_failure: false,
            max_retries: 3,
            semantic: SemanticValidationConfig::default(),
            format_overrides: FormatValidationOverrides {
                cursor: Some(cursor_override),
                ..Default::default()
            },
        };

        // Cursor uses override
        let cursor_cfg = config.semantic_for_format("cursor");
        assert!(!cursor_cfg.check_file_paths);
        assert!(!cursor_cfg.check_contradictions);

        // Claude uses global default (all true)
        let claude_cfg = config.semantic_for_format("claude");
        assert!(claude_cfg.check_file_paths);
        assert!(claude_cfg.check_contradictions);
    }
}

mod validation_report {
    use super::*;

    /// Test validation report content structure.
    #[test]
    fn test_validation_result_structure() {
        let codebase = test_codebase();
        let config = default_config();

        // Generate a result with errors
        let validator = get_validator("claude").unwrap();
        let result = validator
            .validate("no headings here", &config, &codebase)
            .unwrap();

        assert_eq!(result.format, "claude");
        assert!(!result.passed);
        assert!(!result.errors.is_empty());

        // Each error has required fields
        for error in &result.errors {
            assert!(
                !error.message.is_empty(),
                "Error message should not be empty"
            );
            // Layer should be one of Syntax, Schema, Semantic
            match error.layer {
                ValidationLayer::Syntax | ValidationLayer::Schema | ValidationLayer::Semantic => {}
            }
        }
    }

    /// Test validation report includes warnings.
    #[test]
    fn test_validation_result_includes_warnings() {
        let codebase = test_codebase();
        let config = SemanticValidationConfig {
            check_file_paths: true,
            check_contradictions: false,
            check_consistency: false,
            check_reality: false,
        };

        let content = "# Rules\n\n## Refs\n\nSee `src/nonexistent.rs` for details.";
        let validator = get_validator("claude").unwrap();
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(!result.warnings.is_empty(), "Should have warnings");
        for warning in &result.warnings {
            assert!(
                !warning.message.is_empty(),
                "Warning message should not be empty"
            );
        }
    }

    /// Test display_validation_report does not panic.
    #[test]
    fn test_display_validation_report_no_panic() {
        use ruley::utils::validation::display_validation_report;

        let codebase = test_codebase();
        let config = default_config();

        let validator = get_validator("claude").unwrap();
        let result = validator
            .validate("no headings", &config, &codebase)
            .unwrap();

        // Should not panic even with errors/warnings
        display_validation_report(&[result], true); // quiet=true to avoid stdout noise
    }
}

mod cross_format_consistency {
    use super::*;
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::utils::validation::validate_all_formats;

    /// Test cross-format consistency detects conflicting indentation.
    #[test]
    fn test_cross_format_consistency_detects_conflict() {
        let codebase = test_codebase();
        let mut rules = GeneratedRules::new("analysis");

        // Cursor says tabs, Claude says spaces
        rules.add_format(FormattedRules::new(
            "cursor",
            "---\ndescription: Rules\nalwaysApply: true\n---\n\n# Rules\n\nUse tabs for indentation.\n",
        ));
        rules.add_format(FormattedRules::new(
            "claude",
            "# Rules\n\n## Standards\n\nUse spaces for indentation.\n",
        ));

        let config = ValidationConfig {
            enabled: true,
            retry_on_failure: false,
            max_retries: 3,
            semantic: SemanticValidationConfig {
                check_consistency: true,
                check_file_paths: false,
                check_contradictions: false,
                check_reality: false,
            },
            format_overrides: FormatValidationOverrides::default(),
        };

        let formats = vec!["cursor".to_string(), "claude".to_string()];
        let results = validate_all_formats(&rules, &formats, &config, &codebase, "test")
            .expect("Validation should succeed");

        // Should have a cross-format result with conflict errors
        let cross_format = results.iter().find(|r| r.format == "cross-format");
        assert!(
            cross_format.is_some(),
            "Should have cross-format consistency result: {:?}",
            results.iter().map(|r| &r.format).collect::<Vec<_>>()
        );
        let cf = cross_format.unwrap();
        assert!(
            cf.errors
                .iter()
                .any(|e| e.message.contains("Cross-format conflict")),
            "Should detect cross-format indentation conflict: {:?}",
            cf.errors
        );
    }

    /// Test no cross-format conflict when formats agree.
    #[test]
    fn test_cross_format_consistency_no_conflict() {
        let codebase = test_codebase();
        let mut rules = GeneratedRules::new("analysis");

        rules.add_format(FormattedRules::new(
            "cursor",
            "---\ndescription: Rules\nalwaysApply: true\n---\n\n# Rules\n\nUse spaces for indentation.\n",
        ));
        rules.add_format(FormattedRules::new(
            "claude",
            "# Rules\n\n## Standards\n\nUse spaces for indentation.\n",
        ));

        let config = ValidationConfig {
            enabled: true,
            retry_on_failure: false,
            max_retries: 3,
            semantic: SemanticValidationConfig {
                check_consistency: true,
                check_file_paths: false,
                check_contradictions: false,
                check_reality: false,
            },
            format_overrides: FormatValidationOverrides::default(),
        };

        let formats = vec!["cursor".to_string(), "claude".to_string()];
        let results = validate_all_formats(&rules, &formats, &config, &codebase, "test")
            .expect("Validation should succeed");

        // No cross-format result, or it passes
        let cross_format = results.iter().find(|r| r.format == "cross-format");
        if let Some(cf) = cross_format {
            assert!(
                !cf.errors
                    .iter()
                    .any(|e| e.message.contains("indentation style")
                        && e.message.contains("conflict")),
                "Should not have indentation conflict: {:?}",
                cf.errors
            );
        }
    }
}

mod retry_and_auto_fix {
    use super::*;
    use ruley::generator::refinement::{FixAttempt, RefinementResult};

    /// Test that ValidationConfig retry settings are correctly configured.
    #[test]
    fn test_validation_config_retry_settings() {
        let config = ValidationConfig {
            enabled: true,
            retry_on_failure: true,
            max_retries: 5,
            semantic: SemanticValidationConfig::default(),
            format_overrides: FormatValidationOverrides::default(),
        };

        assert!(config.retry_on_failure);
        assert_eq!(config.max_retries, 5);
    }

    /// Test that default config has retry disabled.
    #[test]
    fn test_default_config_retry_disabled() {
        let config = ValidationConfig::default();
        assert!(!config.retry_on_failure);
        assert_eq!(config.max_retries, 3);
    }

    /// Test RefinementResult tracks exhausted retries.
    #[test]
    fn test_refinement_exhausted_retries() {
        let result = RefinementResult {
            success: false,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["err".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec!["err".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 3,
                    errors: vec!["err".to_string()],
                    cost: 0.01,
                },
            ],
            total_cost: 0.03,
            retries_exhausted: true,
        };

        assert!(!result.success);
        assert!(result.retries_exhausted);
        assert_eq!(result.attempts.len(), 3);
    }

    /// Test RefinementResult tracks success after retry.
    #[test]
    fn test_refinement_success_after_retry() {
        let result = RefinementResult {
            success: true,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["err".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec![],
                    cost: 0.005,
                },
            ],
            total_cost: 0.015,
            retries_exhausted: false,
        };

        assert!(result.success);
        assert!(!result.retries_exhausted);
        assert_eq!(result.attempts.len(), 2);
    }

    /// Test total cost accumulates across retry attempts.
    #[test]
    fn test_retry_cost_accumulation() {
        let attempts = vec![
            FixAttempt {
                attempt_number: 1,
                errors: vec!["e1".to_string()],
                cost: 0.01,
            },
            FixAttempt {
                attempt_number: 2,
                errors: vec!["e1".to_string()],
                cost: 0.012,
            },
            FixAttempt {
                attempt_number: 3,
                errors: vec![],
                cost: 0.015,
            },
        ];
        let total: f64 = attempts.iter().map(|a| a.cost).sum();

        let result = RefinementResult {
            success: true,
            attempts,
            total_cost: total,
            retries_exhausted: false,
        };

        assert!((result.total_cost - 0.037).abs() < 0.001);
    }
}
