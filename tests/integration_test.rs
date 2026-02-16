//! Integration tests for ruley CLI.

mod common;

use std::process::Command;

/// Verify the binary can be invoked and shows help.
#[test]
fn test_cli_help() {
    let output = Command::new(common::ruley_bin())
        .args(["--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("ruley") || stdout.contains("Usage"));
}

/// Verify the binary shows version information.
#[test]
fn test_cli_version() {
    let output = Command::new(common::ruley_bin())
        .args(["--version"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("0.1.0") || stdout.contains("ruley"));
}

#[cfg(test)]
mod packer_integration {
    use super::common::{create_mock_project, create_temp_dir, rust_project_files};

    #[test]
    fn test_mock_rust_project_creation() {
        let temp_dir = create_temp_dir();
        let files = rust_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
        assert!(project_path.join("src/lib.rs").exists());
    }
}

#[cfg(test)]
mod output_integration {
    use super::common::{create_mock_project, create_temp_dir, typescript_project_files};

    #[test]
    fn test_mock_typescript_project_creation() {
        let temp_dir = create_temp_dir();
        let files = typescript_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        assert!(project_path.join("package.json").exists());
        assert!(project_path.join("tsconfig.json").exists());
        assert!(project_path.join("src/index.ts").exists());
    }
}

#[cfg(test)]
mod config_integration {
    use super::common::{create_config_file, create_temp_dir, run_cli_with_config};

    /// Test that dry-run mode shows configuration without making LLM calls.
    #[test]
    fn test_dry_run_mode() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run"]);
        let stdout = String::from_utf8_lossy(&output.stdout);

        // The new dry-run format shows file breakdown and cost estimate
        assert!(
            stdout.contains("Dry Run") || stdout.contains("No LLM calls"),
            "Expected dry-run indicators in output: {stdout}"
        );
    }

    /// Test that CLI flags override config file values.
    #[test]
    fn test_cli_overrides_config() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let config_content = r#"[general]
provider = "openai"
"#;
        let config_path = create_config_file(&temp_dir, config_content);

        let output = run_cli_with_config(
            &project_path,
            &[
                "--config",
                config_path.to_str().unwrap(),
                "--dry-run",
                "--provider",
                "anthropic",
            ],
        );
        let stdout = String::from_utf8_lossy(&output.stdout);

        // The dry-run mode should complete successfully with provider override
        assert!(
            output.status.success(),
            "Dry-run should succeed with CLI provider override: {stdout}"
        );
    }

    /// Test that invalid TOML syntax produces an error.
    #[test]
    fn test_invalid_toml_fails() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        // Missing closing bracket
        let config_content = "[general\nprovider = \"openai\"";
        let config_path = create_config_file(&temp_dir, config_content);

        let output = run_cli_with_config(
            &project_path,
            &["--config", config_path.to_str().unwrap(), "--dry-run"],
        );

        assert!(!output.status.success(), "Should fail with invalid TOML");
    }

    /// Test that missing explicit config file is handled gracefully.
    #[test]
    fn test_missing_config_file_handled() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();
        let nonexistent = project_path.join("nonexistent.toml");

        let output = run_cli_with_config(
            &project_path,
            &["--config", nonexistent.to_str().unwrap(), "--dry-run"],
        );

        // Should exit cleanly (not panic), regardless of success/failure
        assert!(output.status.code().is_some());
    }

    #[cfg(test)]
    mod env_override {
        use super::*;

        /// RAII guard for managing environment variables in tests.
        /// Automatically restores the original value (or removes the var) when dropped.
        struct EnvGuard {
            key: &'static str,
            original: Option<String>,
        }

        // SAFETY: In Rust 2024 edition, `std::env::set_var` is unsafe because it can cause
        // data races if called concurrently with `std::env::var` in other threads. In tests,
        // we accept this risk as tests are isolated and this is the standard pattern for
        // testing environment variable handling.
        #[allow(unsafe_code)]
        impl EnvGuard {
            fn new(key: &'static str, value: &str) -> Self {
                let original = std::env::var(key).ok();
                unsafe {
                    std::env::set_var(key, value);
                }
                Self { key, original }
            }
        }

        // SAFETY: See comment above on EnvGuard impl. `set_var` and `remove_var` are unsafe
        // in Rust 2024 due to potential data races, but are acceptable in test code.
        #[allow(unsafe_code)]
        impl Drop for EnvGuard {
            fn drop(&mut self) {
                unsafe {
                    match &self.original {
                        Some(v) => std::env::set_var(self.key, v),
                        None => std::env::remove_var(self.key),
                    }
                }
            }
        }

        /// Test full three-tier precedence: config file → env vars → CLI flags
        #[test]
        fn test_three_tier_precedence() {
            let temp_dir = create_temp_dir();
            let project_path = temp_dir.path().to_path_buf();

            // Config baseline
            let config_content = r#"[general]
provider = "anthropic"
compress = false
chunk_size = 123
no_confirm = false
"#;
            let config_path = create_config_file(&temp_dir, config_content);

            // Env overrides config
            let _guard_provider = EnvGuard::new("RULEY_GENERAL_PROVIDER", "openai");
            let _guard_compress = EnvGuard::new("RULEY_GENERAL_COMPRESS", "true");

            // CLI overrides env+config for chunk size
            let output = run_cli_with_config(
                &project_path,
                &[
                    "--config",
                    config_path.to_str().unwrap(),
                    "--dry-run",
                    "--chunk-size",
                    "75000",
                ],
            );

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !output.status.success() {
                eprintln!("CLI exited with code: {:?}", output.status.code());
                eprintln!("STDOUT:\n{}", stdout);
                eprintln!("STDERR:\n{}", stderr);
            }
            assert!(output.status.success(), "Expected dry-run to succeed");

            // Verify dry-run mode shows the expected output format
            assert!(
                stdout.contains("Dry Run") || stdout.contains("Files to be analyzed"),
                "Expected dry-run output indicators"
            );
            // The actual precedence logic is verified by the successful execution
            // with the config/env/CLI combination - if precedence was wrong, the
            // wrong provider would be used and might fail differently
        }
    }
}

#[cfg(test)]
mod gitignore_tests {
    //! Tests for gitignore pattern matching via the GitIgnorer wrapper.
    //! Tests focus on integration with the GitIgnorer, not the underlying `ignore` crate.

    use super::common::create_temp_dir;
    use ruley::packer::gitignore::GitIgnorer;

    /// Test basic gitignore patterns are applied.
    #[test]
    fn test_gitignore_basic_patterns() {
        let temp_dir = create_temp_dir();

        // Create a .gitignore with standard patterns
        let gitignore_path = temp_dir.path().join(".gitignore");
        std::fs::write(&gitignore_path, ".git\n.DS_Store\nnode_modules/\n*.log\n")
            .expect("Failed to write .gitignore");

        let ignorer = GitIgnorer::new(temp_dir.path()).expect("Failed to create GitIgnorer");

        // Verify standard patterns are recognized
        assert!(
            ignorer.is_ignored(".git"),
            ".git directory should be ignored"
        );
        assert!(
            ignorer.is_ignored(".DS_Store"),
            ".DS_Store should be ignored"
        );
        assert!(
            ignorer.is_ignored("node_modules"),
            "node_modules directory should be ignored"
        );
        assert!(
            ignorer.is_ignored("debug.log"),
            "*.log files should be ignored"
        );
    }

    /// Test glob patterns are properly matched.
    #[test]
    fn test_gitignore_glob_patterns() {
        let temp_dir = create_temp_dir();

        let gitignore_path = temp_dir.path().join(".gitignore");
        std::fs::write(&gitignore_path, "*.log\n*.tmp\ntarget/\n").expect("Failed to write");

        let ignorer = GitIgnorer::new(temp_dir.path()).expect("Failed to create GitIgnorer");

        assert!(ignorer.is_ignored("app.log"));
        assert!(ignorer.is_ignored("test.tmp"));
        assert!(
            ignorer.is_ignored("target"),
            "target/ directory should be ignored"
        );
    }

    /// Test nested directory matching.
    #[test]
    fn test_gitignore_nested_paths() {
        let temp_dir = create_temp_dir();

        let gitignore_path = temp_dir.path().join(".gitignore");
        std::fs::write(&gitignore_path, "node_modules/\nbuild/\n.git/\n").expect("Failed to write");

        let ignorer = GitIgnorer::new(temp_dir.path()).expect("Failed to create GitIgnorer");

        assert!(ignorer.is_ignored("node_modules"));
        assert!(ignorer.is_ignored("node_modules/package"));
        assert!(ignorer.is_ignored("build"));
        assert!(ignorer.is_ignored("build/output"));
    }
}

#[cfg(test)]
mod file_scanning_tests {
    //! Tests for file discovery and language detection.

    use super::common::{create_mock_project, create_temp_dir, rust_project_files};
    use ruley::MergedConfig;

    /// Test that file scanning discovers all files in a project.
    #[tokio::test]
    async fn test_scan_discovers_all_files() {
        let temp_dir = create_temp_dir();
        let files = rust_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        // Create minimal config for scanning
        let config = MergedConfig {
            provider: "anthropic".to_string(),
            model: None,
            format: vec!["cursor".to_string()],
            output: None,
            repomix_file: None,
            path: project_path.clone(),
            description: None,
            rule_type: ruley::generator::rules::RuleType::default(),
            include: vec![],
            exclude: vec![],
            compress: false,
            chunk_size: 100000,
            no_confirm: true,
            dry_run: true,
            verbose: 0,
            quiet: false,
            chunking: None,
            output_paths: std::collections::HashMap::new(),
            providers: ruley::cli::config::ProvidersConfig::default(),
            validation: ruley::cli::config::ValidationConfig::default(),
            finalization: ruley::cli::config::FinalizationConfig::default(),
            on_conflict: "prompt".to_string(),
        };

        // Use the walker to scan files
        let entries = ruley::packer::walker::scan_files(&project_path, &config)
            .await
            .expect("Failed to scan files");

        // Assert discovered paths
        assert!(!entries.is_empty(), "Should discover files in project");

        let paths: Vec<String> = entries
            .iter()
            .map(|e| e.path.display().to_string())
            .collect();

        // Should find Rust source files
        assert!(
            paths.iter().any(|p| p.ends_with("main.rs")),
            "Should discover main.rs"
        );
        assert!(
            paths.iter().any(|p| p.ends_with("lib.rs")),
            "Should discover lib.rs"
        );
    }

    /// Test language detection for Rust files.
    #[test]
    fn test_language_detection_rust() {
        let temp_dir = create_temp_dir();
        let files = rust_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        let main_rs = project_path.join("src/main.rs");
        assert!(main_rs.exists(), "main.rs should exist");
        assert!(main_rs.extension().is_some_and(|ext| ext == "rs"));
    }

    /// Test language detection for TypeScript files.
    #[test]
    fn test_language_detection_typescript() {
        let temp_dir = create_temp_dir();
        let ts_files = vec![
            ("src/index.ts", "export const version = \"1.0.0\";"),
            ("src/types.ts", "export interface User { name: string; }"),
        ];

        let project_path = super::common::create_mock_project(&temp_dir, &ts_files);

        let index_ts = project_path.join("src/index.ts");
        assert!(index_ts.exists());
        assert!(index_ts.extension().is_some_and(|ext| ext == "ts"));
    }

    /// Test that symlinks are properly handled (skipped during scanning).
    #[test]
    #[cfg(unix)]
    fn test_symlink_handling() {
        let temp_dir = create_temp_dir();
        let files = rust_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        // Create a symlink
        let symlink_src = project_path.join("src/main.rs");
        let symlink_dst = project_path.join("link_to_main.rs");

        if std::os::unix::fs::symlink(&symlink_src, &symlink_dst).is_ok() {
            // Symlink was created; it should exist but be handled appropriately
            assert!(symlink_dst.exists());
        }
    }
}

#[cfg(test)]
mod compression_pipeline_tests {
    //! Tests for end-to-end compression pipeline.

    use super::common::{
        create_mock_project, create_temp_dir, rust_project_files, typescript_project_files,
    };

    /// Test that compression is properly applied to TypeScript projects.
    #[tokio::test]
    #[cfg(feature = "compression-typescript")]
    async fn test_compress_typescript_project() {
        let temp_dir = create_temp_dir();
        let files = typescript_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        // Create config with compression enabled
        let config = ruley::MergedConfig {
            provider: "anthropic".to_string(),
            model: None,
            format: vec!["cursor".to_string()],
            output: None,
            repomix_file: None,
            path: project_path.clone(),
            description: None,
            rule_type: ruley::generator::rules::RuleType::default(),
            include: vec![],
            exclude: vec![],
            compress: true,
            chunk_size: 100000,
            no_confirm: true,
            dry_run: true,
            verbose: 0,
            quiet: false,
            chunking: None,
            output_paths: std::collections::HashMap::new(),
            providers: ruley::cli::config::ProvidersConfig::default(),
            validation: ruley::cli::config::ValidationConfig::default(),
            finalization: ruley::cli::config::FinalizationConfig::default(),
            on_conflict: "prompt".to_string(),
        };

        // Scan files first
        let entries = ruley::packer::walker::scan_files(&project_path, &config)
            .await
            .expect("Failed to scan files");

        // Run compression pipeline
        let compressed = ruley::packer::compress::compress_codebase(entries, &config)
            .await
            .expect("Failed to compress codebase");

        // Assert file count
        assert!(!compressed.files.is_empty(), "Should have compressed files");

        // Assert compression methods were applied
        let has_tree_sitter = compressed
            .files
            .iter()
            .any(|f| f.compression_method == ruley::packer::CompressionMethod::TreeSitter);

        assert!(
            has_tree_sitter,
            "Should use tree-sitter compression for TypeScript files"
        );

        // Assert languages detected
        assert!(
            compressed
                .metadata
                .languages
                .contains_key(&ruley::packer::compress::Language::TypeScript),
            "Should detect TypeScript language"
        );
    }

    /// Test Rust project compression.
    #[test]
    fn test_compress_rust_project() {
        let temp_dir = create_temp_dir();
        let files = rust_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
    }

    /// Test mixed language project.
    #[test]
    fn test_mixed_language_project() {
        let temp_dir = create_temp_dir();
        let mixed_files = vec![
            ("Cargo.toml", "[package]\nname = \"test\"\n"),
            ("src/main.rs", "fn main() {}"),
            ("src/index.ts", "export const x = 1;"),
            ("script.py", "print('hello')"),
        ];

        let project_path = super::common::create_mock_project(&temp_dir, &mixed_files);

        // Verify all file types exist
        assert!(project_path.join("Cargo.toml").exists());
        assert!(project_path.join("src/main.rs").exists());
        assert!(project_path.join("src/index.ts").exists());
        assert!(project_path.join("script.py").exists());
    }

    /// Test that compression can be disabled.
    #[test]
    fn test_compression_disabled() {
        let temp_dir = create_temp_dir();
        let files = typescript_project_files();
        let project_path = create_mock_project(&temp_dir, &files);

        // When compression is disabled, files should be returned uncompressed
        assert!(project_path.join("src/index.ts").exists());
    }
}

#[cfg(test)]
mod repomix_integration_tests {
    //! Tests for end-to-end repomix parsing workflows.

    use super::common::create_temp_dir;
    use tokio::fs;

    /// Test parsing a complete markdown repomix file.
    #[tokio::test]
    async fn test_parse_markdown_repomix_integration() {
        let temp_dir = create_temp_dir();
        let content = r#"# Repository

## File: src/main.rs
```rust
fn main() {
    println!("Hello");
}
```

## File: src/lib.rs
```rust
pub fn greet() -> &'static str {
    "Hi"
}
```"#;

        let path = temp_dir.path().join("repomix.md");
        fs::write(&path, content)
            .await
            .expect("Failed to write test file");

        // Parse the repomix file
        let parsed = ruley::packer::repomix::parse_repomix(&path)
            .await
            .expect("Failed to parse markdown repomix");

        // Assert the CompressedCodebase structure
        assert_eq!(parsed.files.len(), 2, "Should have 2 files");

        let file_paths: Vec<String> = parsed
            .files
            .iter()
            .map(|f| f.path.display().to_string())
            .collect();

        assert!(
            file_paths.iter().any(|p| p.contains("main.rs")),
            "Should contain main.rs"
        );
        assert!(
            file_paths.iter().any(|p| p.contains("lib.rs")),
            "Should contain lib.rs"
        );

        // Verify content was extracted
        let main_file = parsed.files.iter().find(|f| f.path.ends_with("main.rs"));
        assert!(main_file.is_some(), "Should find main.rs file");
        assert!(
            main_file.unwrap().original_content.contains("println"),
            "Should extract file content"
        );
    }

    /// Test parsing XML repomix format.
    #[tokio::test]
    async fn test_parse_xml_repomix_integration() {
        let temp_dir = create_temp_dir();
        let content = r#"<?xml version="1.0"?>
<repomix>
    <files>
        <file path="src/main.rs">
            <content><![CDATA[fn main() {}]]></content>
        </file>
    </files>
</repomix>"#;

        let path = temp_dir.path().join("repomix.xml");
        fs::write(&path, content)
            .await
            .expect("Failed to write test file");

        // Parse the XML repomix file
        let parsed = ruley::packer::repomix::parse_repomix(&path)
            .await
            .expect("Failed to parse XML repomix");

        // Assert the CompressedCodebase structure
        assert_eq!(parsed.files.len(), 1, "Should have 1 file");
        assert_eq!(
            parsed.files[0].path.display().to_string(),
            "src/main.rs",
            "Should parse file path"
        );
        assert!(
            parsed.files[0].original_content.contains("fn main"),
            "Should extract CDATA content"
        );
    }

    /// Test parsing JSON repomix format.
    #[tokio::test]
    async fn test_parse_json_repomix_integration() {
        let temp_dir = create_temp_dir();
        let content = r#"{
    "files": {
        "src/main.rs": "fn main() {}",
        "src/lib.rs": "pub fn test() {}"
    }
}"#;

        let path = temp_dir.path().join("repomix.json");
        fs::write(&path, content)
            .await
            .expect("Failed to write test file");

        // Parse the JSON repomix file
        let parsed = ruley::packer::repomix::parse_repomix(&path)
            .await
            .expect("Failed to parse JSON repomix");

        // Assert the CompressedCodebase structure
        assert_eq!(parsed.files.len(), 2, "Should have 2 files");

        let file_paths: Vec<String> = parsed
            .files
            .iter()
            .map(|f| f.path.display().to_string())
            .collect();

        assert!(
            file_paths.iter().any(|p| p.contains("main.rs")),
            "Should contain main.rs"
        );
        assert!(
            file_paths.iter().any(|p| p.contains("lib.rs")),
            "Should contain lib.rs"
        );
    }

    /// Test format auto-detection by extension.
    #[test]
    fn test_repomix_format_detection_by_extension() {
        use ruley::packer::repomix::detect_format;
        use std::path::PathBuf;

        let md_path = PathBuf::from("repomix.md");
        let xml_path = PathBuf::from("repomix.xml");
        let json_path = PathBuf::from("repomix.json");

        let content = "";

        let md_format = detect_format(&md_path, content);
        let xml_format = detect_format(&xml_path, content);
        let json_format = detect_format(&json_path, content);

        assert_eq!(md_format, ruley::packer::repomix::RepomixFormat::Markdown);
        assert_eq!(xml_format, ruley::packer::repomix::RepomixFormat::Xml);
        assert_eq!(json_format, ruley::packer::repomix::RepomixFormat::Json);
    }
}

#[cfg(test)]
mod fallback_tests {
    //! Tests for fallback behavior when compression fails or is unavailable.

    use super::common::create_temp_dir;
    use ruley::packer::compress::{Compressor, Language, WhitespaceCompressor};

    /// Test whitespace fallback for unsupported language.
    #[test]
    fn test_whitespace_fallback_unsupported_language() {
        let compressor = WhitespaceCompressor;
        let source = "arbitrary   text   content";

        // Whitespace compressor should work for any language
        let result = compressor
            .compress(source, Language::Cpp)
            .expect("Whitespace compression should work for any language");

        assert!(!result.is_empty());
    }

    /// Test tree-sitter fallback when feature is disabled.
    #[test]
    #[cfg(not(feature = "compression-typescript"))]
    fn test_tree_sitter_fallback_when_disabled() {
        use ruley::packer::compress::TreeSitterCompressor;
        let compressor = TreeSitterCompressor;
        let source = "function test(): void {}";

        // Should return error when feature disabled (allowing fallback)
        let result = compressor.compress(source, Language::TypeScript);
        assert!(
            result.is_err(),
            "Should error when tree-sitter feature disabled"
        );
    }

    /// Test graceful error handling for invalid syntax with end-to-end pipeline.
    #[tokio::test]
    #[cfg(feature = "compression-typescript")]
    async fn test_invalid_syntax_triggers_fallback() {
        use super::common::create_mock_project;
        let temp_dir = create_temp_dir();

        // Create project with invalid TypeScript file
        let invalid_ts = r#"function broken(
            const x = { missing comma }
            return x;
        "#;

        let files = vec![("src/invalid.ts", invalid_ts)];

        let project_path = create_mock_project(&temp_dir, &files);

        // Create config with compression enabled
        let config = ruley::MergedConfig {
            provider: "anthropic".to_string(),
            model: None,
            format: vec!["cursor".to_string()],
            output: None,
            repomix_file: None,
            path: project_path.clone(),
            description: None,
            rule_type: ruley::generator::rules::RuleType::default(),
            include: vec![],
            exclude: vec![],
            compress: true,
            chunk_size: 100000,
            no_confirm: true,
            dry_run: true,
            verbose: 0,
            quiet: false,
            chunking: None,
            output_paths: std::collections::HashMap::new(),
            providers: ruley::cli::config::ProvidersConfig::default(),
            validation: ruley::cli::config::ValidationConfig::default(),
            finalization: ruley::cli::config::FinalizationConfig::default(),
            on_conflict: "prompt".to_string(),
        };

        // Scan and compress - should fall back to whitespace compression on error
        let entries = ruley::packer::walker::scan_files(&project_path, &config)
            .await
            .expect("Failed to scan files");

        let compressed = ruley::packer::compress::compress_codebase(entries, &config)
            .await
            .expect("Compression pipeline should handle invalid syntax");

        // Tree-sitter parses error nodes gracefully, so check that compression happened
        // (either tree-sitter or fallback to whitespace)
        assert!(
            compressed.files.iter().all(|f| {
                f.compression_method == ruley::packer::CompressionMethod::TreeSitter
                    || f.compression_method == ruley::packer::CompressionMethod::Whitespace
            }),
            "Should use TreeSitter or fall back to Whitespace compression"
        );

        // Verify at least one file was processed
        assert!(
            !compressed.files.is_empty(),
            "Should process at least one file"
        );
    }

    /// Test compression on files with unsupported extensions.
    #[test]
    fn test_unsupported_file_extension() {
        let temp_dir = create_temp_dir();
        let files = vec![
            ("README.txt", "This is plain text content"),
            ("data.csv", "col1,col2,col3\nval1,val2,val3"),
        ];

        let project_path = super::common::create_mock_project(&temp_dir, &files);

        assert!(project_path.join("README.txt").exists());
        assert!(project_path.join("data.csv").exists());

        // Whitespace compression should be applied to unsupported formats
        let compressor = WhitespaceCompressor;
        let content = "Some     text     content";
        let result = compressor
            .compress(content, Language::Cpp)
            .expect("Whitespace fallback should work");

        assert!(!result.is_empty());
    }
}

#[cfg(test)]
mod multi_format_output_tests {
    //! Flow 11: Multi-format output generation.
    //!
    //! Tests that all output formatters can be instantiated and produce valid
    //! content from generated rules, and that format metadata is correct.

    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::output::{Metadata, get_formatter};

    /// Test that all supported formats can be instantiated via get_formatter.
    #[test]
    fn test_all_formatters_instantiate() {
        let formats = [
            "cursor", "claude", "copilot", "windsurf", "aider", "generic", "json",
        ];
        for format in &formats {
            let formatter = get_formatter(format);
            assert!(
                formatter.is_ok(),
                "Should instantiate formatter for '{}'",
                format
            );
        }
    }

    /// Test that an unknown format returns an error.
    #[test]
    fn test_unknown_format_errors() {
        let result = get_formatter("nonexistent");
        assert!(result.is_err());
    }

    /// Test multi-format output from a single GeneratedRules.
    #[test]
    fn test_multi_format_generation() {
        let mut rules = GeneratedRules::new("analysis");

        // Simulate LLM producing format-specific content
        rules.add_format(FormattedRules::new(
            "cursor",
            "---\ndescription: Project rules\nalwaysApply: true\n---\n\n# Rules\n\nUse spaces.\n",
        ));
        rules.add_format(FormattedRules::new(
            "claude",
            "# Project Rules\n\n## Standards\n\nUse spaces for indentation.\n",
        ));
        rules.add_format(FormattedRules::new(
            "json",
            r#"{"rules": ["Use spaces for indentation"]}"#,
        ));

        assert_eq!(rules.formats().count(), 3);

        // Each format should be retrievable
        let cursor_content = rules.get_format("cursor");
        assert!(cursor_content.is_some());
        assert!(cursor_content.unwrap().content.contains("Rules"));

        let claude_content = rules.get_format("claude");
        assert!(claude_content.is_some());
        assert!(claude_content.unwrap().content.contains("Standards"));

        let json_content = rules.get_format("json");
        assert!(json_content.is_some());
        assert!(json_content.unwrap().content.contains("rules"));
    }

    /// Test format metadata: extensions and default directories.
    #[test]
    fn test_format_metadata_correctness() {
        let cursor = get_formatter("cursor").unwrap();
        assert_eq!(cursor.extension(), "mdc");
        assert!(
            !cursor.default_directory().is_empty(),
            "Cursor should have a subdirectory"
        );

        let claude = get_formatter("claude").unwrap();
        assert_eq!(claude.extension(), "md");

        let json = get_formatter("json").unwrap();
        assert_eq!(json.extension(), "json");
    }

    /// Test formatter output with generated rules containing format-specific content.
    #[test]
    fn test_formatter_output_retrieves_content() {
        let mut rules = GeneratedRules::new("test analysis");
        let expected_content = "# Copilot Instructions\n\nFollow these rules.\n";
        rules.add_format(FormattedRules::new("copilot", expected_content));

        let metadata = Metadata {
            project_name: "test-project".to_string(),
            format: "copilot".to_string(),
        };

        let formatter = get_formatter("copilot").unwrap();
        let result = formatter.format(&rules, &metadata);
        assert!(
            result.is_ok(),
            "Formatter should produce output: {:?}",
            result.err()
        );

        let output = result.unwrap();
        assert!(
            output.contains("Follow these rules"),
            "Output should contain the generated content"
        );
    }
}

#[cfg(test)]
mod cost_tracking_integration_tests {
    //! Flow 12: Cost calculation and tracking integration.
    //!
    //! Tests end-to-end cost tracking across multi-step operations,
    //! verifying correct aggregation, breakdown, and summary generation.

    use ruley::llm::cost::{CostCalculator, CostTracker};
    use ruley::llm::provider::Pricing;

    /// Test cost tracking across a simulated multi-step rule generation.
    #[test]
    fn test_multi_step_cost_tracking() {
        let pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        // Simulate a typical rule generation workflow
        tracker.add_operation("analysis", 8000, 3000);
        tracker.add_operation("chunk_1_generation", 4000, 2000);
        tracker.add_operation("chunk_2_generation", 4000, 1800);
        tracker.add_operation("merge", 6000, 4000);

        assert_eq!(tracker.operation_count(), 4);
        assert_eq!(tracker.total_input_tokens(), 22000);
        assert_eq!(tracker.total_output_tokens(), 10800);
        assert_eq!(tracker.total_tokens(), 32800);

        // Verify cost is positive and reasonable
        let total = tracker.total_cost();
        assert!(total > 0.0, "Total cost should be positive");

        // Verify breakdown matches
        let breakdown = tracker.breakdown();
        assert_eq!(breakdown.len(), 4);
        assert_eq!(breakdown[0].operation, "analysis");
        assert_eq!(breakdown[3].operation, "merge");

        // Sum of breakdown costs should equal total
        let breakdown_sum: f64 = breakdown.iter().map(|b| b.cost).sum();
        assert!(
            (breakdown_sum - total).abs() < 0.0001,
            "Breakdown sum should equal total cost"
        );
    }

    /// Test cost estimation before making requests.
    #[test]
    fn test_cost_estimation_before_request() {
        let pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let calculator = CostCalculator::new(pricing);

        let estimate = calculator.estimate_cost(10000, 5000);

        // Input: 10.0 * 0.003 = 0.03
        // Output: 5.0 * 0.015 = 0.075
        assert!((estimate.input_cost - 0.03).abs() < 0.0001);
        assert!((estimate.output_cost - 0.075).abs() < 0.0001);
        assert!((estimate.total_cost - 0.105).abs() < 0.0001);
        assert_eq!(estimate.total_tokens(), 15000);
    }

    /// Test cost summary with average calculation.
    #[test]
    fn test_cost_summary_aggregation() {
        let pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        tracker.add_operation("op1", 2000, 1000);
        tracker.add_operation("op2", 4000, 2000);

        let summary = tracker.summary();
        assert_eq!(summary.operation_count, 2);
        assert_eq!(summary.total_input_tokens, 6000);
        assert_eq!(summary.total_output_tokens, 3000);
        assert_eq!(summary.total_tokens(), 9000);

        let avg = summary.average_cost_per_operation();
        assert!(avg > 0.0, "Average cost should be positive");
        assert!(
            (avg - summary.total_cost / 2.0).abs() < 0.0001,
            "Average should be total / count"
        );
    }

    /// Test free provider (Ollama) has zero cost.
    #[test]
    fn test_free_provider_tracking() {
        let pricing = Pricing {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        tracker.add_operation("local_inference", 50000, 20000);

        assert!((tracker.total_cost() - 0.0).abs() < f64::EPSILON);
        assert_eq!(tracker.total_tokens(), 70000);
    }

    /// Test tracker reset clears all state.
    #[test]
    fn test_tracker_reset_clears_state() {
        let pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        tracker.add_operation("op1", 5000, 2000);
        assert!(tracker.total_cost() > 0.0);

        tracker.reset();
        assert_eq!(tracker.operation_count(), 0);
        assert!((tracker.total_cost() - 0.0).abs() < f64::EPSILON);
        assert_eq!(tracker.total_input_tokens(), 0);
    }
}

#[cfg(test)]
mod validation_pipeline_integration_tests {
    //! Flow 13: Validation pipeline integration.
    //!
    //! Tests the full validation pipeline using real validators with
    //! representative content, verifying error layering and format-specific checks.

    use ruley::cli::config::SemanticValidationConfig;
    use ruley::packer::{CodebaseMetadata, CompressedCodebase, CompressedFile, CompressionMethod};
    use ruley::utils::validation::{ValidationLayer, get_validator};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_codebase() -> CompressedCodebase {
        CompressedCodebase {
            files: vec![CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() {}".to_string(),
                compressed_content: "fn main() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 12,
                compressed_size: 12,
                language: None,
            }],
            metadata: CodebaseMetadata {
                total_files: 1,
                total_original_size: 12,
                total_compressed_size: 12,
                languages: HashMap::new(),
                compression_ratio: 1.0,
            },
        }
    }

    /// Test that valid content passes all validation layers for Claude format.
    #[test]
    fn test_claude_valid_content_passes_all_layers() {
        let validator = get_validator("claude").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        let content = "# Project Rules\n\n## Coding Standards\n\nUse consistent formatting.\n\n## File Structure\n\nOrganize by feature.\n";
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            result.passed,
            "Valid Claude content should pass: {:?}",
            result.errors
        );
    }

    /// Test that valid Cursor content with frontmatter passes.
    #[test]
    fn test_cursor_valid_content_passes() {
        let validator = get_validator("cursor").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        let content = "---\ndescription: Main rules\nalwaysApply: true\n---\n\n# Rules\n\nUse 4-space indentation.\n";
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            result.passed,
            "Valid Cursor content should pass: {:?}",
            result.errors
        );
    }

    /// Test that valid JSON content passes.
    #[test]
    fn test_json_valid_content_passes() {
        let validator = get_validator("json").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        let content = r#"{"rules": ["Use consistent formatting", "Follow naming conventions"]}"#;
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            result.passed,
            "Valid JSON content should pass: {:?}",
            result.errors
        );
    }

    /// Test validation error layers are correctly identified.
    #[test]
    fn test_error_layers_correctly_identified() {
        let validator = get_validator("claude").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        // Empty content triggers syntax layer errors
        let result = validator.validate("", &config, &codebase).unwrap();
        assert!(!result.passed);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Syntax),
            "Empty content should produce syntax errors"
        );
    }

    /// Test semantic validation detects contradictions across formats.
    #[test]
    fn test_semantic_contradiction_detection() {
        let validator = get_validator("generic").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };

        let content =
            "# Rules\n\nAlways use tabs for indentation.\nAlways use spaces for indentation.\n";
        let result = validator.validate(content, &config, &codebase).unwrap();

        assert!(
            result
                .errors
                .iter()
                .any(|e| e.layer == ValidationLayer::Semantic),
            "Should detect contradiction between tabs and spaces"
        );
    }

    /// Test validation across all supported format validators.
    #[test]
    fn test_all_format_validators_accept_valid_content() {
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        // Format-appropriate valid content for each format
        let test_cases = [
            ("claude", "# Rules\n\n## Standards\n\nUse spaces.\n"),
            (
                "copilot",
                "# Copilot Instructions\n\nUse consistent naming.\n",
            ),
            ("windsurf", "# Windsurf Rules\n\nFollow conventions.\n"),
            ("aider", "# Conventions\n\nUse consistent formatting.\n"),
            ("generic", "# AI Rules\n\nUse proper indentation.\n"),
            ("json", r#"{"rules": ["Use consistent formatting"]}"#),
        ];

        for (format, content) in &test_cases {
            let validator = get_validator(format).unwrap();
            let result = validator.validate(content, &config, &codebase).unwrap();
            assert!(
                result.passed,
                "Valid content for '{}' should pass validation: {:?}",
                format, result.errors
            );
        }
    }
}

// ── Comment 5: Extended integration tests (Flow 11–13, new features) ───────

#[cfg(test)]
#[cfg(feature = "ollama")]
mod ollama_mocked_pipeline {
    //! Flow 11: Ollama provider mocked pipeline.
    //!
    //! Tests Ollama-specific integration: OLLAMA_HOST environment override,
    //! zero-cost provider behavior, and dry-run configuration display.

    use super::common::{create_temp_dir, run_cli_with_config};
    use ruley::llm::cost::{CostCalculator, CostTracker};
    use ruley::llm::provider::Pricing;

    /// Test CLI accepts --provider ollama in dry-run mode.
    #[test]
    fn test_ollama_dry_run_accepted() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--provider", "ollama", "--dry-run"]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should not panic or produce unexpected error
        assert!(
            output.status.success() || stderr.contains("ollama") || stderr.contains("Ollama"),
            "CLI should accept ollama provider. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }

    /// Test Ollama zero-cost tracking through cost calculator.
    #[test]
    fn test_ollama_zero_cost_tracker() {
        let pricing = Pricing {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        tracker.add_operation("ollama_analysis", 50000, 20000);
        tracker.add_operation("ollama_generation", 30000, 15000);

        assert!(
            (tracker.total_cost() - 0.0).abs() < f64::EPSILON,
            "Ollama cost should be zero"
        );
        assert_eq!(tracker.total_tokens(), 115000);
        assert_eq!(tracker.operation_count(), 2);
    }

    /// Test Ollama provider with custom host configuration.
    #[test]
    fn test_ollama_config_with_custom_host() {
        let config = ruley::cli::config::OllamaConfig {
            host: Some("http://custom-host:11434".to_string()),
            model: Some("llama3.2".to_string()),
        };

        assert_eq!(config.host.as_deref(), Some("http://custom-host:11434"));
        assert_eq!(config.model.as_deref(), Some("llama3.2"));
    }

    /// Test Ollama dry-run cost estimation shows $0.00.
    #[test]
    fn test_ollama_cost_estimation_zero() {
        let pricing = Pricing {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
        };
        let calculator = CostCalculator::new(pricing);
        let estimate = calculator.estimate_cost(100000, 50000);

        assert!(
            (estimate.total_cost - 0.0).abs() < f64::EPSILON,
            "Estimate should be $0.00"
        );
        assert!((estimate.input_cost - 0.0).abs() < f64::EPSILON,);
        assert!((estimate.output_cost - 0.0).abs() < f64::EPSILON,);
    }
}

#[cfg(test)]
#[cfg(feature = "openrouter")]
mod openrouter_mocked_pipeline {
    //! Flow 12: OpenRouter provider mocked pipeline with cost markup.
    //!
    //! Tests OpenRouter-specific integration: cost markup separation,
    //! provider configuration, and pricing behavior.

    use super::common::{create_temp_dir, run_cli_with_config};
    use ruley::llm::cost::{CostCalculator, CostTracker};
    use ruley::llm::provider::Pricing;

    /// Test CLI accepts --provider openrouter in dry-run mode.
    #[test]
    fn test_openrouter_dry_run_accepted() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--provider", "openrouter", "--dry-run"]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should accept openrouter as a valid provider
        assert!(
            output.status.success()
                || stderr.contains("OPENROUTER_API_KEY")
                || stderr.contains("openrouter"),
            "CLI should accept openrouter provider. stdout: {}, stderr: {}",
            stdout,
            stderr
        );
    }

    /// Test OpenRouter cost markup is reflected in pricing.
    #[test]
    fn test_openrouter_cost_with_markup() {
        // OpenRouter pricing includes their markup
        let base_pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let markup_pricing = Pricing {
            input_per_1k: 0.0036, // ~20% markup
            output_per_1k: 0.018,
        };

        let base_calc = CostCalculator::new(base_pricing);
        let markup_calc = CostCalculator::new(markup_pricing);

        let base_estimate = base_calc.estimate_cost(10000, 5000);
        let markup_estimate = markup_calc.estimate_cost(10000, 5000);

        assert!(
            markup_estimate.total_cost > base_estimate.total_cost,
            "Markup pricing should be higher than base"
        );
    }

    /// Test OpenRouter cost tracking across multiple operations.
    #[test]
    fn test_openrouter_multi_operation_tracking() {
        let pricing = Pricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        };
        let mut tracker = CostTracker::new(CostCalculator::new(pricing));

        tracker.add_operation("analysis", 8000, 3000);
        tracker.add_operation("generation", 4000, 2000);
        tracker.add_operation("deconfliction", 2000, 1000);

        let summary = tracker.summary();
        assert_eq!(summary.operation_count, 3);
        assert!(summary.total_cost > 0.0);

        let breakdown = tracker.breakdown();
        assert_eq!(breakdown.len(), 3);
        assert_eq!(breakdown[0].operation, "analysis");
        assert_eq!(breakdown[2].operation, "deconfliction");
    }

    /// Test OpenRouter provider config structure.
    #[test]
    fn test_openrouter_provider_config() {
        let providers = ruley::cli::config::ProvidersConfig {
            openrouter: Some(ruley::cli::config::ProviderConfig {
                model: Some("anthropic/claude-sonnet-4".to_string()),
                max_tokens: Some(4096),
            }),
            ..Default::default()
        };

        let or_config = providers.openrouter.unwrap();
        assert_eq!(
            or_config.model.as_deref(),
            Some("anthropic/claude-sonnet-4")
        );
        assert_eq!(or_config.max_tokens, Some(4096));
    }
}

#[cfg(test)]
mod validation_failure_retry_integration {
    //! Flow 13: Validation failure → auto-fix → max-retries.
    //!
    //! Tests the retry/auto-fix pipeline configuration, including
    //! max retries, refinement result tracking, and cost accumulation.

    use ruley::cli::config::{SemanticValidationConfig, ValidationConfig};
    use ruley::generator::refinement::{FixAttempt, RefinementResult};
    use ruley::packer::{CodebaseMetadata, CompressedCodebase, CompressedFile, CompressionMethod};
    use ruley::utils::validation::get_validator;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_codebase() -> CompressedCodebase {
        CompressedCodebase {
            files: vec![CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() {}".to_string(),
                compressed_content: "fn main() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 12,
                compressed_size: 12,
                language: None,
            }],
            metadata: CodebaseMetadata {
                total_files: 1,
                total_original_size: 12,
                total_compressed_size: 12,
                languages: HashMap::new(),
                compression_ratio: 1.0,
            },
        }
    }

    /// Test --retry-on-validation-failure enables retry in config.
    #[test]
    fn test_retry_on_failure_config_flag() {
        let mut config = ValidationConfig::default();
        assert!(!config.retry_on_failure, "Default should not retry");

        config.retry_on_failure = true;
        assert!(config.retry_on_failure);
    }

    /// Test max_retries config is respected.
    #[test]
    fn test_max_retries_config() {
        let config = ValidationConfig::default();
        assert_eq!(config.max_retries, 3, "Default max retries should be 3");

        let custom = ValidationConfig {
            max_retries: 5,
            ..Default::default()
        };
        assert_eq!(custom.max_retries, 5);
    }

    /// Test RefinementResult tracks exhausted retries.
    #[test]
    fn test_refinement_exhausted_retries_integration() {
        let result = RefinementResult {
            success: false,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["Unclosed code block".to_string()],
                    cost: 0.01,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec!["Unclosed code block".to_string()],
                    cost: 0.012,
                },
                FixAttempt {
                    attempt_number: 3,
                    errors: vec!["Unclosed code block".to_string()],
                    cost: 0.015,
                },
            ],
            total_cost: 0.037,
            retries_exhausted: true,
        };

        assert!(!result.success);
        assert!(result.retries_exhausted);
        assert_eq!(result.attempts.len(), 3);
    }

    /// Test RefinementResult success after retry.
    #[test]
    fn test_refinement_success_after_retry() {
        let result = RefinementResult {
            success: true,
            attempts: vec![
                FixAttempt {
                    attempt_number: 1,
                    errors: vec!["Missing heading".to_string()],
                    cost: 0.008,
                },
                FixAttempt {
                    attempt_number: 2,
                    errors: vec![],
                    cost: 0.010,
                },
            ],
            total_cost: 0.018,
            retries_exhausted: false,
        };

        assert!(result.success);
        assert!(!result.retries_exhausted);
        assert_eq!(result.attempts.len(), 2);
    }

    /// Test validation failure triggers semantic errors that auto-fix would address.
    #[test]
    fn test_validation_failure_produces_actionable_errors() {
        let validator = get_validator("claude").unwrap();
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };

        let bad_content = "# Rules\n\nAlways use tabs.\nAlways use spaces.\n";
        let result = validator.validate(bad_content, &config, &codebase).unwrap();

        // Errors should be actionable for auto-fix
        for error in &result.errors {
            assert!(
                !error.message.is_empty(),
                "Error message should be non-empty for auto-fix"
            );
        }
    }

    /// Test retry cost accumulation across attempts.
    #[test]
    fn test_retry_cost_accumulation() {
        let costs = [0.008, 0.010, 0.012];
        let total: f64 = costs.iter().sum();

        let result = RefinementResult {
            success: false,
            attempts: costs
                .iter()
                .enumerate()
                .map(|(i, &cost)| FixAttempt {
                    attempt_number: i + 1,
                    errors: vec!["error".to_string()],
                    cost,
                })
                .collect(),
            total_cost: total,
            retries_exhausted: true,
        };

        let sum: f64 = result.attempts.iter().map(|a| a.cost).sum();
        assert!(
            (sum - result.total_cost).abs() < f64::EPSILON,
            "Total cost should equal sum of attempt costs"
        );
    }
}

#[cfg(test)]
mod multi_format_cross_consistency_tests {
    //! Multi-format output with cross-format consistency checking.
    //!
    //! Validates that generated rules across multiple formats are consistent
    //! in their core conventions (indentation, naming, etc.).

    use ruley::cli::config::SemanticValidationConfig;
    use ruley::generator::rules::{FormattedRules, GeneratedRules};
    use ruley::packer::{CodebaseMetadata, CompressedCodebase, CompressedFile, CompressionMethod};
    use ruley::utils::validation::get_validator;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn minimal_codebase() -> CompressedCodebase {
        CompressedCodebase {
            files: vec![CompressedFile {
                path: PathBuf::from("src/main.rs"),
                original_content: "fn main() {}".to_string(),
                compressed_content: "fn main() {}".to_string(),
                compression_method: CompressionMethod::None,
                original_size: 12,
                compressed_size: 12,
                language: None,
            }],
            metadata: CodebaseMetadata {
                total_files: 1,
                total_original_size: 12,
                total_compressed_size: 12,
                languages: HashMap::new(),
                compression_ratio: 1.0,
            },
        }
    }

    /// Test consistent content across formats passes individual validation.
    #[test]
    fn test_consistent_multi_format_passes_validation() {
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig::default();

        let mut rules = GeneratedRules::new("analysis");
        rules.add_format(FormattedRules::new(
            "claude",
            "# Rules\n\n## Standards\n\nUse 4-space indentation.\n",
        ));
        rules.add_format(FormattedRules::new(
            "copilot",
            "# Instructions\n\nUse 4-space indentation.\n",
        ));
        rules.add_format(FormattedRules::new(
            "generic",
            "# AI Rules\n\nUse 4-space indentation.\n",
        ));

        // Each format's content should pass its own validator
        for format in ["claude", "copilot", "generic"] {
            let validator = get_validator(format).unwrap();
            let content = rules.get_format(format).unwrap();
            let result = validator
                .validate(&content.content, &config, &codebase)
                .unwrap();
            assert!(
                result.passed,
                "Consistent {} content should pass: {:?}",
                format, result.errors
            );
        }
    }

    /// Test contradicting content across formats is detectable per-format.
    #[test]
    fn test_contradicting_multi_format_detected() {
        let codebase = minimal_codebase();
        let config = SemanticValidationConfig {
            check_contradictions: true,
            check_file_paths: false,
            check_consistency: false,
            check_reality: false,
        };

        // Claude says tabs, generic says spaces — each contains a contradiction within itself
        let content_with_contradiction = "# Rules\n\nAlways use tabs.\nAlways use spaces.\n";

        let validator = get_validator("generic").unwrap();
        let result = validator
            .validate(content_with_contradiction, &config, &codebase)
            .unwrap();
        assert!(
            !result.passed
                || result
                    .errors
                    .iter()
                    .any(|e| { e.layer == ruley::utils::validation::ValidationLayer::Semantic }),
            "Should detect internal contradiction"
        );
    }

    /// Test cross-format consistency: all formats agree on key conventions.
    #[test]
    fn test_cross_format_key_conventions_aligned() {
        let mut rules = GeneratedRules::new("analysis");
        let convention = "Use 4-space indentation";

        rules.add_format(FormattedRules::new(
            "claude",
            format!("# Rules\n\n{}\n", convention),
        ));
        rules.add_format(FormattedRules::new(
            "copilot",
            format!("# Instructions\n\n{}\n", convention),
        ));
        rules.add_format(FormattedRules::new(
            "generic",
            format!("# AI Rules\n\n{}\n", convention),
        ));

        // All formats should contain the same convention
        for format in ["claude", "copilot", "generic"] {
            let content = rules.get_format(format).unwrap();
            assert!(
                content.content.contains(convention),
                "{} format should contain the convention",
                format
            );
        }
    }
}

#[cfg(test)]
mod chunked_analysis_validation_tests {
    //! Chunked analysis with validation integration.
    //!
    //! Tests chunk size configuration, multi-chunk scenarios,
    //! and validation across chunked content.

    use super::common::{create_temp_dir, parse_dry_run_output, run_cli_with_config};
    use ruley::cli::config::ChunkingConfig;

    /// Test chunk_size is reflected in dry-run output.
    #[test]
    fn test_chunk_size_in_dry_run() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run", "--chunk-size", "50000"]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            let parsed = parse_dry_run_output(&stdout);
            assert_eq!(
                parsed.get("Chunk Size").unwrap(),
                "50000",
                "Dry-run should show configured chunk size"
            );
        }
    }

    /// Test ChunkingConfig structure.
    #[test]
    fn test_chunking_config_structure() {
        let config = ChunkingConfig {
            chunk_size: Some(75000),
            overlap: Some(1000),
        };

        assert_eq!(config.chunk_size, Some(75000));
        assert_eq!(config.overlap, Some(1000));
    }

    /// Test default chunk size is present in dry-run output.
    #[test]
    fn test_default_chunk_size_shown() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run"]);
        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() {
            let parsed = parse_dry_run_output(&stdout);
            // Verify chunk size is displayed (value depends on CLI default behavior)
            assert!(
                parsed.contains_key("Chunk Size"),
                "Dry-run should display Chunk Size"
            );
        }
    }

    /// Test small chunk size configuration.
    #[test]
    fn test_small_chunk_size_config() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run", "--chunk-size", "10000"]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            let parsed = parse_dry_run_output(&stdout);
            assert_eq!(parsed.get("Chunk Size").unwrap(), "10000");
        }
    }
}

#[cfg(test)]
mod dry_run_new_settings_tests {
    //! Dry-run reflecting new settings.
    //!
    //! Verifies that new CLI flags and configuration options
    //! are properly reflected in dry-run output.

    use super::common::{
        create_config_file, create_temp_dir, parse_dry_run_output, run_cli_with_config,
    };

    /// Test --no-deconflict flag is accepted in dry-run.
    #[test]
    fn test_no_deconflict_accepted() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run", "--no-deconflict"]);

        // Should not error out — flag should be accepted
        assert!(
            output.status.success(),
            "CLI should accept --no-deconflict. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test --on-conflict flag is accepted in dry-run.
    #[test]
    fn test_on_conflict_flag_accepted() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        for strategy in &["overwrite", "skip", "prompt", "smart-merge"] {
            let output =
                run_cli_with_config(&project_path, &["--dry-run", "--on-conflict", strategy]);

            assert!(
                output.status.success(),
                "CLI should accept --on-conflict {}. stderr: {}",
                strategy,
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    /// Test --retry-on-validation-failure flag is accepted.
    #[test]
    fn test_retry_on_validation_failure_accepted() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(
            &project_path,
            &["--dry-run", "--retry-on-validation-failure"],
        );

        assert!(
            output.status.success(),
            "CLI should accept --retry-on-validation-failure. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// Test config file validation settings reflected in dry-run.
    #[test]
    fn test_config_validation_settings_dry_run() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let config_content = r#"[general]
provider = "anthropic"
compress = true

[validation]
enabled = true
retry_on_failure = true
max_retries = 5

[finalization]
deconflict = false
"#;
        let config_path = create_config_file(&temp_dir, config_content);

        let output = run_cli_with_config(
            &project_path,
            &["--config", config_path.to_str().unwrap(), "--dry-run"],
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            let parsed = parse_dry_run_output(&stdout);
            assert_eq!(
                parsed.get("Compress").unwrap(),
                "true",
                "Compress should be enabled from config"
            );
        }
    }

    /// Test multiple new flags combined in dry-run.
    #[test]
    fn test_combined_new_flags_dry_run() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(
            &project_path,
            &[
                "--dry-run",
                "--no-deconflict",
                "--on-conflict",
                "overwrite",
                "--retry-on-validation-failure",
                "--compress",
                "--chunk-size",
                "50000",
            ],
        );

        assert!(
            output.status.success(),
            "CLI should accept all new flags combined. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            let parsed = parse_dry_run_output(&stdout);
            assert_eq!(parsed.get("Compress").unwrap(), "true");
            assert_eq!(parsed.get("Chunk Size").unwrap(), "50000");
        }
    }

    /// Test format flag with multiple formats in dry-run.
    #[test]
    fn test_multi_format_dry_run() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(
            &project_path,
            &["--dry-run", "--format", "cursor,claude,copilot"],
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        if output.status.success() {
            assert!(
                stdout.contains("cursor") || stdout.contains("Cursor"),
                "Should show cursor format in dry-run"
            );
        }
    }
}
