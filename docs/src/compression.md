# Tree-Sitter Compression

[TOC]

ruley uses tree-sitter grammars to compress source code before sending it to the LLM. This reduces token count by approximately 70%, significantly lowering costs for large codebases.

## How It Works

Tree-sitter parses source files into abstract syntax trees (ASTs). ruley walks these ASTs to extract structural elements -- function signatures, type definitions, class declarations, imports -- while removing implementation bodies. The result is a compressed representation that preserves the project's API surface and architecture while discarding the details.

### Before Compression

```rust
pub fn analyze_codebase(path: &Path, config: &Config) -> Result<Analysis> {
    let files = scan_files(path, config)?;
    let mut analysis = Analysis::new();
    for file in &files {
        let content = std::fs::read_to_string(&file.path)?;
        let tokens = tokenize(&content);
        analysis.add_file(file, tokens);
    }
    analysis.finalize()
}
```

### After Compression

```rust
pub fn analyze_codebase(path: &Path, config: &Config) -> Result<Analysis> { ... }
```

The LLM sees the function signature, return type, and parameter types -- enough to understand the codebase's API surface without the implementation details.

## Supported Languages

Each language requires a tree-sitter grammar compiled into ruley via Cargo feature flags:

| Language   | Feature Flag                       | Grammar Version               |
| ---------- | ---------------------------------- | ----------------------------- |
| TypeScript | `compression-typescript` (default) | tree-sitter-typescript 0.23.2 |
| Python     | `compression-python`               | tree-sitter-python 0.25.0     |
| Rust       | `compression-rust`                 | tree-sitter-rust 0.24.0       |
| Go         | `compression-go`                   | tree-sitter-go 0.25.0         |

Enable all languages with:

```bash
cargo install ruley --features compression-all
```

Files in unsupported languages are included at full size (no compression applied).

## Usage

Enable compression with the `--compress` flag:

```bash
ruley --compress
```

Or in the config file:

```toml
[general]
compress = true
```

## What Gets Extracted

The compression extracts structural elements that help the LLM understand your codebase:

- **Functions**: Signatures, parameters, return types
- **Types**: Struct/class definitions, enum variants, type aliases
- **Traits/Interfaces**: Method signatures
- **Imports**: Module dependencies
- **Constants**: Top-level constant definitions
- **Module structure**: File and directory organization

## What Gets Removed

Implementation details that don't affect the LLM's understanding of conventions:

- Function bodies (replaced with `{ ... }`)
- Loop internals
- Conditional branches
- Local variable assignments
- Comments (optional, depending on grammar)

## Compression Metrics

ruley tracks and reports compression statistics:

- **Total files**: Number of files processed
- **Original size**: Total bytes before compression
- **Compressed size**: Total bytes after compression
- **Compression ratio**: Ratio of compressed to original (lower is better)

These metrics are displayed during pipeline execution and in the final summary.

## When to Use Compression

**Use compression when:**

- Your codebase is large (>1000 files or >500K tokens)
- You want to minimize LLM costs
- The codebase has languages with tree-sitter grammar support

**Skip compression when:**

- Your codebase is small (the cost savings are negligible)
- You need the LLM to see implementation details for accurate convention extraction
- Your primary language doesn't have a tree-sitter grammar in ruley

## ABI Compatibility

ruley uses tree-sitter 0.26.x (ABI v15). Language parsers may use slightly older ABI versions:

- tree-sitter-go 0.25.0: ABI v15
- tree-sitter-python 0.25.0: ABI v15
- tree-sitter-rust 0.24.0: ABI v15
- tree-sitter-typescript 0.23.2: ABI v14 (compatible via backward compatibility)

The tree-sitter core library supports backward-compatible ABI versions, so older grammar versions work correctly.
