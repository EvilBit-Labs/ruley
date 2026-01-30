//! Integration tests for the caching and state management system.
//!
//! These tests verify the full lifecycle of the cache manager and state persistence,
//! testing multiple operations in realistic scenarios.

use chrono::{TimeZone, Utc};
use ruley::utils::cache::{CachedFileEntry, TempFileManager, ensure_gitignore_entry};
use ruley::utils::state::{
    CURRENT_STATE_VERSION, ConflictAction, State, UserSelections, load_state, save_state,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a TempDir for tests.
fn create_test_project() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Test the full cache manager lifecycle: create, write, read, cleanup.
#[test]
fn test_cache_manager_lifecycle() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    // 1. Create cache manager (should create .ruley/ directory)
    let manager = TempFileManager::new(project_root).expect("Failed to create TempFileManager");

    // Verify .ruley directory was created
    let ruley_dir = project_root.join(".ruley");
    assert!(
        ruley_dir.exists(),
        ".ruley directory should exist after manager creation"
    );
    assert!(ruley_dir.is_dir(), ".ruley should be a directory");

    // Verify the manager points to the correct directory
    assert_eq!(manager.ruley_dir(), ruley_dir);

    // 2. Write some scanned files
    let scanned_files = vec![
        CachedFileEntry {
            path: PathBuf::from("src/main.rs"),
            size: 1024,
            language: Some("Rust".to_string()),
        },
        CachedFileEntry {
            path: PathBuf::from("src/lib.rs"),
            size: 2048,
            language: Some("Rust".to_string()),
        },
        CachedFileEntry {
            path: PathBuf::from("README.md"),
            size: 512,
            language: None,
        },
    ];

    let files_path = manager
        .write_scanned_files(&scanned_files)
        .expect("Failed to write scanned files");
    assert!(files_path.exists(), "files.json should be created");

    // 3. Write compressed codebase
    let compressed_content = r#"
=== File: src/main.rs ===
fn main() {
    println!("Hello, world!");
}

=== File: src/lib.rs ===
pub fn greet() -> &'static str {
    "Hello!"
}
"#;

    let compressed_path = manager
        .write_compressed_codebase(compressed_content)
        .expect("Failed to write compressed codebase");
    assert!(compressed_path.exists(), "compressed.txt should be created");

    // 4. Write chunk results (simulate multi-chunk analysis)
    let chunk_0 = r#"{"chunk_id": 0, "analysis": "Main entry point detected"}"#;
    let chunk_1 = r#"{"chunk_id": 1, "analysis": "Library module with greet function"}"#;

    let chunk_0_path = manager
        .write_chunk_result(0, chunk_0)
        .expect("Failed to write chunk 0");
    let chunk_1_path = manager
        .write_chunk_result(1, chunk_1)
        .expect("Failed to write chunk 1");

    assert!(chunk_0_path.exists(), "chunk-0.json should be created");
    assert!(chunk_1_path.exists(), "chunk-1.json should be created");

    // 5. Read chunk results back and verify content
    let read_chunk_0 = manager
        .read_chunk_result(0)
        .expect("Failed to read chunk 0");
    let read_chunk_1 = manager
        .read_chunk_result(1)
        .expect("Failed to read chunk 1");

    assert_eq!(read_chunk_0, chunk_0, "Chunk 0 content should match");
    assert_eq!(read_chunk_1, chunk_1, "Chunk 1 content should match");

    // Read compressed codebase back
    let read_compressed = manager
        .read_compressed_codebase()
        .expect("Failed to read compressed codebase");
    assert_eq!(
        read_compressed, compressed_content,
        "Compressed content should match"
    );

    // Read scanned files back
    let read_files = manager
        .read_scanned_files()
        .expect("Failed to read scanned files");
    assert_eq!(read_files.len(), 3, "Should have 3 scanned files");
    assert_eq!(read_files[0].path, PathBuf::from("src/main.rs"));

    // Create state.json to simulate state persistence
    let state_path = ruley_dir.join("state.json");
    std::fs::write(&state_path, r#"{"version": "1.0.0", "important": true}"#)
        .expect("Failed to write state.json");

    // 6. Cleanup temp files (preserve state)
    let cleanup_result = manager
        .cleanup_temp_files(true)
        .expect("Failed to cleanup temp files");

    // Should have deleted: files.json, compressed.txt, chunk-0.json, chunk-1.json
    assert_eq!(cleanup_result.deleted, 4, "Should delete 4 temp files");
    assert!(
        cleanup_result.is_clean(),
        "Should have no failures or skips"
    );

    // 7. Verify temp files are gone but state.json is preserved
    assert!(
        !files_path.exists(),
        "files.json should be deleted after cleanup"
    );
    assert!(
        !compressed_path.exists(),
        "compressed.txt should be deleted after cleanup"
    );
    assert!(
        !chunk_0_path.exists(),
        "chunk-0.json should be deleted after cleanup"
    );
    assert!(
        !chunk_1_path.exists(),
        "chunk-1.json should be deleted after cleanup"
    );
    assert!(
        state_path.exists(),
        "state.json should be preserved after cleanup"
    );
}

/// Test that .gitignore is properly created and updated.
#[test]
fn test_gitignore_entry_created() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();
    let gitignore_path = project_root.join(".gitignore");

    // 1. Create project without .gitignore
    assert!(
        !gitignore_path.exists(),
        ".gitignore should not exist initially"
    );

    // 2. Call ensure_gitignore_entry
    ensure_gitignore_entry(project_root).expect("Failed to ensure gitignore entry");

    // 3. Verify .gitignore exists and contains .ruley/
    assert!(gitignore_path.exists(), ".gitignore should be created");

    let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");
    assert!(
        content.contains(".ruley/"),
        ".gitignore should contain .ruley/ entry"
    );
    assert!(
        content.ends_with('\n'),
        ".gitignore should end with newline"
    );

    // 4. Call again - should be idempotent (no duplicate)
    ensure_gitignore_entry(project_root).expect("Second call should succeed");

    let content_after =
        std::fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");

    // Count occurrences of .ruley/
    let count = content_after.matches(".ruley/").count();
    assert_eq!(
        count, 1,
        "Should have exactly one .ruley/ entry after multiple calls"
    );

    // Content should be unchanged
    assert_eq!(
        content, content_after,
        ".gitignore content should be unchanged on second call"
    );
}

/// Test that .gitignore entry is properly appended to existing file.
#[test]
fn test_gitignore_appends_to_existing() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();
    let gitignore_path = project_root.join(".gitignore");

    // Create existing .gitignore with some content
    let existing_content = "node_modules/\ntarget/\n*.log\n";
    std::fs::write(&gitignore_path, existing_content).expect("Failed to write .gitignore");

    // Call ensure_gitignore_entry
    ensure_gitignore_entry(project_root).expect("Failed to ensure gitignore entry");

    // Verify .ruley/ was appended
    let content = std::fs::read_to_string(&gitignore_path).expect("Failed to read .gitignore");

    // Should contain all original entries
    assert!(content.contains("node_modules/"));
    assert!(content.contains("target/"));
    assert!(content.contains("*.log"));

    // Should also contain .ruley/
    assert!(content.contains(".ruley/"));

    // .ruley/ should be at the end
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.last(), Some(&".ruley/"));
}

/// Test state persistence across different cache manager instances.
#[test]
fn test_state_persistence_across_managers() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    // 1. Create first cache manager
    let manager1 = TempFileManager::new(project_root).expect("Failed to create first manager");

    // 2. Create and save a state
    let fixed_time = Utc.with_ymd_and_hms(2026, 1, 29, 15, 30, 0).unwrap();
    let original_state = State {
        version: CURRENT_STATE_VERSION.to_string(),
        last_run: fixed_time,
        user_selections: UserSelections {
            file_conflict_action: Some(ConflictAction::SmartMerge),
            apply_to_all: true,
        },
        output_files: vec![
            PathBuf::from("output/CLAUDE.md"),
            PathBuf::from("output/.cursorrules"),
        ],
        cost_spent: 0.0567,
        token_count: 123456,
        compression_ratio: 0.72,
    };

    save_state(&original_state, manager1.ruley_dir()).expect("Failed to save state");

    // Verify state.json exists
    let state_path = manager1.ruley_dir().join("state.json");
    assert!(state_path.exists(), "state.json should be created");

    // 3. Create a NEW cache manager (same directory)
    let manager2 = TempFileManager::new(project_root).expect("Failed to create second manager");

    // Managers should point to the same directory
    assert_eq!(manager1.ruley_dir(), manager2.ruley_dir());

    // 4. Load state using the new manager's ruley_dir
    let loaded_state = load_state(manager2.ruley_dir())
        .expect("Should not error on load")
        .expect("State should exist");

    // 5. Verify state matches
    assert_eq!(
        loaded_state.version, original_state.version,
        "Version should match"
    );
    assert_eq!(
        loaded_state.last_run, original_state.last_run,
        "Last run time should match"
    );
    assert_eq!(
        loaded_state.user_selections.file_conflict_action,
        original_state.user_selections.file_conflict_action,
        "Conflict action should match"
    );
    assert_eq!(
        loaded_state.user_selections.apply_to_all, original_state.user_selections.apply_to_all,
        "Apply to all should match"
    );
    assert_eq!(
        loaded_state.output_files, original_state.output_files,
        "Output files should match"
    );
    assert!(
        (loaded_state.cost_spent - original_state.cost_spent).abs() < f32::EPSILON,
        "Cost spent should match"
    );
    assert_eq!(
        loaded_state.token_count, original_state.token_count,
        "Token count should match"
    );
    assert!(
        (loaded_state.compression_ratio - original_state.compression_ratio).abs() < f32::EPSILON,
        "Compression ratio should match"
    );

    // Full equality check
    assert_eq!(loaded_state, original_state, "Full state should match");
}

/// Test that cleanup preserves state across operations.
#[test]
fn test_cleanup_preserves_state_with_temp_files() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    let manager = TempFileManager::new(project_root).expect("Failed to create manager");

    // Write temp files
    manager
        .write_scanned_files(&[CachedFileEntry {
            path: PathBuf::from("test.rs"),
            size: 100,
            language: Some("Rust".to_string()),
        }])
        .expect("Failed to write files");

    manager
        .write_compressed_codebase("test content")
        .expect("Failed to write compressed");

    // Save state
    let state = State::default();
    save_state(&state, manager.ruley_dir()).expect("Failed to save state");

    // Cleanup with preserve_state = true
    let result = manager.cleanup_temp_files(true).expect("Failed to cleanup");
    assert_eq!(result.deleted, 2, "Should delete 2 temp files");

    // State should still be loadable
    let loaded = load_state(manager.ruley_dir())
        .expect("Should not error")
        .expect("State should exist");

    assert_eq!(loaded.version, CURRENT_STATE_VERSION);
}

/// Test state update workflow (save, modify, save again).
#[test]
fn test_state_update_workflow() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    let manager = TempFileManager::new(project_root).expect("Failed to create manager");

    // Initial state
    let state = State {
        cost_spent: 0.01,
        token_count: 1000,
        ..Default::default()
    };

    save_state(&state, manager.ruley_dir()).expect("Failed to save initial state");

    // Load and verify
    let loaded1 = load_state(manager.ruley_dir())
        .expect("Load should succeed")
        .expect("State should exist");
    assert_eq!(loaded1.token_count, 1000);

    // Modify and save again
    let mut state2 = loaded1;
    state2.cost_spent = 0.05;
    state2.token_count = 5000;
    state2.user_selections.file_conflict_action = Some(ConflictAction::Overwrite);

    save_state(&state2, manager.ruley_dir()).expect("Failed to save updated state");

    // Load and verify updates
    let loaded2 = load_state(manager.ruley_dir())
        .expect("Load should succeed")
        .expect("State should exist");

    assert_eq!(loaded2.token_count, 5000);
    assert!(
        (loaded2.cost_spent - 0.05).abs() < f32::EPSILON,
        "Cost should be updated"
    );
    assert_eq!(
        loaded2.user_selections.file_conflict_action,
        Some(ConflictAction::Overwrite)
    );
}

/// Test handling of missing state file.
#[test]
fn test_missing_state_returns_none() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    let manager = TempFileManager::new(project_root).expect("Failed to create manager");

    // Don't save any state
    let loaded = load_state(manager.ruley_dir()).expect("Load should not error");

    assert!(loaded.is_none(), "Missing state should return None");
}

/// Test handling of corrupted state file.
#[test]
fn test_corrupted_state_returns_none() {
    let temp_dir = create_test_project();
    let project_root = temp_dir.path();

    let manager = TempFileManager::new(project_root).expect("Failed to create manager");

    // Write corrupted JSON
    let state_path = manager.ruley_dir().join("state.json");
    std::fs::write(&state_path, "{ invalid json }").expect("Failed to write corrupted file");

    // Load should return None gracefully
    let loaded = load_state(manager.ruley_dir()).expect("Load should not error on corruption");

    assert!(
        loaded.is_none(),
        "Corrupted state should return None, not error"
    );
}

/// Test multiple managers operating on different projects.
#[test]
fn test_independent_project_caches() {
    let project_a = create_test_project();
    let project_b = create_test_project();

    // Create managers for different projects
    let manager_a = TempFileManager::new(project_a.path()).expect("Failed to create manager A");
    let manager_b = TempFileManager::new(project_b.path()).expect("Failed to create manager B");

    // Write different content to each
    manager_a
        .write_compressed_codebase("Project A content")
        .expect("Failed to write A");
    manager_b
        .write_compressed_codebase("Project B content")
        .expect("Failed to write B");

    // Save different states
    let state_a = State {
        token_count: 100,
        ..Default::default()
    };

    let state_b = State {
        token_count: 200,
        ..Default::default()
    };

    save_state(&state_a, manager_a.ruley_dir()).expect("Failed to save state A");
    save_state(&state_b, manager_b.ruley_dir()).expect("Failed to save state B");

    // Verify isolation
    let content_a = manager_a
        .read_compressed_codebase()
        .expect("Failed to read A");
    let content_b = manager_b
        .read_compressed_codebase()
        .expect("Failed to read B");

    assert_eq!(content_a, "Project A content");
    assert_eq!(content_b, "Project B content");

    let loaded_a = load_state(manager_a.ruley_dir())
        .expect("Load A failed")
        .expect("State A should exist");
    let loaded_b = load_state(manager_b.ruley_dir())
        .expect("Load B failed")
        .expect("State B should exist");

    assert_eq!(loaded_a.token_count, 100);
    assert_eq!(loaded_b.token_count, 200);
}
