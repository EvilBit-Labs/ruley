//! State management module for persisting user preferences and run metadata.
//!
//! This module manages persistent state stored in `.ruley/state.json`:
//! - User preferences for file conflict resolution
//! - Last run timestamp
//! - Cost/token/compression metrics from the last run
//! - Output file paths from the last run

use crate::utils::error::RuleyError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Current state file version for migration support.
pub const CURRENT_STATE_VERSION: &str = "1.0.0";

/// State file name within the `.ruley/` directory.
const STATE_FILE: &str = "state.json";

/// Action to take when a file conflict is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictAction {
    /// Overwrite the existing file
    Overwrite,
    /// Attempt smart merge of content
    SmartMerge,
    /// Skip writing this file
    Skip,
}

/// User selections and preferences persisted across runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UserSelections {
    /// The action to take when a file conflict is detected.
    pub file_conflict_action: Option<ConflictAction>,
    /// Whether to apply the conflict action to all files without prompting.
    pub apply_to_all: bool,
}

/// Persistent state for the ruley CLI.
///
/// This state is persisted to `.ruley/state.json` and preserves:
/// - User preferences for file conflict resolution
/// - Metrics from the last run (cost, tokens, compression ratio)
/// - Output file paths from the last run
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct State {
    /// Version of the state file format for migrations.
    pub version: String,
    /// Timestamp of the last run.
    pub last_run: DateTime<Utc>,
    /// User selections and preferences.
    pub user_selections: UserSelections,
    /// Output files generated in the last run.
    pub output_files: Vec<PathBuf>,
    /// Total cost spent in the last run (USD). Must be >= 0.0.
    pub cost_spent: f32,
    /// Total token count from the last run.
    pub token_count: usize,
    /// Compression ratio (compressed_size / original_size).
    ///
    /// Values range from 0.0 to 1.0 where:
    /// - 0.3 means 70% size reduction (compressed to 30% of original)
    /// - 1.0 means no compression (100% of original size)
    ///
    /// Must be in range 0.0..=1.0.
    pub compression_ratio: f32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            version: CURRENT_STATE_VERSION.to_string(),
            last_run: Utc::now(),
            user_selections: UserSelections::default(),
            output_files: Vec::new(),
            cost_spent: 0.0,
            token_count: 0,
            compression_ratio: 1.0,
        }
    }
}

impl State {
    /// Validate that all fields contain sensible values.
    ///
    /// # Returns
    /// `Ok(())` if all fields are valid, or `Err` with a description of the issue.
    ///
    /// # Validated Constraints
    /// - `compression_ratio` must be in range 0.0..=1.0
    /// - `cost_spent` must be >= 0.0
    pub fn validate(&self) -> Result<(), RuleyError> {
        if !(0.0..=1.0).contains(&self.compression_ratio) {
            return Err(RuleyError::State(format!(
                "compression_ratio must be between 0.0 and 1.0, got {}",
                self.compression_ratio
            )));
        }
        if self.cost_spent < 0.0 {
            return Err(RuleyError::State(format!(
                "cost_spent must be >= 0.0, got {}",
                self.cost_spent
            )));
        }
        Ok(())
    }
}

/// Save state to the `.ruley/state.json` file.
///
/// # Arguments
/// * `state` - The state to persist
/// * `ruley_dir` - Path to the `.ruley/` directory
///
/// # Errors
/// Returns `RuleyError::State` if the file cannot be written or serialized.
pub fn save_state(state: &State, ruley_dir: &Path) -> Result<(), RuleyError> {
    let path = ruley_dir.join(STATE_FILE);
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| RuleyError::State(format!("Failed to serialize state: {}", e)))?;

    std::fs::write(&path, json)
        .map_err(|e| RuleyError::State(format!("Failed to write {}: {}", path.display(), e)))?;

    Ok(())
}

/// Load state from the `.ruley/state.json` file.
///
/// # Arguments
/// * `ruley_dir` - Path to the `.ruley/` directory
///
/// # Returns
/// - `Ok(Some(state))` if the file exists and is valid
/// - `Ok(None)` if the file doesn't exist (normal case for first run)
///
/// # Design: Graceful Degradation
///
/// This function intentionally returns `Ok(None)` (not an error) for recoverable
/// issues like corrupted files, permission errors, or schema mismatches. This design
/// allows the CLI to proceed with a fresh state rather than failing. Warnings are
/// logged for these cases so users can investigate if needed.
///
/// The rationale is that state is non-critical - losing preferences is better than
/// blocking the user's workflow.
pub fn load_state(ruley_dir: &Path) -> Result<Option<State>, RuleyError> {
    let path = ruley_dir.join(STATE_FILE);

    // If file doesn't exist, return None
    if !path.exists() {
        return Ok(None);
    }

    // Try to read the file
    let json = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) => {
            tracing::warn!("Failed to read state file {}: {}", path.display(), e);
            return Ok(None);
        }
    };

    // Try to parse the JSON
    match serde_json::from_str::<serde_json::Value>(&json) {
        Ok(value) => {
            // Check version and migrate if needed
            let version = value
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            match migrate_state(value, &version) {
                Ok(state) => {
                    // Validate the loaded state
                    if let Err(e) = state.validate() {
                        tracing::warn!("State validation failed: {}", e);
                        return Ok(None);
                    }
                    Ok(Some(state))
                }
                Err(e) => {
                    tracing::warn!("Failed to migrate state from version '{}': {}", version, e);
                    Ok(None)
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "State file {} is corrupted (invalid JSON): {}",
                path.display(),
                e
            );
            Ok(None)
        }
    }
}

/// Migrate state from an older version to the current version.
///
/// # Arguments
/// * `old_state` - The parsed JSON value of the old state
/// * `from_version` - The version string of the old state
///
/// # Errors
/// Returns `RuleyError::State` if the migration fails.
pub fn migrate_state(
    old_state: serde_json::Value,
    from_version: &str,
) -> Result<State, RuleyError> {
    match from_version {
        // Identity migration for current version (1.0.0)
        CURRENT_STATE_VERSION => serde_json::from_value(old_state)
            .map_err(|e| RuleyError::State(format!("Failed to parse v1.0.0 state: {}", e))),

        // Unknown version - try to parse as current, or return default
        _ => {
            tracing::warn!(
                "Unknown state version '{}', attempting to parse as current version",
                from_version
            );
            serde_json::from_value(old_state).map_err(|e| {
                RuleyError::State(format!(
                    "Failed to parse unknown version '{}' state: {}",
                    from_version, e
                ))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use tempfile::TempDir;

    /// Helper to create a TempDir for tests
    fn create_test_dir() -> TempDir {
        tempfile::tempdir().expect("Failed to create temp dir")
    }

    #[test]
    fn test_state_serialization() {
        // Create a state with specific values
        let fixed_time = Utc.with_ymd_and_hms(2026, 1, 29, 12, 34, 56).unwrap();
        let state = State {
            version: CURRENT_STATE_VERSION.to_string(),
            last_run: fixed_time,
            user_selections: UserSelections {
                file_conflict_action: Some(ConflictAction::Overwrite),
                apply_to_all: true,
            },
            output_files: vec![PathBuf::from("/tmp/rules.md")],
            cost_spent: 0.0234,
            token_count: 45678,
            compression_ratio: 0.68,
        };

        // Serialize to JSON
        let json = serde_json::to_string_pretty(&state).expect("Failed to serialize");

        // Verify the JSON contains expected fields
        assert!(json.contains("\"version\": \"1.0.0\""));
        assert!(json.contains("\"file_conflict_action\": \"overwrite\""));
        assert!(json.contains("\"apply_to_all\": true"));
        assert!(json.contains("\"cost_spent\": 0.0234"));
        assert!(json.contains("\"token_count\": 45678"));
        assert!(json.contains("\"compression_ratio\": 0.68"));

        // Deserialize back
        let deserialized: State = serde_json::from_str(&json).expect("Failed to deserialize");

        // Verify round-trip
        assert_eq!(state, deserialized);
    }

    #[test]
    fn test_save_load_state() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create .ruley dir");

        let fixed_time = Utc.with_ymd_and_hms(2026, 1, 29, 10, 0, 0).unwrap();
        let state = State {
            version: CURRENT_STATE_VERSION.to_string(),
            last_run: fixed_time,
            user_selections: UserSelections {
                file_conflict_action: Some(ConflictAction::SmartMerge),
                apply_to_all: false,
            },
            output_files: vec![PathBuf::from("output/rules.md")],
            cost_spent: 1.5,
            token_count: 100000,
            compression_ratio: 0.7,
        };

        // Save state
        save_state(&state, &ruley_dir).expect("Failed to save state");

        // Verify file exists
        let state_path = ruley_dir.join("state.json");
        assert!(state_path.exists(), "state.json should exist");

        // Load state back
        let loaded = load_state(&ruley_dir)
            .expect("Failed to load state")
            .expect("State should exist");

        // Verify loaded state matches original
        assert_eq!(state, loaded);
    }

    #[test]
    fn test_load_missing_state() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create .ruley dir");

        // Load from directory without state.json
        let result = load_state(&ruley_dir).expect("Should not error");

        // Should return None, not an error
        assert!(result.is_none(), "Missing state file should return None");
    }

    #[test]
    fn test_load_corrupted_state() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create .ruley dir");

        // Write corrupted JSON
        let state_path = ruley_dir.join("state.json");
        std::fs::write(&state_path, "{ invalid json }").expect("Failed to write corrupted file");

        // Load should return None (not error) for corrupted files
        let result = load_state(&ruley_dir).expect("Should not error on corrupted file");

        // Should return None with a warning logged (not an error)
        assert!(
            result.is_none(),
            "Corrupted state file should return None, not error"
        );
    }

    #[test]
    fn test_migrate_v1_to_v1() {
        let fixed_time = Utc.with_ymd_and_hms(2026, 1, 29, 12, 0, 0).unwrap();
        let original_state = State {
            version: "1.0.0".to_string(),
            last_run: fixed_time,
            user_selections: UserSelections {
                file_conflict_action: Some(ConflictAction::Skip),
                apply_to_all: true,
            },
            output_files: vec![PathBuf::from("test.md")],
            cost_spent: 0.5,
            token_count: 5000,
            compression_ratio: 0.8,
        };

        // Convert to JSON Value
        let value = serde_json::to_value(&original_state).expect("Failed to serialize");

        // Migrate (identity migration for v1.0.0)
        let migrated = migrate_state(value, "1.0.0").expect("Migration should succeed");

        // Verify migration preserved all fields
        assert_eq!(original_state, migrated);
    }

    #[test]
    fn test_default_state() {
        let state = State::default();

        assert_eq!(state.version, CURRENT_STATE_VERSION);
        assert!(state.output_files.is_empty());
        assert_eq!(state.cost_spent, 0.0);
        assert_eq!(state.token_count, 0);
        assert_eq!(state.compression_ratio, 1.0);
    }

    #[test]
    fn test_default_user_selections() {
        let selections = UserSelections::default();

        assert!(selections.file_conflict_action.is_none());
        assert!(!selections.apply_to_all);
    }

    #[test]
    fn test_conflict_action_serialization() {
        // Test each variant serializes to snake_case
        let overwrite = serde_json::to_string(&ConflictAction::Overwrite).unwrap();
        let smart_merge = serde_json::to_string(&ConflictAction::SmartMerge).unwrap();
        let skip = serde_json::to_string(&ConflictAction::Skip).unwrap();

        assert_eq!(overwrite, "\"overwrite\"");
        assert_eq!(smart_merge, "\"smart_merge\"");
        assert_eq!(skip, "\"skip\"");

        // Test deserialization
        let parsed: ConflictAction = serde_json::from_str("\"smart_merge\"").unwrap();
        assert_eq!(parsed, ConflictAction::SmartMerge);
    }

    #[test]
    fn test_load_state_with_invalid_schema() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create .ruley dir");

        // Write valid JSON but invalid schema (missing required fields)
        let state_path = ruley_dir.join("state.json");
        std::fs::write(
            &state_path,
            r#"{"version": "1.0.0", "unexpected_field": true}"#,
        )
        .expect("Failed to write file");

        // Load should return None (graceful degradation)
        let result = load_state(&ruley_dir).expect("Should not error on invalid schema");
        assert!(
            result.is_none(),
            "Invalid schema should return None, not error"
        );
    }

    #[test]
    fn test_state_error_display_format() {
        let err = RuleyError::State("test error message".to_string());
        let display = err.to_string();
        assert!(
            display.contains("State error:"),
            "Should contain 'State error:'"
        );
        assert!(
            display.contains("test error message"),
            "Should contain the message"
        );
    }

    #[test]
    fn test_migrate_unknown_version_compatible_schema() {
        let fixed_time = Utc.with_ymd_and_hms(2026, 1, 29, 12, 0, 0).unwrap();
        let state = State {
            version: "2.0.0".to_string(), // Unknown future version
            last_run: fixed_time,
            ..Default::default()
        };

        let value = serde_json::to_value(&state).expect("Failed to serialize");

        // Should still parse if schema is compatible (unknown versions attempt current parse)
        let migrated = migrate_state(value, "2.0.0").expect("Should attempt parse");
        assert_eq!(migrated.last_run, fixed_time);
    }

    #[test]
    fn test_migrate_unknown_version_incompatible_schema_fails() {
        let value = serde_json::json!({
            "version": "99.0.0",
            "completely_different_field": true
        });

        let result = migrate_state(value, "99.0.0");
        assert!(result.is_err(), "Incompatible schema should fail migration");
    }

    #[test]
    fn test_state_validate_valid() {
        let state = State::default();
        assert!(state.validate().is_ok(), "Default state should be valid");
    }

    #[test]
    fn test_state_validate_compression_ratio_out_of_range() {
        let state = State {
            compression_ratio: 1.5,
            ..Default::default()
        };
        assert!(
            state.validate().is_err(),
            "compression_ratio > 1.0 should fail"
        );

        let state = State {
            compression_ratio: -0.1,
            ..Default::default()
        };
        assert!(
            state.validate().is_err(),
            "compression_ratio < 0.0 should fail"
        );
    }

    #[test]
    fn test_state_validate_negative_cost() {
        let state = State {
            cost_spent: -1.0,
            ..Default::default()
        };
        assert!(state.validate().is_err(), "Negative cost_spent should fail");
    }

    #[test]
    fn test_load_state_validates_loaded_data() {
        let temp_dir = create_test_dir();
        let ruley_dir = temp_dir.path().join(".ruley");
        std::fs::create_dir_all(&ruley_dir).expect("Failed to create .ruley dir");

        // Write valid JSON with invalid compression_ratio
        let state_path = ruley_dir.join("state.json");
        std::fs::write(
            &state_path,
            r#"{"version": "1.0.0", "last_run": "2026-01-29T12:00:00Z", "user_selections": {"file_conflict_action": null, "apply_to_all": false}, "output_files": [], "cost_spent": 0.0, "token_count": 0, "compression_ratio": 5.0}"#,
        )
        .expect("Failed to write file");

        // Load should return None due to validation failure
        let result = load_state(&ruley_dir).expect("Should not error");
        assert!(
            result.is_none(),
            "Invalid compression_ratio should return None after validation"
        );
    }
}
