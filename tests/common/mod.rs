//! Common test utilities and fixtures for integration tests.

use std::path::PathBuf;
use tempfile::TempDir;

/// Path to the `ruley` binary built by Cargo for integration tests.
pub fn ruley_bin() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_ruley") {
        return PathBuf::from(path);
    }

    // Fallback (best-effort): target/{debug|release}/ruley
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));

    let exe = if cfg!(windows) { "ruley.exe" } else { "ruley" };
    let subdir = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    target_dir.join(subdir).join(exe)
}

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

/// Standard Python project files for testing.
#[allow(dead_code)]
pub fn python_project_files() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "setup.py",
            r#"from setuptools import setup

setup(
    name="test-project",
    version="0.1.0",
    packages=["src"],
    python_requires=">=3.8",
)
"#,
        ),
        (
            "src/__init__.py",
            r#""""Test module."""

__version__ = "0.1.0"
"#,
        ),
        (
            "src/main.py",
            r#"def greet(name: str) -> str:
    """Greet someone by name."""
    return f"Hello, {name}!"


if __name__ == "__main__":
    print(greet("World"))
"#,
        ),
        (
            "src/utils.py",
            r#""""Utility functions."""


def add(a: int, b: int) -> int:
    """Add two numbers."""
    return a + b


def multiply(a: int, b: int) -> int:
    """Multiply two numbers."""
    return a * b
"#,
        ),
    ]
}

/// Helper to create individual files with content in a directory.
#[allow(dead_code)]
pub fn create_file_with_content(dir: &TempDir, path: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.path().join(path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).expect("Failed to create parent directories");
    }
    std::fs::write(&file_path, content).expect("Failed to write file");
    file_path
}

/// Helper to create repomix files for testing.
#[allow(dead_code)]
pub fn create_repomix_file(dir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    create_file_with_content(dir, filename, content)
}

/// Creates a temporary ruley.toml config file with specified content.
pub fn create_config_file(dir: &TempDir, content: &str) -> PathBuf {
    let config_path = dir.path().join("ruley.toml");
    std::fs::write(&config_path, content).expect("Failed to write config file");
    config_path
}

/// Parses dry-run output into key-value pairs.
///
/// Extracts lines matching `Key: Value` or `Key:  Value` patterns from stdout.
/// Strips tree-drawing Unicode prefixes (├─, └─, │) before parsing.
/// Used by dry-run tests to verify configuration is displayed correctly.
pub fn parse_dry_run_output(stdout: &str) -> std::collections::HashMap<String, String> {
    let mut result = std::collections::HashMap::new();

    for line in stdout.lines() {
        // Strip ANSI escape codes, then trim whitespace and tree-drawing chars
        let stripped = strip_ansi_codes(line);
        let trimmed = stripped
            .trim()
            .trim_start_matches(|c: char| !c.is_ascii_alphabetic());

        // Match lines like "Key:     Value" or "Key: Value"
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim();
            let value = trimmed[colon_pos + 1..].trim();

            // Only capture non-empty keys that start with an alphabetic character
            if !key.is_empty()
                && !value.is_empty()
                && key.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            {
                result.insert(key.to_string(), value.to_string());
            }
        }
    }

    result
}

/// Strip ANSI escape codes from a string.
fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until we find a letter that ends the escape sequence
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Runs the CLI with specified arguments and captures output.
///
/// Uses a controlled set of environment variables to avoid test pollution.
/// Only passes essential variables (PATH, HOME) and RULEY_* variables.
pub fn run_cli_with_config(dir: &PathBuf, args: &[&str]) -> std::process::Output {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR should be set in tests");

    let mut cmd = std::process::Command::new(ruley_bin());
    cmd.arg(dir);
    cmd.args(args);
    cmd.current_dir(&manifest_dir);

    // Use a controlled environment to avoid test pollution.
    // Only pass essential system variables and RULEY_* variables.
    cmd.env_clear();
    for (key, value) in std::env::vars() {
        if key == "PATH"
            || key == "HOME"
            || key == "CARGO_MANIFEST_DIR"
            || key.starts_with("RULEY_")
        {
            cmd.env(&key, &value);
        }
    }

    cmd.output().expect("Failed to execute command")
}
