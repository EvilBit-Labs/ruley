//! Common test utilities and fixtures for integration tests.

use std::collections::HashMap;
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

/// Creates a temporary ruley.toml config file with specified content.
pub fn create_config_file(dir: &TempDir, content: &str) -> PathBuf {
    let config_path = dir.path().join("ruley.toml");
    std::fs::write(&config_path, content).expect("Failed to write config file");
    config_path
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

/// Parse `--dry-run` output into a `key -> value` map.
///
/// The dry-run summary prints lines like:
/// `Compress:     true`
/// `Chunk Size:   100000`
///
/// This helper extracts those `Key: value` pairs for precise assertions.
pub fn parse_dry_run_output(stdout: &str) -> HashMap<String, String> {
    let mut parsed = HashMap::new();

    for line in stdout.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let mut value = value.trim().to_string();

        // Normalize common cases for stability in assertions.
        if let Some(stripped) = value.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            value = stripped.to_string();
        }
        if value.eq_ignore_ascii_case("true") {
            value = "true".to_string();
        } else if value.eq_ignore_ascii_case("false") {
            value = "false".to_string();
        }

        parsed.insert(key.to_string(), value);
    }

    parsed
}
