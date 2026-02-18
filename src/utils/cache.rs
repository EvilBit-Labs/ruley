// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Cache management module for .ruley/ directory lifecycle and temp files.
//!
//! This module manages temporary files created during the rule generation pipeline:
//! - `files.json` - Scanned file list
//! - `compressed.txt` - Compressed codebase content
//! - `chunk-{id}.json` - Individual chunk analysis results
//! - `state.json` - Persistent state (preserved across cleanups)

use crate::utils::error::RuleyError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Serializable representation of a file entry for caching.
///
/// This is a simplified version of [`crate::packer::FileEntry`] that can be serialized
/// to JSON. It omits the `content` field which would be redundant in the cache since
/// the compressed content is stored separately in `compressed.txt`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFileEntry {
    /// Path to the file (relative to project root, or absolute)
    pub path: PathBuf,
    /// Size of the file in bytes
    pub size: u64,
    /// Detected programming language (as string identifier, e.g., "rust", "typescript")
    pub language: Option<String>,
}

impl CachedFileEntry {
    /// Create a new cached file entry.
    ///
    /// # Arguments
    /// * `path` - Path to the file (should be non-empty)
    /// * `size` - Size of the file in bytes
    /// * `language` - Optional detected programming language
    ///
    /// # Returns
    /// A new `CachedFileEntry` instance.
    pub fn new(path: PathBuf, size: u64, language: Option<String>) -> Self {
        Self {
            path,
            size,
            language,
        }
    }
}

/// Result of a cleanup operation, tracking both successes and failures.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CleanupResult {
    /// Number of files successfully deleted
    pub deleted: usize,
    /// Number of files that failed to delete
    pub failed: usize,
    /// Number of files skipped (e.g., couldn't read metadata)
    pub skipped: usize,
}

impl CleanupResult {
    /// Returns true if all operations succeeded (no failures or skips)
    pub fn is_clean(&self) -> bool {
        self.failed == 0 && self.skipped == 0
    }

    /// Returns the total number of files processed
    pub fn total(&self) -> usize {
        self.deleted + self.failed + self.skipped
    }
}

impl std::fmt::Display for CleanupResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} deleted", self.deleted)?;
        if self.failed > 0 {
            write!(f, ", {} failed", self.failed)?;
        }
        if self.skipped > 0 {
            write!(f, ", {} skipped", self.skipped)?;
        }
        Ok(())
    }
}

/// Manages the `.ruley/` directory and temporary files.
///
/// The manager ensures the directory exists with proper permissions
/// (0o700 on Unix for owner read/write/execute only; default ACL on Windows)
/// and provides methods for reading/writing temp files used during pipeline execution.
#[derive(Debug)]
pub struct TempFileManager {
    /// Path to the .ruley/ directory
    ruley_dir: PathBuf,
}

impl TempFileManager {
    /// File name for scanned files list
    const FILES_JSON: &'static str = "files.json";
    /// File name for compressed codebase
    const COMPRESSED_TXT: &'static str = "compressed.txt";
    /// File name for persistent state (preserved during cleanup)
    const STATE_JSON: &'static str = "state.json";
    /// Prefix for chunk result files
    const CHUNK_PREFIX: &'static str = "chunk-";

    /// Create a new TempFileManager, ensuring the `.ruley/` directory exists.
    ///
    /// # Arguments
    /// * `project_root` - Path to the project root directory
    ///
    /// # Errors
    /// Returns `RuleyError::Cache` if the directory cannot be created or permissions cannot be set.
    pub fn new(project_root: &Path) -> Result<Self, RuleyError> {
        let ruley_dir = project_root.join(".ruley");

        // Create directory with proper permissions
        if !ruley_dir.exists() {
            std::fs::create_dir_all(&ruley_dir).map_err(|e| {
                RuleyError::Cache(format!(
                    "Failed to create .ruley directory at {}: {}",
                    ruley_dir.display(),
                    e
                ))
            })?;

            // Set directory permissions to 0o700 (owner read/write/execute only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let permissions = std::fs::Permissions::from_mode(0o700);
                std::fs::set_permissions(&ruley_dir, permissions).map_err(|e| {
                    RuleyError::Cache(format!(
                        "Failed to set permissions on .ruley directory: {}",
                        e
                    ))
                })?;
            }
        }

        Ok(Self { ruley_dir })
    }

    /// Get the path to the `.ruley/` directory.
    pub fn ruley_dir(&self) -> &Path {
        &self.ruley_dir
    }

    /// Write scanned files list to `files.json`.
    ///
    /// # Arguments
    /// * `files` - Slice of `CachedFileEntry` to serialize
    ///
    /// # Returns
    /// Path to the written file on success.
    pub fn write_scanned_files(&self, files: &[CachedFileEntry]) -> Result<PathBuf, RuleyError> {
        let path = self.ruley_dir.join(Self::FILES_JSON);
        let json = serde_json::to_string_pretty(files)
            .map_err(|e| RuleyError::Cache(format!("Failed to serialize scanned files: {}", e)))?;

        std::fs::write(&path, json)
            .map_err(|e| RuleyError::Cache(format!("Failed to write {}: {}", path.display(), e)))?;

        Ok(path)
    }

    /// Read scanned files list from `files.json`.
    ///
    /// # Returns
    /// Vector of `CachedFileEntry` on success.
    pub fn read_scanned_files(&self) -> Result<Vec<CachedFileEntry>, RuleyError> {
        let path = self.ruley_dir.join(Self::FILES_JSON);
        let json = std::fs::read_to_string(&path)
            .map_err(|e| RuleyError::Cache(format!("Failed to read {}: {}", path.display(), e)))?;

        serde_json::from_str(&json)
            .map_err(|e| RuleyError::Cache(format!("Failed to parse {}: {}", path.display(), e)))
    }

    /// Write compressed codebase content to `compressed.txt`.
    ///
    /// # Arguments
    /// * `codebase` - Compressed codebase content as string
    ///
    /// # Returns
    /// Path to the written file on success.
    pub fn write_compressed_codebase(&self, codebase: &str) -> Result<PathBuf, RuleyError> {
        let path = self.ruley_dir.join(Self::COMPRESSED_TXT);

        std::fs::write(&path, codebase)
            .map_err(|e| RuleyError::Cache(format!("Failed to write {}: {}", path.display(), e)))?;

        Ok(path)
    }

    /// Read compressed codebase content from `compressed.txt`.
    ///
    /// # Returns
    /// Compressed codebase content as string on success.
    pub fn read_compressed_codebase(&self) -> Result<String, RuleyError> {
        let path = self.ruley_dir.join(Self::COMPRESSED_TXT);

        std::fs::read_to_string(&path)
            .map_err(|e| RuleyError::Cache(format!("Failed to read {}: {}", path.display(), e)))
    }

    /// Write a chunk analysis result to `chunk-{id}.json`.
    ///
    /// # Arguments
    /// * `chunk_id` - Numeric identifier for the chunk
    /// * `result` - Analysis result content
    ///
    /// # Returns
    /// Path to the written file on success.
    pub fn write_chunk_result(&self, chunk_id: usize, result: &str) -> Result<PathBuf, RuleyError> {
        let filename = format!("{}{}.json", Self::CHUNK_PREFIX, chunk_id);
        let path = self.ruley_dir.join(&filename);

        std::fs::write(&path, result)
            .map_err(|e| RuleyError::Cache(format!("Failed to write {}: {}", path.display(), e)))?;

        Ok(path)
    }

    /// Read a chunk analysis result from `chunk-{id}.json`.
    ///
    /// # Arguments
    /// * `chunk_id` - Numeric identifier for the chunk
    ///
    /// # Returns
    /// Analysis result content as string on success.
    pub fn read_chunk_result(&self, chunk_id: usize) -> Result<String, RuleyError> {
        let filename = format!("{}{}.json", Self::CHUNK_PREFIX, chunk_id);
        let path = self.ruley_dir.join(&filename);

        std::fs::read_to_string(&path)
            .map_err(|e| RuleyError::Cache(format!("Failed to read {}: {}", path.display(), e)))
    }

    /// Clean up temporary files in the `.ruley/` directory.
    ///
    /// # Arguments
    /// * `preserve_state` - If true, preserve `state.json`; if false, delete all files
    ///   in the `.ruley/` directory (not just known temp file types)
    ///
    /// # Returns
    /// A `CleanupResult` with counts of deleted, failed, and skipped files.
    pub fn cleanup_temp_files(&self, preserve_state: bool) -> Result<CleanupResult, RuleyError> {
        let mut result = CleanupResult::default();

        let entries = std::fs::read_dir(&self.ruley_dir).map_err(|e| {
            RuleyError::Cache(format!(
                "Failed to read .ruley directory {}: {}",
                self.ruley_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| RuleyError::Cache(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                result.skipped += 1;
                continue;
            };

            if preserve_state && filename == Self::STATE_JSON {
                continue;
            }

            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("Failed to delete temp file {}: {}", path.display(), e);
                result.failed += 1;
            } else {
                result.deleted += 1;
            }
        }

        Ok(result)
    }

    /// Clean up temporary files older than the specified age threshold.
    ///
    /// This method always preserves `state.json` regardless of age.
    ///
    /// # Arguments
    /// * `age_threshold` - Files older than this duration will be deleted
    ///
    /// # Returns
    /// A `CleanupResult` with counts of deleted, failed, and skipped files.
    /// Files are skipped if metadata cannot be read or if modification time
    /// appears to be in the future (possible clock skew).
    pub fn cleanup_old_temp_files(
        &self,
        age_threshold: Duration,
    ) -> Result<CleanupResult, RuleyError> {
        let mut result = CleanupResult::default();
        let now = std::time::SystemTime::now();

        let entries = std::fs::read_dir(&self.ruley_dir).map_err(|e| {
            RuleyError::Cache(format!(
                "Failed to read .ruley directory {}: {}",
                self.ruley_dir.display(),
                e
            ))
        })?;

        for entry in entries {
            let entry = entry
                .map_err(|e| RuleyError::Cache(format!("Failed to read directory entry: {}", e)))?;

            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
                result.skipped += 1;
                continue;
            };

            if filename == Self::STATE_JSON {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to get metadata for {}: {}", path.display(), e);
                    result.skipped += 1;
                    continue;
                }
            };

            let modified = match metadata.modified() {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(
                        "Failed to get modification time for {}: {}",
                        path.display(),
                        e
                    );
                    result.skipped += 1;
                    continue;
                }
            };

            let age = match now.duration_since(modified) {
                Ok(d) => d,
                Err(_) => {
                    tracing::debug!(
                        "File {} has modification time in future, skipping (possible clock skew)",
                        path.display()
                    );
                    result.skipped += 1;
                    continue;
                }
            };

            if age > age_threshold {
                if let Err(e) = std::fs::remove_file(&path) {
                    tracing::warn!("Failed to delete old temp file {}: {}", path.display(), e);
                    result.failed += 1;
                } else {
                    result.deleted += 1;
                }
            }
        }

        Ok(result)
    }
}

/// Ensure `.ruley/` is listed in the project's `.gitignore` file.
///
/// This is a standalone function that can be called independently of `TempFileManager`.
///
/// # Arguments
/// * `project_root` - Path to the project root directory
///
/// # Behavior
/// - If `.gitignore` doesn't exist, creates it with `.ruley/` entry
/// - If `.gitignore` exists but doesn't contain `.ruley/`, appends the entry
/// - If `.gitignore` already contains `.ruley/`, does nothing
/// - Whitespace around entries is trimmed during comparison (e.g., `  .ruley/  ` matches `.ruley/`)
///
/// # Known Limitations
///
/// This function has a TOCTOU (time-of-check-time-of-use) race condition: if multiple
/// ruley processes run concurrently in the same directory, they could both see the entry
/// as missing and append duplicates. For a CLI tool typically run once at a time, this
/// is low-impact. Consider using file locking if concurrent execution becomes common.
pub fn ensure_gitignore_entry(project_root: &Path) -> Result<(), RuleyError> {
    let gitignore_path = project_root.join(".gitignore");
    let ruley_entry = ".ruley/";

    if gitignore_path.exists() {
        // Read existing content
        let content = std::fs::read_to_string(&gitignore_path)
            .map_err(|e| RuleyError::Cache(format!("Failed to read .gitignore: {}", e)))?;

        // Check if entry already exists (exact line match)
        let has_entry = content.lines().any(|line| line.trim() == ruley_entry);

        if !has_entry {
            // Append entry with proper newline handling
            let mut new_content = content;
            if !new_content.ends_with('\n') && !new_content.is_empty() {
                new_content.push('\n');
            }
            new_content.push_str(ruley_entry);
            new_content.push('\n');

            std::fs::write(&gitignore_path, new_content)
                .map_err(|e| RuleyError::Cache(format!("Failed to update .gitignore: {}", e)))?;
        }
    } else {
        // Create new .gitignore with entry
        let content = format!("{}\n", ruley_entry);
        std::fs::write(&gitignore_path, content)
            .map_err(|e| RuleyError::Cache(format!("Failed to create .gitignore: {}", e)))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a TempDir for tests
    fn create_test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn test_new_creates_directory() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        // Verify .ruley directory exists
        let ruley_dir = temp_dir.path().join(".ruley");
        assert!(ruley_dir.exists(), ".ruley directory should exist");
        assert!(ruley_dir.is_dir(), ".ruley should be a directory");

        // Verify the manager points to the correct directory
        assert_eq!(manager.ruley_dir(), ruley_dir);

        // Verify permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&ruley_dir).expect("Failed to get metadata");
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "Directory should have 0o700 permissions");
        }
    }

    #[test]
    fn test_new_with_existing_directory() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");

        // Create directory manually first
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create dir");

        // Should succeed with existing directory
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");
        assert_eq!(manager.ruley_dir(), ruley_dir);
    }

    #[test]
    fn test_write_read_scanned_files() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        let files = vec![
            CachedFileEntry {
                path: PathBuf::from("src/main.rs"),
                size: 1024,
                language: Some("rust".to_string()),
            },
            CachedFileEntry {
                path: PathBuf::from("src/lib.rs"),
                size: 512,
                language: Some("rust".to_string()),
            },
            CachedFileEntry {
                path: PathBuf::from("README.md"),
                size: 256,
                language: None,
            },
        ];

        // Write files
        let path = manager
            .write_scanned_files(&files)
            .expect("Failed to write");
        assert!(path.exists(), "files.json should exist");
        assert!(path.ends_with("files.json"));

        // Read back and verify
        let read_files = manager.read_scanned_files().expect("Failed to read");
        assert_eq!(read_files.len(), 3);
        assert_eq!(read_files[0].path, PathBuf::from("src/main.rs"));
        assert_eq!(read_files[0].size, 1024);
        assert_eq!(read_files[0].language, Some("rust".to_string()));
        assert_eq!(read_files[2].language, None);
    }

    #[test]
    fn test_write_read_compressed_codebase() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        let codebase = "fn main() {\n    println!(\"Hello, world!\");\n}";

        // Write codebase
        let path = manager
            .write_compressed_codebase(codebase)
            .expect("Failed to write");
        assert!(path.exists(), "compressed.txt should exist");
        assert!(path.ends_with("compressed.txt"));

        // Read back and verify
        let read_codebase = manager.read_compressed_codebase().expect("Failed to read");
        assert_eq!(read_codebase, codebase);
    }

    #[test]
    fn test_write_read_chunk_result() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        let chunk_0_result = r#"{"analysis": "chunk 0 data"}"#;
        let chunk_1_result = r#"{"analysis": "chunk 1 data"}"#;

        // Write chunks
        let path_0 = manager
            .write_chunk_result(0, chunk_0_result)
            .expect("Failed to write chunk 0");
        let path_1 = manager
            .write_chunk_result(1, chunk_1_result)
            .expect("Failed to write chunk 1");

        assert!(path_0.exists());
        assert!(path_1.exists());
        assert!(path_0.ends_with("chunk-0.json"));
        assert!(path_1.ends_with("chunk-1.json"));

        // Read back and verify
        let read_0 = manager
            .read_chunk_result(0)
            .expect("Failed to read chunk 0");
        let read_1 = manager
            .read_chunk_result(1)
            .expect("Failed to read chunk 1");

        assert_eq!(read_0, chunk_0_result);
        assert_eq!(read_1, chunk_1_result);
    }

    #[test]
    fn test_read_nonexistent_chunk_returns_error() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        let result = manager.read_chunk_result(999);
        assert!(result.is_err(), "Reading non-existent chunk should fail");

        let err = result.unwrap_err();
        assert!(
            matches!(err, RuleyError::Cache(_)),
            "Error should be Cache variant"
        );
    }

    #[test]
    fn test_cleanup_removes_temp_preserves_state() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        // Create various temp files
        manager
            .write_scanned_files(&[])
            .expect("Failed to write files.json");
        manager
            .write_compressed_codebase("test")
            .expect("Failed to write compressed");
        manager
            .write_chunk_result(0, "chunk0")
            .expect("Failed to write chunk");

        // Create state.json manually
        let state_path = manager.ruley_dir().join("state.json");
        std::fs::write(&state_path, r#"{"state": "important"}"#)
            .expect("Failed to write state.json");

        // Cleanup with preserve_state = true
        let result = manager.cleanup_temp_files(true).expect("Failed to cleanup");
        assert_eq!(result.deleted, 3, "Should delete 3 temp files");
        assert!(result.is_clean(), "Should have no failures or skips");

        // Verify state.json is preserved
        assert!(state_path.exists(), "state.json should be preserved");

        // Verify other files are deleted
        assert!(!manager.ruley_dir().join("files.json").exists());
        assert!(!manager.ruley_dir().join("compressed.txt").exists());
        assert!(!manager.ruley_dir().join("chunk-0.json").exists());
    }

    #[test]
    fn test_cleanup_removes_all_when_not_preserving() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        // Create state.json
        let state_path = manager.ruley_dir().join("state.json");
        std::fs::write(&state_path, r#"{"state": "data"}"#).expect("Failed to write state.json");

        // Cleanup with preserve_state = false
        let result = manager
            .cleanup_temp_files(false)
            .expect("Failed to cleanup");
        assert_eq!(result.deleted, 1, "Should delete state.json");

        // Verify state.json is deleted
        assert!(!state_path.exists(), "state.json should be deleted");
    }

    #[test]
    fn test_cleanup_old_files() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        // Create a file
        manager
            .write_compressed_codebase("old content")
            .expect("Failed to write");

        // Create state.json
        let state_path = manager.ruley_dir().join("state.json");
        std::fs::write(&state_path, r#"{"state": "data"}"#).expect("Failed to write state.json");

        // With a zero threshold, all files (except state.json) should be considered "old"
        let result = manager
            .cleanup_old_temp_files(Duration::from_secs(0))
            .expect("Failed to cleanup");

        assert_eq!(result.deleted, 1, "Should delete 1 old file");
        assert!(state_path.exists(), "state.json should be preserved");
        assert!(!manager.ruley_dir().join("compressed.txt").exists());
    }

    #[test]
    fn test_cleanup_old_files_respects_threshold() {
        let temp_dir = create_test_dir();
        let manager = TempFileManager::new(temp_dir.path()).expect("Failed to create manager");

        // Create a file
        manager
            .write_compressed_codebase("recent content")
            .expect("Failed to write");

        // With a large threshold, no files should be deleted (files are very recent)
        let result = manager
            .cleanup_old_temp_files(Duration::from_secs(3600))
            .expect("Failed to cleanup");

        assert_eq!(result.deleted, 0, "Should not delete recent files");
        assert!(manager.ruley_dir().join("compressed.txt").exists());
    }

    #[test]
    fn test_ensure_gitignore_creates_file() {
        let temp_dir = create_test_dir();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Verify .gitignore doesn't exist
        assert!(!gitignore_path.exists());

        // Call ensure_gitignore_entry
        ensure_gitignore_entry(temp_dir.path()).expect("Failed to ensure gitignore");

        // Verify .gitignore was created with the entry
        assert!(gitignore_path.exists());
        let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read");
        assert!(content.contains(".ruley/"));
        assert!(content.ends_with('\n'));
    }

    #[test]
    fn test_ensure_gitignore_appends_entry() {
        let temp_dir = create_test_dir();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing .gitignore without .ruley/ entry
        std::fs::write(&gitignore_path, "node_modules/\ntarget/\n")
            .expect("Failed to write gitignore");

        // Call ensure_gitignore_entry
        ensure_gitignore_entry(temp_dir.path()).expect("Failed to ensure gitignore");

        // Verify .ruley/ was appended
        let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read");
        assert!(content.contains("node_modules/"));
        assert!(content.contains("target/"));
        assert!(content.contains(".ruley/"));
    }

    #[test]
    fn test_ensure_gitignore_appends_to_file_without_trailing_newline() {
        let temp_dir = create_test_dir();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing .gitignore without trailing newline
        std::fs::write(&gitignore_path, "node_modules/").expect("Failed to write gitignore");

        // Call ensure_gitignore_entry
        ensure_gitignore_entry(temp_dir.path()).expect("Failed to ensure gitignore");

        // Verify proper formatting
        let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read");
        assert_eq!(content, "node_modules/\n.ruley/\n");
    }

    #[test]
    fn test_ensure_gitignore_no_duplicate() {
        let temp_dir = create_test_dir();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing .gitignore with .ruley/ entry
        let original_content = "node_modules/\n.ruley/\ntarget/\n";
        std::fs::write(&gitignore_path, original_content).expect("Failed to write gitignore");

        // Call ensure_gitignore_entry
        ensure_gitignore_entry(temp_dir.path()).expect("Failed to ensure gitignore");

        // Verify no duplicate was added
        let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read");
        assert_eq!(content, original_content);

        // Count occurrences of .ruley/
        let count = content.matches(".ruley/").count();
        assert_eq!(count, 1, "Should only have one .ruley/ entry");
    }

    #[test]
    fn test_ensure_gitignore_handles_whitespace_variations() {
        let temp_dir = create_test_dir();
        let gitignore_path = temp_dir.path().join(".gitignore");

        // Create existing .gitignore with .ruley/ entry (with leading/trailing whitespace)
        // Note: We only match exact ".ruley/" on trimmed lines
        let original_content = "node_modules/\n  .ruley/  \ntarget/\n";
        std::fs::write(&gitignore_path, original_content).expect("Failed to write gitignore");

        // Call ensure_gitignore_entry
        ensure_gitignore_entry(temp_dir.path()).expect("Failed to ensure gitignore");

        // Verify no duplicate was added (trimmed comparison should match)
        let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read");
        assert_eq!(content, original_content);
    }
}
