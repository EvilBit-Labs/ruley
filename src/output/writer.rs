//! File writing logic for output formatters.
//!
//! This module handles the file writing stage of the pipeline:
//! - Determining output paths (respecting user overrides)
//! - Detecting file conflicts
//! - Creating backup files
//! - Writing output files to disk

use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, get_formatter};
use crate::utils::error::RuleyError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Options for controlling output file writing.
#[derive(Debug, Clone)]
pub struct WriteOptions {
    /// Base directory for output files (usually project root)
    pub base_path: PathBuf,
    /// Custom output paths by format (overrides defaults)
    pub output_paths: HashMap<String, String>,
    /// Whether to create backups of existing files
    pub create_backups: bool,
    /// Whether to overwrite existing files without confirmation
    pub force: bool,
}

impl WriteOptions {
    /// Create new write options with the given base path.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            output_paths: HashMap::new(),
            create_backups: true,
            force: false,
        }
    }

    /// Set custom output paths.
    pub fn with_output_paths(mut self, paths: HashMap<String, String>) -> Self {
        self.output_paths = paths;
        self
    }

    /// Set whether to create backups.
    pub fn with_backups(mut self, create_backups: bool) -> Self {
        self.create_backups = create_backups;
        self
    }

    /// Set whether to force overwrite.
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

/// Result of writing an output file.
#[derive(Debug, Clone)]
pub struct OutputResult {
    /// Format that was written
    pub format: String,
    /// Path where the file was written
    pub path: PathBuf,
    /// Whether a backup was created
    pub backup_created: bool,
    /// Path to the backup file (if created)
    pub backup_path: Option<PathBuf>,
    /// Whether the file was newly created (vs overwritten)
    pub is_new: bool,
}

/// Write output files for all generated formats.
///
/// # Arguments
///
/// * `rules` - The generated rules containing format-specific content
/// * `formats` - List of formats to write
/// * `project_name` - Name of the project (used in metadata)
/// * `options` - Write options controlling paths and behavior
///
/// # Returns
///
/// A vector of results indicating what was written.
pub fn write_output(
    rules: &GeneratedRules,
    formats: &[String],
    project_name: &str,
    options: &WriteOptions,
) -> Result<Vec<OutputResult>, RuleyError> {
    let mut results = Vec::with_capacity(formats.len());

    for format in formats {
        let result = write_format(rules, format, project_name, options)?;
        results.push(result);
    }

    Ok(results)
}

/// Write output for a single format.
fn write_format(
    rules: &GeneratedRules,
    format: &str,
    project_name: &str,
    options: &WriteOptions,
) -> Result<OutputResult, RuleyError> {
    let formatter = get_formatter(format)?;

    let metadata = Metadata {
        project_name: project_name.to_string(),
        format: format.to_string(),
    };

    // Get the formatted content
    let content = formatter.format(rules, &metadata)?;

    // Determine output path
    let output_path = determine_output_path(format, formatter.as_ref(), options);

    // Check for existing file
    let is_new = !output_path.exists();
    let mut backup_created = false;
    let mut backup_path = None;

    // Handle existing file - require --force to overwrite
    if !is_new {
        if !options.force {
            return Err(RuleyError::OutputFormat(format!(
                "Output file already exists: {}. Use --force to overwrite.",
                output_path.display()
            )));
        }
        if options.create_backups {
            let backup = create_backup(&output_path)?;
            backup_created = true;
            backup_path = Some(backup);
        }
    }

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                RuleyError::OutputFormat(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }

    // Write the file
    std::fs::write(&output_path, content).map_err(|e| {
        RuleyError::OutputFormat(format!("Failed to write {}: {}", output_path.display(), e))
    })?;

    tracing::info!("Wrote {} format to {}", format, output_path.display());

    Ok(OutputResult {
        format: format.to_string(),
        path: output_path,
        backup_created,
        backup_path,
        is_new,
    })
}

/// Determine the output path for a format.
fn determine_output_path(
    format: &str,
    formatter: &dyn crate::output::OutputFormatter,
    options: &WriteOptions,
) -> PathBuf {
    // Check for custom path override
    if let Some(custom_path) = options.output_paths.get(format) {
        return options.base_path.join(custom_path);
    }

    // Use formatter defaults
    let dir = formatter.default_directory();
    let filename = formatter.default_filename();
    let ext = formatter.extension();

    let file_with_ext = format!("{}.{}", filename, ext);

    if dir.is_empty() {
        options.base_path.join(file_with_ext)
    } else {
        options.base_path.join(dir).join(file_with_ext)
    }
}

/// Create a backup of an existing file.
///
/// Returns the path to the backup file.
fn create_backup(path: &Path) -> Result<PathBuf, RuleyError> {
    let backup_path = generate_backup_path(path);

    std::fs::copy(path, &backup_path).map_err(|e| {
        RuleyError::OutputFormat(format!(
            "Failed to create backup of {}: {}",
            path.display(),
            e
        ))
    })?;

    tracing::debug!(
        "Created backup: {} -> {}",
        path.display(),
        backup_path.display()
    );

    Ok(backup_path)
}

/// Generate a backup path for a file.
///
/// Uses simple `.bak` suffix: `file.ext` -> `file.ext.bak`
fn generate_backup_path(path: &Path) -> PathBuf {
    let backup_name = format!(
        "{}.bak",
        path.file_name()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
    );

    path.with_file_name(backup_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_determine_output_path_default() {
        let formatter = get_formatter("cursor").unwrap();
        let options = WriteOptions::new("/project");

        let path = determine_output_path("cursor", formatter.as_ref(), &options);
        assert_eq!(path, PathBuf::from("/project/.cursor/rules/project.mdc"));
    }

    #[test]
    fn test_determine_output_path_custom() {
        let formatter = get_formatter("cursor").unwrap();
        let mut output_paths = HashMap::new();
        output_paths.insert("cursor".to_string(), "custom/path/rules.mdc".to_string());

        let options = WriteOptions::new("/project").with_output_paths(output_paths);

        let path = determine_output_path("cursor", formatter.as_ref(), &options);
        assert_eq!(path, PathBuf::from("/project/custom/path/rules.mdc"));
    }

    #[test]
    fn test_generate_backup_path() {
        let path = Path::new("/project/CLAUDE.md");
        let backup = generate_backup_path(path);

        assert_eq!(backup, PathBuf::from("/project/CLAUDE.md.bak"));
    }

    #[test]
    fn test_create_backup() {
        let temp_dir = TempDir::new().unwrap();
        let original = temp_dir.path().join("test.txt");
        fs::write(&original, "original content").unwrap();

        let backup_path = create_backup(&original).unwrap();

        assert!(backup_path.exists());
        assert_eq!(
            fs::read_to_string(&backup_path).unwrap(),
            "original content"
        );
    }

    #[test]
    fn test_write_options_builder() {
        let mut paths = HashMap::new();
        paths.insert("cursor".to_string(), "custom.mdc".to_string());

        let options = WriteOptions::new("/project")
            .with_output_paths(paths.clone())
            .with_backups(false)
            .with_force(true);

        assert_eq!(options.base_path, PathBuf::from("/project"));
        assert_eq!(options.output_paths, paths);
        assert!(!options.create_backups);
        assert!(options.force);
    }
}
