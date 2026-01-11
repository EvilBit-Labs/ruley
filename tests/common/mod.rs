//! Common test utilities and fixtures for integration tests.

use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory for test fixtures.
pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Creates a mock project structure for testing.
pub fn create_mock_project(dir: &TempDir, files: &[(&str, &str)]) -> PathBuf {
    let root = dir.path().to_path_buf();

    for (path, content) in files {
        let file_path = root.join(path);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).expect("Failed to create parent directories");
        }
        std::fs::write(&file_path, content).expect("Failed to write file");
    }

    root
}

/// Standard Rust project files for testing.
pub fn rust_project_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "Cargo.toml",
            r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
        ),
        (
            "src/main.rs",
            r#"fn main() {
    println!("Hello, world!");
}
"#,
        ),
        (
            "src/lib.rs",
            r#"pub fn greet() -> &'static str {
    "Hello!"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        assert_eq!(greet(), "Hello!");
    }
}
"#,
        ),
    ]
}

/// Standard TypeScript project files for testing.
pub fn typescript_project_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "package.json",
            r#"{
  "name": "test-project",
  "version": "1.0.0",
  "main": "dist/index.js",
  "scripts": {
    "build": "tsc",
    "test": "jest"
  }
}
"#,
        ),
        (
            "tsconfig.json",
            r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "strict": true,
    "outDir": "./dist"
  }
}
"#,
        ),
        (
            "src/index.ts",
            r#"export function greet(name: string): string {
    return `Hello, ${name}!`;
}
"#,
        ),
    ]
}

/// Creates a temporary ruley.toml config file with specified content.
pub fn create_config_file(dir: &TempDir, content: &str) -> PathBuf {
    let config_path = dir.path().join("ruley.toml");
    std::fs::write(&config_path, content).expect("Failed to write config file");
    config_path
}

/// Runs the CLI with specified arguments and captures output.
pub fn run_cli_with_config(dir: &PathBuf, args: &[&str]) -> std::process::Output {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set in tests");

    let mut cmd = std::process::Command::new("cargo");
    cmd.args(["run", "--"]);
    cmd.arg(dir);
    cmd.args(args);
    cmd.current_dir(&manifest_dir);
    cmd.envs(std::env::vars());
    cmd.output().expect("Failed to execute command")
}
