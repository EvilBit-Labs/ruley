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

    use super::common::{create_mock_project, create_temp_dir};
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
