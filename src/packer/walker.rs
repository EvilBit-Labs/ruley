use crate::MergedConfig;
use crate::utils::error::RuleyError;
use globset::{GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

use super::compress::Language;

/// Represents a file discovered during directory scanning.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Path to the file
    pub path: PathBuf,
    /// Size of the file in bytes
    pub size: u64,
    /// Detected programming language
    pub language: Option<Language>,
}

impl FileEntry {
    /// Create a new FileEntry by reading metadata and detecting language.
    pub fn new(path: PathBuf) -> Result<Self, RuleyError> {
        let metadata = std::fs::metadata(&path).map_err(|e| {
            RuleyError::FileSystem(std::io::Error::new(
                e.kind(),
                format!("Failed to read metadata for {}: {}", path.display(), e),
            ))
        })?;

        let language = detect_language(&path);

        Ok(Self {
            path,
            size: metadata.len(),
            language,
        })
    }
}

/// Detect programming language from file extension.
pub(crate) fn detect_language(path: &Path) -> Option<Language> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext_str| match ext_str {
            "ts" | "tsx" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "py" => Some(Language::Python),
            "rs" => Some(Language::Rust),
            "go" => Some(Language::Go),
            "java" => Some(Language::Java),
            "c" | "h" => Some(Language::C),
            "cpp" | "hpp" | "cc" | "cxx" => Some(Language::Cpp),
            "rb" => Some(Language::Ruby),
            "php" => Some(Language::Php),
            _ => None,
        })
}

/// Build a GlobSet from a list of patterns.
fn build_globset(patterns: &[String]) -> Result<GlobSet, RuleyError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = globset::Glob::new(pattern).map_err(|e| {
            RuleyError::Config(format!("Invalid glob pattern '{}': {}", pattern, e))
        })?;
        builder.add(glob);
    }
    builder
        .build()
        .map_err(|e| RuleyError::Config(format!("Failed to build glob set: {}", e)))
}

/// Normalize path to a forward-slash separated string for glob matching.
fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Scan files in a directory with pattern matching and language detection.
pub async fn scan_files(root: &Path, config: &MergedConfig) -> Result<Vec<FileEntry>, RuleyError> {
    // Build glob sets for include and exclude patterns
    let include_set = build_globset(&config.include)?;
    let exclude_set = build_globset(&config.exclude)?;

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .follow_links(false)
        .max_depth(None)
        .build();

    let mut entries = Vec::new();

    for result in walker {
        match result {
            Ok(entry) => {
                // Skip directories
                if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    continue;
                }

                let path = entry.path();

                // Check for symlinks
                if entry.path_is_symlink() {
                    tracing::debug!("Skipping symlink: {}", path.display());
                    continue;
                }

                // Normalize path for glob matching
                let normalized_path = normalize_path(path);

                // Check include patterns: if set is non-empty, require a match
                if !config.include.is_empty() && !include_set.is_match(&normalized_path) {
                    continue;
                }

                // Check exclude patterns: if matches, skip
                if exclude_set.is_match(&normalized_path) {
                    continue;
                }

                // Create FileEntry
                match FileEntry::new(path.to_path_buf()) {
                    Ok(file_entry) => entries.push(file_entry),
                    Err(e) => {
                        // Log warnings for specific errors but continue scanning
                        if matches!(e, RuleyError::FileSystem(ref io_err) if io_err.kind() == std::io::ErrorKind::PermissionDenied)
                        {
                            tracing::warn!("Permission denied: {}", path.display());
                        } else {
                            tracing::warn!("Failed to process file {}: {}", path.display(), e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!("Error walking directory: {}", e);
                continue;
            }
        }
    }

    tracing::info!("Scanned {} files", entries.len());
    Ok(entries)
}

pub struct FileWalker {
    root: std::path::PathBuf,
}

impl FileWalker {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn walk(&self) -> Result<Vec<std::path::PathBuf>, RuleyError> {
        let mut files = Vec::new();

        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = result.map_err(|e| {
                RuleyError::FileSystem(std::io::Error::other(format!(
                    "Failed to walk directory: {}",
                    e
                )))
            })?;
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }
}
