# Technical Specification: Issue #7 - Caching and State Management

## Issue Summary

| Field        | Value                                                                    |
| ------------ | ------------------------------------------------------------------------ |
| Issue Number | #7                                                                       |
| Title        | Caching and State Management: Temp Files, State Persistence, and Cleanup |
| State        | OPEN                                                                     |
| Labels       | enhancement                                                              |
| Branch       | `feature/caching-state-management`                                       |

## Problem Statement

The ruley CLI tool needs a hybrid caching system to:

1. **Preserve intermediate results** during pipeline execution for error recovery and debugging
2. **Persist user preferences** and metadata across runs for improved UX
3. **Automatically clean up** stale temporary files to prevent disk bloat
4. **Auto-manage .gitignore** to prevent accidental commits of cache files

Currently, the `TempFileRefs` structure is minimal (just a `Vec<PathBuf>`) with no structured storage, no state persistence, and no cleanup policy.

## Technical Approach

### Architecture Overview

```
.ruley/                              # Cache directory (user read/write only)
├── state.json                       # Persistent state across runs
├── files.json                       # Temp: Scanned file list (Stage 2)
├── compressed.txt                   # Temp: Compressed codebase (Stage 3)
├── chunk-0.json                     # Temp: Chunk 0 analysis (Stage 4)
├── chunk-1.json                     # Temp: Chunk 1 analysis (Stage 4)
└── ...
```

### Design Decisions

1. **File format**: JSON for human readability and debugging
2. **Cleanup policy**: 24-hour threshold for old temp files
3. **State versioning**: Support migration for future schema changes
4. **Error handling**: Temp files preserved on error, deleted on success
5. **Async I/O**: Use `tokio::task::spawn_blocking` for `std::fs` operations

### Key Types

```rust
// Cache management (src/utils/cache.rs)
pub struct TempFileManager {
    ruley_dir: PathBuf,
}

// State persistence (src/utils/state.rs)
pub struct State {
    pub version: String, // "1.0.0"
    pub last_run: DateTime<Utc>,
    pub user_selections: UserSelections,
    pub output_files: Vec<PathBuf>,
    pub cost_spent: f32,
    pub token_count: usize,
    pub compression_ratio: f32,
}
```

## Implementation Plan

### Task 1: Create Cache Management Module

**File**: `src/utils/cache.rs`

**Purpose**: Manage `.ruley/` directory lifecycle and temp files

**Implementation**:

```rust
use crate::utils::error::RuleyError;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub struct TempFileManager {
    ruley_dir: PathBuf,
}

impl TempFileManager {
    /// Create a new cache manager, ensuring .ruley/ directory exists
    pub fn new(project_root: &Path) -> Result<Self, RuleyError>;

    /// Get the .ruley/ directory path
    pub fn ruley_dir(&self) -> &Path;

    /// Write scanned files list to files.json
    pub async fn write_scanned_files(&self, files: &[FileEntry]) -> Result<PathBuf, RuleyError>;

    /// Write compressed codebase to compressed.txt
    pub async fn write_compressed_codebase(&self, codebase: &str) -> Result<PathBuf, RuleyError>;

    /// Write chunk analysis result
    pub async fn write_chunk_result(
        &self,
        chunk_id: usize,
        result: &str,
    ) -> Result<PathBuf, RuleyError>;

    /// Read chunk analysis result
    pub async fn read_chunk_result(&self, chunk_id: usize) -> Result<String, RuleyError>;

    /// Delete all temp files (preserve state.json if preserve_state=true)
    pub async fn cleanup_temp_files(&self, preserve_state: bool) -> Result<usize, RuleyError>;

    /// Delete temp files older than threshold (always preserves state.json)
    pub async fn cleanup_old_temp_files(
        &self,
        age_threshold: Duration,
    ) -> Result<usize, RuleyError>;
}

/// Ensure .ruley/ is in .gitignore
pub async fn ensure_gitignore_entry(project_root: &Path) -> Result<(), RuleyError>;
```

**Tests** (TDD - write first):

- `test_new_creates_directory` - verify directory creation with correct permissions
- `test_write_read_scanned_files` - round-trip serialization
- `test_write_read_chunk_result` - round-trip chunk data
- `test_cleanup_removes_temp_preserves_state` - cleanup behavior
- `test_cleanup_old_files` - age-based cleanup
- `test_ensure_gitignore_creates_file` - missing .gitignore
- `test_ensure_gitignore_appends_entry` - existing .gitignore without entry
- `test_ensure_gitignore_no_duplicate` - existing .gitignore with entry

### Task 2: Create State Management Module

**File**: `src/utils/state.rs`

**Purpose**: Persist user preferences and run metadata

**Implementation**:

```rust
use crate::utils::error::RuleyError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CURRENT_STATE_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub version: String,
    pub last_run: DateTime<Utc>,
    pub user_selections: UserSelections,
    pub output_files: Vec<PathBuf>,
    pub cost_spent: f32,
    pub token_count: usize,
    pub compression_ratio: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserSelections {
    pub file_conflict_action: Option<ConflictAction>,
    pub apply_to_all: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictAction {
    Overwrite,
    SmartMerge,
    Skip,
}

impl Default for State {
    fn default() -> Self;
}

/// Save state to state.json (pretty-printed JSON)
pub async fn save_state(state: &State, ruley_dir: &Path) -> Result<(), RuleyError>;

/// Load state from state.json (returns None if missing, default if corrupted)
pub async fn load_state(ruley_dir: &Path) -> Result<Option<State>, RuleyError>;

/// Migrate state from old version to current
pub fn migrate_state(old_state: serde_json::Value, from_version: &str)
-> Result<State, RuleyError>;
```

**Tests** (TDD - write first):

- `test_state_serialization` - serde round-trip
- `test_save_load_state` - file persistence
- `test_load_missing_state` - returns None
- `test_load_corrupted_state` - logs warning, returns default
- `test_migrate_v1_to_v1` - identity migration

### Task 3: Update Utils Module Exports

**File**: `src/utils/mod.rs`

**Changes**:

- Add `pub mod cache;`
- Add `pub mod state;`

### Task 4: Extend PipelineContext

**File**: `src/lib.rs`

**Changes to PipelineContext**:

```rust
pub struct PipelineContext {
    // ... existing fields ...
    pub cache_manager: Option<TempFileManager>,
    pub loaded_state: Option<State>,
}

impl PipelineContext {
    pub fn new(config: MergedConfig) -> Self {
        Self {
            // ... existing initialization ...
            cache_manager: None,
            loaded_state: None,
        }
    }
}
```

### Task 5: Integrate Cache in Pipeline Init (Stage 1)

**File**: `src/lib.rs`

**Changes to `run()` function**:

After config validation in Stage 1:

```rust
// Create cache manager
let cache_manager = TempFileManager::new(&ctx.config.path)?;

// Cleanup old temp files (24-hour threshold)
let old_files_cleaned = cache_manager
    .cleanup_old_temp_files(Duration::from_secs(24 * 3600))
    .await?;
if old_files_cleaned > 0 {
    tracing::info!("Cleaned up {} old temp files", old_files_cleaned);
}

// Ensure .ruley/ is in .gitignore
cache::ensure_gitignore_entry(&ctx.config.path).await?;

// Load previous state
let loaded_state = state::load_state(cache_manager.ruley_dir()).await?;
if let Some(ref state) = loaded_state {
    tracing::debug!("Loaded previous state from {:?}", state.last_run);
}

ctx.cache_manager = Some(cache_manager);
ctx.loaded_state = loaded_state;
```

### Task 6: Write Temp Files During Pipeline

**File**: `src/lib.rs`

**Stage 2 (Scanning)** - after file scanning:

```rust
if let Some(ref cache) = ctx.cache_manager {
    let path = cache.write_scanned_files(&file_entries).await?;
    ctx.temp_files.add(path);
}
```

**Stage 3 (Compressing)** - after compression:

```rust
if let Some(ref cache) = ctx.cache_manager {
    let path = cache.write_compressed_codebase(&compressed_content).await?;
    ctx.temp_files.add(path);
}
```

**Stage 4 (Analyzing)** - after each chunk:

```rust
if let Some(ref cache) = ctx.cache_manager {
    let path = cache.write_chunk_result(chunk_id, &chunk_result).await?;
    ctx.temp_files.add(path);
}
```

### Task 7: State Persistence and Cleanup (Stage 10)

**File**: `src/lib.rs`

**Stage 10 (Cleanup)** - on success:

```rust
if let Some(ref cache) = ctx.cache_manager {
    // Build state from context
    let state = State {
        version: CURRENT_STATE_VERSION.to_string(),
        last_run: Utc::now(),
        user_selections: UserSelections::default(),
        output_files: ctx.output_files.clone(),
        cost_spent: ctx.cost_tracker.as_ref()
            .map(|t| t.summary().total_cost as f32)
            .unwrap_or(0.0),
        token_count: ctx.compressed_codebase.as_ref()
            .map(|c| c.metadata.total_original_size)
            .unwrap_or(0),
        compression_ratio: ctx.compressed_codebase.as_ref()
            .map(|c| c.metadata.compression_ratio)
            .unwrap_or(0.0),
    };

    // Save state
    state::save_state(&state, cache.ruley_dir()).await?;

    // Clean up temp files (preserve state.json)
    let cleaned = cache.cleanup_temp_files(true).await?;
    tracing::info!("Cleaned up {} temp files", cleaned);
}
```

### Task 8: Error Path Preservation

**File**: `src/main.rs`

**In error handling**:

```rust
if let Err(e) = result {
    // Log that temp files are preserved
    eprintln!("Error: {}", e);
    eprintln!("Temp files preserved in .ruley/ for debugging");
    // Do NOT call cleanup - temp files remain
}
```

### Task 9: Add Error Variants

**File**: `src/utils/error.rs`

**Add variants to RuleyError**:

```rust
#[derive(Debug, Error)]
pub enum RuleyError {
    // ... existing variants ...
    #[error("Cache error: {0}")]
    Cache(String),

    #[error("State error: {0}")]
    State(String),
}
```

### Task 10: Integration Tests

**File**: `tests/cache_integration_test.rs`

**Tests**:

- `test_pipeline_creates_temp_files` - verify temp files created during run
- `test_successful_run_cleans_temp_saves_state` - cleanup on success
- `test_error_preserves_temp_files` - no cleanup on error
- `test_gitignore_entry_created` - .gitignore auto-management

## Test Plan

### Unit Tests (in module files)

| Test                                               | Purpose               |
| -------------------------------------------------- | --------------------- |
| `cache::test_new_creates_directory`                | Directory creation    |
| `cache::test_write_read_scanned_files`             | File round-trip       |
| `cache::test_cleanup_removes_temp_preserves_state` | Cleanup logic         |
| `cache::test_cleanup_old_files`                    | Age-based cleanup     |
| `cache::test_ensure_gitignore_*`                   | .gitignore management |
| `state::test_state_serialization`                  | Serde round-trip      |
| `state::test_save_load_state`                      | Persistence           |
| `state::test_load_corrupted_state`                 | Error handling        |

### Integration Tests

| Test                                          | Purpose                       |
| --------------------------------------------- | ----------------------------- |
| `test_pipeline_creates_temp_files`            | End-to-end temp file creation |
| `test_successful_run_cleans_temp_saves_state` | Success path                  |
| `test_error_preserves_temp_files`             | Error path                    |

## Files to Modify/Create

| File                              | Action | Purpose                      |
| --------------------------------- | ------ | ---------------------------- |
| `src/utils/cache.rs`              | Create | TempFileManager              |
| `src/utils/state.rs`              | Create | State persistence            |
| `src/utils/mod.rs`                | Modify | Export new modules           |
| `src/utils/error.rs`              | Modify | Add error variants           |
| `src/lib.rs`                      | Modify | PipelineContext, integration |
| `src/main.rs`                     | Modify | Error path messaging         |
| `tests/cache_integration_test.rs` | Create | Integration tests            |

## Success Criteria

1. `.ruley/` directory created on first run with user-only permissions
2. Temp files (files.json, compressed.txt, chunk-\*.json) written during pipeline
3. Temp files deleted on successful completion
4. Temp files preserved on error for debugging
5. state.json persists across runs with version, timestamps, and metrics
6. `.ruley/` automatically added to .gitignore
7. Old temp files (>24 hours) cleaned on startup
8. All tests pass: `cargo test`
9. Zero clippy warnings: `cargo clippy -- -D warnings`

## Out of Scope

- Progress bars (separate ticket)
- Error handling UI (separate ticket)
- Smart merge conflict resolution logic (future ticket)
- Resume functionality using cached chunks (future ticket)

## Dependencies

- **Ticket #1**: Foundation (Context, Config) - MERGED
- **Ticket #2**: Input Processing (FileEntry, CompressedCodebase) - MERGED
- **Ticket #5**: Packer (repomix parsing) - MERGED

## Technical References

- AGENTS.md: Async patterns, Error handling, File I/O
- src/lib.rs: PipelineContext, TempFileRefs, pipeline stages
- src/utils/error.rs: RuleyError enum pattern
- src/packer/gitignore.rs: Existing gitignore utilities
