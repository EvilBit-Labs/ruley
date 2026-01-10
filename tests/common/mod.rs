//! Common test utilities and fixtures for integration tests.

use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a temporary directory for test fixtures.
///
/// # Returns
///
/// A `TempDir` that will be automatically cleaned up when dropped.
pub fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Creates a mock project structure for testing.
///
/// # Arguments
///
/// * `dir` - The directory to create the mock project in
/// * `files` - A slice of (relative_path, content) tuples
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
