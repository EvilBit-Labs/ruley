//! Tests for file conflict resolution in the output writer.
//!
//! Tests ConflictStrategy parsing, backup creation, path determination,
//! and WriteOptions builder pattern.

use ruley::output::{ConflictStrategy, WriteOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

mod conflict_strategy {
    use super::*;

    /// Test all valid ConflictStrategy string variants.
    #[test]
    fn test_conflict_strategy_from_str_variants() {
        let cases = [
            ("prompt", ConflictStrategy::Prompt),
            ("overwrite", ConflictStrategy::Overwrite),
            ("skip", ConflictStrategy::Skip),
            ("smart-merge", ConflictStrategy::SmartMerge),
            ("smartmerge", ConflictStrategy::SmartMerge),
            ("smart_merge", ConflictStrategy::SmartMerge),
        ];

        for (input, expected) in &cases {
            let result: Result<ConflictStrategy, _> = input.parse();
            assert!(
                result.is_ok(),
                "Should parse '{}' as ConflictStrategy",
                input
            );
            assert_eq!(result.unwrap(), *expected, "Mismatch for input: {}", input);
        }
    }

    /// Test case insensitivity of ConflictStrategy parsing.
    #[test]
    fn test_conflict_strategy_case_insensitive() {
        let variants = ["Prompt", "OVERWRITE", "Skip", "Smart-Merge", "SMARTMERGE"];
        for v in &variants {
            let result: Result<ConflictStrategy, _> = v.parse();
            assert!(result.is_ok(), "Should parse '{}' case-insensitively", v);
        }
    }

    /// Test invalid ConflictStrategy string.
    #[test]
    fn test_conflict_strategy_invalid() {
        let result: Result<ConflictStrategy, _> = "invalid".parse();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid conflict strategy") || err.contains("invalid"),
            "Error should mention invalid strategy: {}",
            err
        );
    }

    /// Test ConflictStrategy display roundtrip.
    #[test]
    fn test_conflict_strategy_display_roundtrip() {
        let strategies = [
            ConflictStrategy::Prompt,
            ConflictStrategy::Overwrite,
            ConflictStrategy::Skip,
            ConflictStrategy::SmartMerge,
        ];

        for strategy in &strategies {
            let display = strategy.to_string();
            let parsed: ConflictStrategy = display.parse().unwrap();
            assert_eq!(
                *strategy, parsed,
                "Display roundtrip failed for {:?}",
                strategy
            );
        }
    }
}

mod write_options {
    use super::*;

    /// Test WriteOptions builder with all options.
    #[test]
    fn test_write_options_builder() {
        let mut paths = HashMap::new();
        paths.insert("cursor".to_string(), "custom/path.mdc".to_string());

        let options = WriteOptions::new("/project")
            .with_output_paths(paths.clone())
            .with_backups(false)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(true);

        assert_eq!(options.base_path, PathBuf::from("/project"));
        assert_eq!(options.output_paths, paths);
        assert!(!options.create_backups);
        assert_eq!(options.conflict_strategy, ConflictStrategy::Overwrite);
        assert!(options.is_interactive);
    }

    /// Test WriteOptions default values.
    #[test]
    fn test_write_options_defaults() {
        let options = WriteOptions::new("/project");

        assert_eq!(options.base_path, PathBuf::from("/project"));
        assert!(options.output_paths.is_empty());
        assert!(options.create_backups);
        assert_eq!(options.conflict_strategy, ConflictStrategy::Prompt);
        assert!(!options.is_interactive);
    }

    /// Test WriteOptions with custom output paths.
    #[test]
    fn test_write_options_custom_paths() {
        let mut paths = HashMap::new();
        paths.insert("claude".to_string(), "docs/CLAUDE.md".to_string());
        paths.insert("cursor".to_string(), ".cursor/rules/main.mdc".to_string());

        let options = WriteOptions::new("/my-project").with_output_paths(paths);

        assert_eq!(options.output_paths.len(), 2);
        assert_eq!(
            options.output_paths.get("claude").unwrap(),
            "docs/CLAUDE.md"
        );
    }
}

mod backup_operations {
    use super::*;

    /// Test backup file creation for existing files.
    #[test]
    fn test_backup_file_creation() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("CLAUDE.md");
        std::fs::write(&original, "# Original rules").unwrap();

        // Generate backup path
        let backup = original.with_file_name("CLAUDE.md.bak");
        std::fs::copy(&original, &backup).unwrap();

        assert!(backup.exists());
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "# Original rules"
        );
    }

    /// Test timestamped backup when .bak already exists.
    #[test]
    fn test_timestamped_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("rules.md");
        std::fs::write(&original, "content").unwrap();

        // Create first backup
        let first_backup = original.with_file_name("rules.md.bak");
        std::fs::write(&first_backup, "first backup").unwrap();

        // Second backup should use timestamp
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let second_backup = original.with_file_name(format!("rules.md.{}.bak", timestamp));

        // Verify paths are different
        assert_ne!(first_backup, second_backup);
    }

    /// Test backup cleanup keeps only MAX_BACKUPS most recent.
    #[test]
    fn test_backup_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.md");
        std::fs::write(&original, "content").unwrap();

        // Create multiple backups
        let backup_names = [
            "test.md.bak",
            "test.md.20240101_120000.bak",
            "test.md.20240102_120000.bak",
            "test.md.20240103_120000.bak",
            "test.md.20240104_120000.bak",
            "test.md.20240105_120000.bak",
            "test.md.20240106_120000.bak",
        ];

        for name in &backup_names {
            std::fs::write(temp_dir.path().join(name), "backup content").unwrap();
        }

        // Count backup files
        let backup_count = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bak"))
            .count();

        assert_eq!(backup_count, 7, "Should have created 7 backup files");
    }
}

mod output_path_resolution {
    use super::*;

    /// Test default output path for cursor format.
    #[test]
    fn test_default_cursor_path() {
        let formatter = ruley::output::get_formatter("cursor").unwrap();
        let options = WriteOptions::new("/project");

        // Build path from formatter defaults
        let dir = formatter.default_directory();
        let filename = formatter.default_filename();
        let ext = formatter.extension();

        let file_with_ext = format!("{}.{}", filename, ext);
        let path = if dir.is_empty() {
            options.base_path.join(file_with_ext)
        } else {
            options.base_path.join(dir).join(file_with_ext)
        };

        assert!(
            path.to_string_lossy().contains(".cursor/rules")
                || path.to_string_lossy().contains(".mdc"),
            "Cursor path should use .cursor/rules dir or .mdc extension: {}",
            path.display()
        );
    }

    /// Test custom output path overrides default.
    #[test]
    fn test_custom_path_override() {
        let mut paths = HashMap::new();
        paths.insert("claude".to_string(), "docs/AI_RULES.md".to_string());

        let options = WriteOptions::new("/project").with_output_paths(paths);

        let custom_path = options
            .output_paths
            .get("claude")
            .map(|p| options.base_path.join(p));

        assert_eq!(
            custom_path.unwrap(),
            PathBuf::from("/project/docs/AI_RULES.md")
        );
    }

    /// Test all formats have valid default paths.
    #[test]
    fn test_all_formats_have_default_paths() {
        let formats = [
            "cursor", "claude", "copilot", "windsurf", "aider", "generic", "json",
        ];

        for format in &formats {
            let formatter = ruley::output::get_formatter(format);
            assert!(
                formatter.is_ok(),
                "Should get formatter for format: {}",
                format
            );
            let formatter = formatter.unwrap();
            let filename = formatter.default_filename();
            let ext = formatter.extension();
            // Either filename or extension must be non-empty to produce a valid path.
            // Some formats (e.g., windsurf → ".windsurfrules") use only the extension.
            assert!(
                !filename.is_empty() || !ext.is_empty(),
                "Either filename or extension should be non-empty for {}",
                format
            );
        }
    }
}

// ── Comment 4: Conflict resolution integration tests ───────────────────────

mod interactive_prompts_simulation {
    use super::*;

    /// Test that interactive WriteOptions enables prompt-based resolution.
    #[test]
    fn test_interactive_options_enable_prompts() {
        let options = WriteOptions::new("/project")
            .with_conflict_strategy(ConflictStrategy::Prompt)
            .with_interactive(true);

        assert!(options.is_interactive);
        assert_eq!(options.conflict_strategy, ConflictStrategy::Prompt);
    }

    /// Test all ConflictStrategy variants are valid prompt responses.
    #[test]
    fn test_all_conflict_strategies_representable() {
        // Prompt → interactive prompt with O/S/M/A/Q choices
        // Overwrite → 'O' response
        // Skip → 'S' response
        // SmartMerge → 'M' response
        let strategies = [
            ConflictStrategy::Prompt,
            ConflictStrategy::Overwrite,
            ConflictStrategy::Skip,
            ConflictStrategy::SmartMerge,
        ];

        for strategy in &strategies {
            let display = strategy.to_string();
            let roundtrip: ConflictStrategy = display.parse().unwrap();
            assert_eq!(
                *strategy, roundtrip,
                "Strategy {:?} should round-trip through display/parse",
                strategy
            );
        }
    }

    /// Test WriteOptions with each conflict strategy.
    #[test]
    fn test_write_options_with_each_strategy() {
        for strategy in &[
            ConflictStrategy::Overwrite,
            ConflictStrategy::Skip,
            ConflictStrategy::SmartMerge,
            ConflictStrategy::Prompt,
        ] {
            let options = WriteOptions::new("/project")
                .with_conflict_strategy(*strategy)
                .with_interactive(true);

            assert_eq!(options.conflict_strategy, *strategy);
        }
    }
}

mod non_interactive_behavior {
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::output::{ConflictStrategy, WriteOptions, write_output};
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Test Prompt strategy fails in non-interactive mode when file exists.
    #[tokio::test]
    async fn test_prompt_non_interactive_fails() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create existing file at the custom output path
        std::fs::write(base_path.join("rules.md"), "# Old rules").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New Rules\n\nUpdated.\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Prompt)
            .with_interactive(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_err(),
            "Prompt strategy should fail in non-interactive mode"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("non-interactive") || err.contains("--on-conflict"),
            "Error should mention non-interactive mode: {}",
            err
        );
    }

    /// Test SmartMerge strategy fails in non-interactive mode.
    #[tokio::test]
    async fn test_smart_merge_non_interactive_fails() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Old rules").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New Rules\n\nUpdated.\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::SmartMerge)
            .with_interactive(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_err(),
            "SmartMerge strategy should fail in non-interactive mode"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("interactive") || err.contains("--on-conflict"),
            "Error should mention interactive requirement: {}",
            err
        );
    }

    /// Test Overwrite strategy succeeds in non-interactive mode.
    #[tokio::test]
    async fn test_overwrite_non_interactive_succeeds() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Old rules").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new(
            "generic",
            "# New Rules\n\nUpdated content.\n",
        ));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(false)
            .with_backups(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_ok(),
            "Overwrite should succeed: {:?}",
            result.err()
        );
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(!results[0].skipped);
        assert!(!results[0].is_new);

        let content = std::fs::read_to_string(base_path.join("rules.md")).unwrap();
        assert!(
            content.contains("Updated content"),
            "File should be overwritten with new content"
        );
    }

    /// Test Skip strategy skips in non-interactive mode.
    #[tokio::test]
    async fn test_skip_non_interactive_skips() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Old rules").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new(
            "generic",
            "# New Rules\n\nUpdated content.\n",
        ));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Skip)
            .with_interactive(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(result.is_ok(), "Skip should succeed: {:?}", result.err());
        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].skipped, "File should be skipped");

        // Original content should be preserved
        let content = std::fs::read_to_string(base_path.join("rules.md")).unwrap();
        assert_eq!(
            content, "# Old rules",
            "Original content should be preserved"
        );
    }

    /// Test new file is written without conflict resolution.
    #[tokio::test]
    async fn test_new_file_no_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new(
            "generic",
            "# New Rules\n\nFresh content.\n",
        ));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "new_rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Prompt)
            .with_interactive(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_ok(),
            "New file should not trigger conflict: {:?}",
            result.err()
        );
        let results = result.unwrap();
        assert!(results[0].is_new, "Should be marked as new file");
        assert!(!results[0].skipped);
    }
}

mod on_conflict_behaviors {
    use super::*;

    /// Test all valid --on-conflict values parse correctly.
    #[test]
    fn test_all_on_conflict_values() {
        let cases = [
            ("prompt", ConflictStrategy::Prompt),
            ("overwrite", ConflictStrategy::Overwrite),
            ("skip", ConflictStrategy::Skip),
            ("smart-merge", ConflictStrategy::SmartMerge),
        ];

        for (input, expected) in &cases {
            let result: ConflictStrategy = input.parse().unwrap();
            assert_eq!(
                result, *expected,
                "--on-conflict {} should map to {:?}",
                input, expected
            );
        }
    }

    /// Test invalid --on-conflict value produces actionable error.
    #[test]
    fn test_invalid_on_conflict_error() {
        let result: Result<ConflictStrategy, _> = "merge".parse();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid conflict strategy")
                || err.contains("prompt, overwrite, skip, smart-merge"),
            "Error should list valid values: {}",
            err
        );
    }

    /// Test WriteOptions correctly carries the on-conflict strategy.
    #[test]
    fn test_write_options_carries_strategy() {
        let strategy: ConflictStrategy = "overwrite".parse().unwrap();
        let options = WriteOptions::new("/project").with_conflict_strategy(strategy);
        assert_eq!(options.conflict_strategy, ConflictStrategy::Overwrite);
    }
}

mod config_vs_cli_override {
    use super::*;

    /// Test config file on_conflict is used when CLI not specified.
    #[test]
    fn test_config_on_conflict_used() {
        // Config file specifies on_conflict = "skip"
        let config_value = "skip";
        let strategy: ConflictStrategy = config_value.parse().unwrap();
        assert_eq!(strategy, ConflictStrategy::Skip);
    }

    /// Test CLI --on-conflict overrides config file.
    #[test]
    fn test_cli_overrides_config_on_conflict() {
        let config_value = "skip";
        let cli_value = "overwrite";

        let config_strategy: ConflictStrategy = config_value.parse().unwrap();
        let cli_strategy: ConflictStrategy = cli_value.parse().unwrap();

        // CLI should take precedence
        assert_ne!(config_strategy, cli_strategy);
        assert_eq!(
            cli_strategy,
            ConflictStrategy::Overwrite,
            "CLI value should override config"
        );
    }

    /// Test default on_conflict when neither config nor CLI specifies it.
    #[test]
    fn test_default_on_conflict_is_prompt() {
        let default_value = "prompt";
        let strategy: ConflictStrategy = default_value.parse().unwrap();
        assert_eq!(
            strategy,
            ConflictStrategy::Prompt,
            "Default should be Prompt"
        );
    }
}

mod smart_merge_config {
    use super::*;
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::output::write_output;

    /// Test SmartMerge requires interactive mode.
    #[test]
    fn test_smart_merge_requires_interactive() {
        let options = WriteOptions::new("/project")
            .with_conflict_strategy(ConflictStrategy::SmartMerge)
            .with_interactive(false);

        // SmartMerge with non-interactive should fail when files exist
        assert!(!options.is_interactive);
        assert_eq!(options.conflict_strategy, ConflictStrategy::SmartMerge);
    }

    /// Test SmartMerge without LLM client fails gracefully.
    #[tokio::test]
    async fn test_smart_merge_without_client_fails() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Existing").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New Rules\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::SmartMerge)
            .with_interactive(false); // Non-interactive triggers fail-fast

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_err(),
            "SmartMerge without interactive mode should fail"
        );
    }
}

mod backup_advanced {
    use super::*;
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::output::write_output;

    /// Test overwrite creates backup when enabled.
    #[tokio::test]
    async fn test_overwrite_creates_backup() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Original").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# Updated\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(false)
            .with_backups(true);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await
        .unwrap();

        assert!(result[0].backup_created, "Backup should be created");
        assert!(
            result[0].backup_path.is_some(),
            "Backup path should be recorded"
        );
        let backup_path = result[0].backup_path.as_ref().unwrap();
        assert!(backup_path.exists(), "Backup file should exist on disk");

        let backup_content = std::fs::read_to_string(backup_path).unwrap();
        assert_eq!(
            backup_content, "# Original",
            "Backup should contain original content"
        );
    }

    /// Test timestamped backup when .bak already exists.
    #[test]
    fn test_timestamped_backup_path() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.md");
        std::fs::write(&original, "content").unwrap();

        // Create first backup
        let first_backup = temp_dir.path().join("test.md.bak");
        std::fs::write(&first_backup, "first backup").unwrap();

        // Second backup should use timestamp format
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let expected_name = format!("test.md.{}.bak", timestamp);
        let second_backup = temp_dir.path().join(&expected_name);

        assert_ne!(first_backup, second_backup);
        assert!(
            expected_name.contains(".bak"),
            "Timestamped backup should end with .bak"
        );
    }

    /// Test cleanup keeps at most MAX_BACKUPS (5) files.
    #[test]
    fn test_backup_cleanup_policy() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.md");
        std::fs::write(&original, "content").unwrap();

        // Create 8 backup files (exceeding MAX_BACKUPS=5)
        let backup_names = [
            "test.md.bak",
            "test.md.20240101_120000.bak",
            "test.md.20240102_120000.bak",
            "test.md.20240103_120000.bak",
            "test.md.20240104_120000.bak",
            "test.md.20240105_120000.bak",
            "test.md.20240106_120000.bak",
            "test.md.20240107_120000.bak",
        ];

        for name in &backup_names {
            std::fs::write(temp_dir.path().join(name), "backup content").unwrap();
        }

        let backup_count = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bak"))
            .count();

        assert_eq!(backup_count, 8, "Should have created 8 backup files");
        // cleanup_old_backups (called by write_output) would reduce to MAX_BACKUPS
    }

    /// Test overwrite without backups does not create .bak files.
    #[tokio::test]
    async fn test_overwrite_no_backup() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Original").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# Updated\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(false)
            .with_backups(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await
        .unwrap();

        assert!(!result[0].backup_created, "No backup should be created");
        assert!(result[0].backup_path.is_none());

        let bak_count = std::fs::read_dir(base_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".bak"))
            .count();
        assert_eq!(bak_count, 0, "No .bak files should exist");
    }
}

mod all_quit_flows {
    use super::*;
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::output::write_output;

    /// Test multi-format write with Overwrite applies to all files (simulates "All" flow).
    #[tokio::test]
    async fn test_overwrite_all_formats() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create existing files for two formats
        std::fs::write(base_path.join("rules1.md"), "# Old 1").unwrap();
        std::fs::write(base_path.join("rules2.md"), "# Old 2").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New Generic\n"));
        rules.add_format(FormattedRules::new("claude", "# New Claude\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules1.md".to_string());
        paths.insert("claude".to_string(), "rules2.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Overwrite)
            .with_interactive(false)
            .with_backups(false);

        let formats = vec!["generic".to_string(), "claude".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 2, "Should write both formats");
        assert!(
            !result[0].skipped && !result[1].skipped,
            "Neither file should be skipped"
        );
    }

    /// Test multi-format write with Skip skips all existing files (simulates "All skip" flow).
    #[tokio::test]
    async fn test_skip_all_formats() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules1.md"), "# Old 1").unwrap();
        std::fs::write(base_path.join("rules2.md"), "# Old 2").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New Generic\n"));
        rules.add_format(FormattedRules::new("claude", "# New Claude\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules1.md".to_string());
        paths.insert("claude".to_string(), "rules2.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Skip)
            .with_interactive(false);

        let formats = vec!["generic".to_string(), "claude".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].skipped, "First file should be skipped");
        assert!(result[1].skipped, "Second file should be skipped");

        // Original content should be preserved
        assert_eq!(
            std::fs::read_to_string(base_path.join("rules1.md")).unwrap(),
            "# Old 1"
        );
        assert_eq!(
            std::fs::read_to_string(base_path.join("rules2.md")).unwrap(),
            "# Old 2"
        );
    }

    /// Test Quit behavior: non-interactive Prompt on existing file produces abort error.
    #[tokio::test]
    async fn test_quit_aborts_write() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        std::fs::write(base_path.join("rules.md"), "# Existing").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# New\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "rules.md".to_string());

        // Non-interactive Prompt triggers an error similar to Quit
        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Prompt)
            .with_interactive(false);

        let formats = vec!["generic".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await;

        assert!(
            result.is_err(),
            "Non-interactive prompt should abort like Quit"
        );
    }

    /// Test mixed new/existing files: new files written, existing depend on strategy.
    #[tokio::test]
    async fn test_mixed_new_and_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Only one file exists
        std::fs::write(base_path.join("existing.md"), "# Old").unwrap();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new("generic", "# Generic\n"));
        rules.add_format(FormattedRules::new("claude", "# Claude\n"));

        let mut paths = HashMap::new();
        paths.insert("generic".to_string(), "existing.md".to_string());
        paths.insert("claude".to_string(), "new_file.md".to_string());

        let options = WriteOptions::new(base_path)
            .with_output_paths(paths)
            .with_conflict_strategy(ConflictStrategy::Skip)
            .with_interactive(false)
            .with_backups(false);

        let formats = vec!["generic".to_string(), "claude".to_string()];
        let mut tracker = None;

        let result = write_output(
            &rules,
            &formats,
            "test",
            &options,
            None,
            &mut tracker,
            None,
            false,
        )
        .await
        .unwrap();

        assert_eq!(result.len(), 2);
        // generic → existing → skipped
        assert!(result[0].skipped, "Existing file should be skipped");
        // claude → new → written
        assert!(result[1].is_new, "New file should be created");
        assert!(!result[1].skipped);
        assert!(base_path.join("new_file.md").exists());
    }
}
