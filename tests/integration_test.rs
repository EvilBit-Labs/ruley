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
    use super::common::{
        create_config_file, create_temp_dir, parse_dry_run_output, run_cli_with_config,
    };

    /// Test that dry-run mode shows configuration without making LLM calls.
    #[test]
    fn test_dry_run_mode() {
        let temp_dir = create_temp_dir();
        let project_path = temp_dir.path().to_path_buf();

        let output = run_cli_with_config(&project_path, &["--dry-run"]);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(stdout.contains("Dry Run Mode"));
        assert!(stdout.contains("No LLM calls will be made"));
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

        assert!(
            stdout.contains("anthropic") || stdout.contains("Anthropic"),
            "CLI provider should override config file"
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
            let old_provider = std::env::var("RULEY_GENERAL_PROVIDER").ok();
            let old_compress = std::env::var("RULEY_GENERAL_COMPRESS").ok();
            unsafe {
                std::env::set_var("RULEY_GENERAL_PROVIDER", "openai");
                std::env::set_var("RULEY_GENERAL_COMPRESS", "true");
            }

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

            // Restore env
            unsafe {
                match old_provider {
                    Some(v) => std::env::set_var("RULEY_GENERAL_PROVIDER", v),
                    None => std::env::remove_var("RULEY_GENERAL_PROVIDER"),
                }
                match old_compress {
                    Some(v) => std::env::set_var("RULEY_GENERAL_COMPRESS", v),
                    None => std::env::remove_var("RULEY_GENERAL_COMPRESS"),
                }
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            if !output.status.success() {
                eprintln!("CLI exited with code: {:?}", output.status.code());
                eprintln!("STDOUT:\n{}", stdout);
                eprintln!("STDERR:\n{}", stderr);
            }
            assert!(output.status.success(), "Expected dry-run to succeed");

            let parsed = parse_dry_run_output(&stdout);

            // Env wins over config
            assert_eq!(parsed.get("Provider").unwrap(), "openai");
            assert_eq!(parsed.get("Compress").unwrap(), "true");
            // CLI wins over env/config for chunk size
            assert_eq!(parsed.get("Chunk Size").unwrap(), "75000");
        }
    }
}
