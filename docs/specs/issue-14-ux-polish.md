# Technical Specification: Issue #14 - UX Polish

## Issue Summary

**Issue**: #14 - UX Polish: Progress Bars, Cost Estimation, and Error Messages **Type**: Enhancement **Priority**: Medium (dependency of core pipeline polish) **Dependencies**: Tickets 1-6 (Foundation, Input Processing, LLM Integration, Output Handling)

## Problem Statement

The current ruley CLI lacks user experience polish:

1. **No progress visibility** - Users don't see real-time feedback during long-running operations
2. **Basic cost display** - Cost estimation exists but lacks detailed breakdown and tree formatting
3. **Generic error messages** - Errors show technical details but lack contextual suggestions
4. **No success summary** - Users don't get a comprehensive summary after successful runs
5. **Minimal dry-run output** - Dry-run mode shows basic config but not analysis preview

## Technical Approach

### Architecture Overview

Implement a UX layer that integrates with the existing 10-stage pipeline without modifying core logic:

```
┌─────────────────────────────────────────────────────────────────┐
│                         UX Layer                                │
├─────────────────────────────────────────────────────────────────┤
│  ProgressManager    │  CostDisplay    │  ErrorFormatter         │
│  (MultiProgress)    │  (tree view)    │  (contextual)           │
├─────────────────────────────────────────────────────────────────┤
│                    Pipeline Stages                               │
│  Init → Scanning → Compressing → Analyzing → ... → Complete     │
└─────────────────────────────────────────────────────────────────┘
```

### Key Design Decisions

1. **Centralized ProgressManager** - Single `MultiProgress` instance in `PipelineContext`
2. **Respect flags** - Honor `--quiet`, `--no-confirm`, `--verbose` throughout
3. **Unicode formatting** - Use box-drawing characters for tree views
4. **Graceful degradation** - Progress bars degrade gracefully in non-TTY environments

## Implementation Plan

### Task 1: ProgressManager Implementation

**File**: `src/utils/progress.rs`

Implement `ProgressManager` with `MultiProgress` support:

```rust
pub struct ProgressManager {
    multi: MultiProgress,
    bars: HashMap<String, ProgressBar>,
}

impl ProgressManager {
    pub fn new() -> Self;
    pub fn add_stage(&mut self, name: &str, total: u64) -> ProgressBar;
    pub fn update(&self, stage: &str, current: u64, message: &str);
    pub fn finish(&self, stage: &str, message: &str);
}
```

**Stage-specific templates**:

- **Scanning**: `"[{bar:40.cyan/blue}] {pos}/{len} Scanning files... {msg}"`
- **Compressing**: `"[{bar:40.cyan/blue}] {pos}/{len} Compressing... ({msg})"`
- **Analyzing**: `"{spinner:.green} Analyzing... {msg}"`
- **Formatting**: `"[{bar:40.cyan/blue}] {pos}/{len} Generating {msg} format"`
- **Writing**: `"[{bar:40.cyan/blue}] {pos}/{len} Writing files... {msg}"`

**Acceptance Criteria**:

- [ ] `ProgressManager` struct with thread-safe `MultiProgress`
- [ ] Stage-specific progress bar styles
- [ ] `update()` and `finish()` methods
- [ ] Graceful handling when stdout is not a TTY

### Task 2: Cost Estimation Display

**File**: `src/utils/cost_display.rs` (new)

Implement detailed cost breakdown with tree formatting:

```rust
pub fn display_cost_estimate(
    codebase: &CompressedCodebase,
    chunks: &[Chunk],
    formats: &[String],
    provider: &str,
    pricing: &Pricing,
) -> Result<()>;

pub async fn prompt_confirmation(message: &str) -> Result<bool>;
```

**Output format** (single chunk):

```
Analysis Summary:
├─ Files: 127 files (45 TypeScript, 32 Python, 50 other)
├─ Tokens: 48,234 (before compression: 156,891)
├─ Compression: 69% reduction
├─ Chunks: 1 (within context limit)
├─ Formats: cursor, claude, copilot
└─ Estimated cost: $0.14 (Anthropic Claude Sonnet)

Breakdown:
├─ Initial analysis: $0.12 (48,234 tokens)
└─ Format refinements: $0.02 (3 formats × ~500 tokens each)

Continue? [Y/n]
```

**Output format** (multi-chunk):

```
Analysis Summary:
├─ Files: 487 files (234 TypeScript, 123 Python, 130 other)
├─ Tokens: 234,567 (before compression: 789,123)
├─ Compression: 70% reduction
├─ Chunks: 3 (exceeds context limit)
└─ Estimated cost: $1.87 (Anthropic Claude Sonnet)

Breakdown:
├─ Chunk 1 analysis: $0.58 (78,189 tokens)
├─ Chunk 2 analysis: $0.58 (78,189 tokens)
├─ Chunk 3 analysis: $0.58 (78,189 tokens)
├─ Merge call: $0.08 (~10,000 tokens)
└─ Format refinements: $0.05 (3 formats × ~500 tokens each)

Note: Large codebase requires chunking. Use --include patterns to reduce scope.

Continue? [Y/n]
```

**Acceptance Criteria**:

- [ ] Tree-formatted cost breakdown using Unicode box-drawing characters
- [ ] File breakdown by language
- [ ] Compression ratio display
- [ ] Per-chunk cost breakdown for multi-chunk analysis
- [ ] Helpful notes for large codebases
- [ ] Respects `--no-confirm` and `--quiet` flags
- [ ] Default to Y for confirmation prompt

### Task 3: Contextual Error Formatting

**File**: `src/utils/error.rs` (extend existing)

Add contextual error formatting with suggestions:

```rust
pub fn format_error(error: &RuleyError, verbose: bool) -> String;
```

**Error structure**:

```
⚠ Error: {error_type}

What happened:
├─ Stage: {stage}
├─ Error: {error_message}
└─ Context: {additional_context}

Suggestion:
• {actionable_suggestion_1}
• {actionable_suggestion_2}

For more details, run with --verbose
```

**Specific error formatters**:

1. **Missing API Key**:

```
⚠ Error: API key not found

What happened:
├─ Attempting to connect to Anthropic Claude
└─ No API key found

How to fix:
Set the ANTHROPIC_API_KEY environment variable:

   export ANTHROPIC_API_KEY=sk-...

Get your key at: https://console.anthropic.com/

Alternative:
• Use OpenAI with --provider openai (requires OPENAI_API_KEY)
```

2. **Rate Limited**:

```
⚠ Error: Failed to analyze codebase

What happened:
├─ Stage: Analyzing codebase with Anthropic Claude
├─ Error: Rate limit exceeded (429)
└─ Tokens sent: 48,234

Suggestion:
• Wait 60 seconds and try again
• Or use --provider openai to switch providers
• Or reduce scope with --include patterns
```

3. **Context Limit Exceeded**:

```
⚠ Error: Codebase too large

What happened:
├─ Stage: Analyzing codebase
├─ Tokens: 456,789
└─ Context limit: 200,000 tokens

Suggestion:
• Use --include patterns to reduce scope
• Example: ruley --include "src/**/*.ts"
• Or use --compress to enable tree-sitter compression
```

**Acceptance Criteria**:

- [ ] `format_error()` function with structured output
- [ ] Error-specific suggestions for all `RuleyError` variants
- [ ] Verbose mode shows full error chain and debug info
- [ ] Unicode symbols for visual clarity

### Task 4: Success Summary Display

**File**: `src/utils/summary.rs` (new)

Implement success summary after completion:

```rust
pub fn display_success_summary(
    ctx: &PipelineContext,
    written_paths: &[WriteResult],
    elapsed: Duration,
) -> Result<()>;
```

**Output format**:

```
✓ Rules generated successfully

Output Files:
├─ Cursor: .cursor/rules/project.mdc (3.2 KB)
├─ Claude: CLAUDE.md (2.8 KB)
└─ Copilot: .github/copilot-instructions.md (2.1 KB)

Statistics:
├─ Files analyzed: 127
├─ Tokens processed: 48,234
├─ Compression: 69% reduction
├─ Actual cost: $0.14
└─ Time: 12.3s

Next Steps:
• Restart your AI IDE to load the new rules
• Test AI suggestions in your codebase
• Re-run ruley when conventions change
```

**Acceptance Criteria**:

- [ ] Tree-formatted output file list with sizes
- [ ] Statistics section with actual cost vs estimated
- [ ] Time elapsed display
- [ ] "Next Steps" guidance section
- [ ] Respects `--quiet` flag

### Task 5: Dry-Run Mode Enhancement

**File**: `src/utils/dry_run.rs` (new)

Implement enhanced dry-run display:

```rust
pub fn display_dry_run_summary(
    codebase: &CompressedCodebase,
    formats: &[String],
    config: &MergedConfig,
) -> Result<()>;
```

**Output format**:

```
Dry Run - No LLM calls will be made

Files to be analyzed:
├─ TypeScript (45 files, 12,345 tokens)
│  ├─ src/main.ts (234 tokens)
│  ├─ src/lib.ts (189 tokens)
│  └─ ... (43 more files)
├─ Python (32 files, 8,901 tokens)
│  └─ ...
└─ Other (50 files, 26,988 tokens)

Total: 127 files, 48,234 tokens
Compression: 69% reduction (from 156,891 tokens)
Estimated cost: $0.14

Output formats: cursor, claude, copilot
Output locations:
├─ .cursor/rules/project.mdc
├─ CLAUDE.md
└─ .github/copilot-instructions.md
```

**Acceptance Criteria**:

- [ ] File breakdown by language with token counts
- [ ] First few files per language with "... (N more)" notation
- [ ] Compression statistics
- [ ] Output format and location preview
- [ ] Estimated cost without making LLM calls

### Task 6: Pipeline Integration

**File**: `src/lib.rs` (modify)

Integrate UX components into pipeline:

1. **Add to PipelineContext**:

```rust
pub struct PipelineContext {
    // ... existing fields
    pub progress_manager: Option<ProgressManager>,
    pub start_time: std::time::Instant,
}
```

2. **Stage Integration Points**:

   - **Init**: Initialize `ProgressManager` if not `--quiet`
   - **Scanning**: Progress bar for file discovery
   - **Compressing**: Progress bar for compression
   - **Analyzing**: Spinner for LLM analysis, cost confirmation
   - **Formatting**: Progress bar for each format
   - **Writing**: Progress bar for output files
   - **Complete**: Success summary display

3. **Replace existing displays**:

   - Replace `display_dry_run_config()` with `display_dry_run_summary()`
   - Replace simple cost confirmation with `display_cost_estimate()` + `prompt_confirmation()`
   - Add success summary after Stage 10

**Acceptance Criteria**:

- [ ] Progress bars update during each stage
- [ ] Cost display uses new tree format
- [ ] Success summary shown after completion
- [ ] Dry-run uses enhanced display
- [ ] All displays respect `--quiet` flag

### Task 7: Error Handling Integration

**File**: `src/main.rs` (modify)

Integrate contextual error formatting:

```rust
async fn main() {
    if let Err(e) = run_main().await {
        let formatted = utils::error::format_error(&e, verbose);
        eprintln!("{}", formatted);
        std::process::exit(1);
    }
}
```

**Challenge**: `verbose` flag needs to be accessible before config merging fails.

**Solution**: Parse verbose flag early or default to non-verbose for early errors.

**Acceptance Criteria**:

- [ ] All errors use contextual formatting
- [ ] Verbose mode shows full error chain
- [ ] Early errors (config parsing) handled gracefully

## Test Plan

### Unit Tests

1. **Cost calculation accuracy**:

   - Verify cost calculations match known pricing
   - Test single-chunk and multi-chunk estimates
   - Test format refinement cost estimates

2. **Error formatting**:

   - Verify all `RuleyError` variants have formatters
   - Test verbose vs non-verbose output
   - Verify suggestions are actionable

3. **Progress bar templates**:

   - Verify stage-specific templates compile
   - Test progress update logic

### Integration Tests

1. **Dry-run output**:

   - Run `ruley --dry-run` and verify output format
   - Verify no LLM calls are made
   - Test with different file compositions

2. **Error scenarios**:

   - Missing API key error message
   - Invalid path error message
   - Rate limit simulation (mock)

3. **Success flow**:

   - Complete successful run
   - Verify summary format and statistics

### Manual Tests

1. **Terminal rendering**:

   - Verify progress bars render correctly in various terminals
   - Test behavior with `NO_COLOR=1`
   - Test behavior in non-TTY (piped output)

2. **User confirmation**:

   - Test Y/n prompt behavior
   - Test with `--no-confirm` flag

## Files to Modify/Create

| File                        | Action | Description                                |
| --------------------------- | ------ | ------------------------------------------ |
| `src/utils/progress.rs`     | Modify | Add `ProgressManager` with `MultiProgress` |
| `src/utils/cost_display.rs` | Create | Cost estimation display functions          |
| `src/utils/error.rs`        | Modify | Add `format_error()` and helpers           |
| `src/utils/summary.rs`      | Create | Success summary display                    |
| `src/utils/dry_run.rs`      | Create | Enhanced dry-run display                   |
| `src/utils/mod.rs`          | Modify | Export new modules                         |
| `src/lib.rs`                | Modify | Integrate UX components into pipeline      |
| `src/main.rs`               | Modify | Use contextual error formatting            |

## Success Criteria

1. **Progress visibility**: Users see real-time progress during all pipeline stages
2. **Cost transparency**: Detailed cost breakdown before LLM calls with clear formatting
3. **Actionable errors**: All errors include contextual suggestions
4. **Success clarity**: Clear summary with statistics after successful runs
5. **Dry-run utility**: Comprehensive preview without making LLM calls
6. **Flag respect**: All displays honor `--quiet`, `--verbose`, `--no-confirm`

## Out of Scope

- Core pipeline logic changes (Tickets 1-6)
- LLM provider implementation
- Output format implementation
- Configuration file format changes
- Interactive TUI (beyond progress bars)
- Color themes or customization

## Dependencies

- **Ticket 1**: Foundation (`PipelineContext`, `MergedConfig`, `RuleyError`)
- **Ticket 2**: Input Processing (`CompressedCodebase`, file scanning)
- **Ticket 3**: LLM Integration (`CostCalculator`, `Chunk`, LLM client)
- **Ticket 5**: Output Handling (`WriteResult`, output paths)

**Existing Cargo.toml dependencies** (already present):

- `indicatif = "0.18.3"` - Progress bars
- `console = "0.16.2"` - Terminal styling
- `tokio` - Async I/O for confirmation prompts

## Risks and Mitigations

| Risk                                   | Mitigation                                                     |
| -------------------------------------- | -------------------------------------------------------------- |
| Progress bars flicker in non-TTY       | Detect TTY and use simple output                               |
| Unicode rendering issues               | Test on multiple terminals, provide ASCII fallback             |
| Cost estimation accuracy               | Clear labeling as "estimated", track actual vs estimated       |
| Performance impact of progress updates | Batch updates, use `set_position()` not `inc()` for efficiency |
