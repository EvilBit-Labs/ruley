//! Integration tests for ruley CLI.
//!
//! These tests verify the end-to-end behavior of the ruley application,
//! including codebase packing, LLM integration, and rule generation.

mod common;

use std::process::Command;

/// Verify the binary can be invoked and shows help.
#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "CLI should exit successfully with --help"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ruley") || stdout.contains("Usage"),
        "Help output should contain program name or usage"
    );
}

/// Verify the binary shows version information.
#[test]
fn test_cli_version() {
    let output = Command::new("cargo")
        .args(["run", "--", "--version"])
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "CLI should exit successfully with --version"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("0.1.0") || stdout.contains("ruley"),
        "Version output should contain version number"
    );
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
