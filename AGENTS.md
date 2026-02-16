# ruley Agent Guidelines

This document provides unified guidance for AI agents working on the ruley project. It synthesizes all project rules and standards into actionable directives.

## Project Overview

**ruley** is a single-crate Rust CLI tool for generating AI IDE rules from codebases.

### Core Architecture

- **Language**: Rust 2024 Edition
- **Package**: Single crate `ruley` (not a workspace)
- **Linting**: Zero warnings policy with `unsafe_code = "deny"` (allows `#[allow]` in tests)
- **License**: Apache-2.0

### Module Structure

1. **cli/**: Command-line interface with clap argument parsing
2. **packer/**: Repository packing (file discovery, gitignore, git operations, compression)
3. **llm/**: Multi-provider LLM integration (Anthropic, OpenAI, Ollama, OpenRouter, etc.)
4. **generator/**: Rule generation logic and prompt templates
5. **output/**: Multi-format output formatters (Cursor, Claude, Copilot, Windsurf, Aider, etc.)
6. **utils/**: Shared utilities (error types, progress bars)

### Output Pipeline

- **Stage 5 (Formatting)**: LLM generates format-specific content, stored in `GeneratedRules.rules_by_format`
- **Stage 6 (Writing)**: Writer module retrieves formatted content via formatters and writes to disk
- **Formatters don't transform**: They retrieve content from `GeneratedRules` by format name

### Prompt Templates

- **Location**: `prompts/*.md` - excluded from mdformat (uses `{{placeholders}}` and `<tags>` syntax)
- **Placeholders**: Use `{{variable}}` syntax (e.g., `{{analysis}}`, `{{primary_language}}`)
- **XML tags**: Use unescaped `<tag>content</tag>` for LLM context markers

### Architecture Principles

- Provider-agnostic LLM interface via traits
- Format-agnostic rule generation (single analysis, multiple outputs)
- Token-efficient processing with tree-sitter compression
- Local-first design (works without network for packing)

## Code Standards

### Rust 2024 Edition

- Use Rust 2024 Edition features:
  - `let-else` for early returns with pattern matching
  - `if-let` chains for multiple pattern matches
  - Modern async patterns with `async-trait`
  - Strong typing with newtypes and enums

### Code Quality

- **Zero warnings**: All code must pass `cargo clippy -- -D warnings`
- **No unsafe code**: `unsafe_code = "deny"` enforced at package level (tests may use `#[allow(unsafe_code)]` for env var manipulation in Rust 2024)
- **Formatting**: Use standard `rustfmt` with project configuration
- **File size**: Keep files focused and manageable (500-600 lines max when possible)

### Code Organization

- Use trait-based interfaces for provider-agnostic design
- Implement comprehensive error handling (see Error Handling section)
- Use strongly-typed structures with serde for serialization
- Prefer composition over deep trait hierarchies
- Use modules to organize related functionality

### Modern Rust Patterns

- **Ownership**: Prefer borrowing (`&str` over `String` when possible)
- **Iterators**: Use iterator methods for functional-style code
- **Avoid allocations**: Use `Cow<str>` when ownership is conditional
- **Type safety**: Avoid `unwrap()`, use `?` operator; avoid `as` casts, use `TryFrom`/`TryInto`
- **Performance**: Trust the compiler, prefer clear code over micro-optimizations
- **Static regex**: Use `std::sync::LazyLock` for compiled regexes (e.g., `static RE: LazyLock<Regex> = LazyLock::new(|| ...)`)
- **cfg-gated imports**: Place imports used only in `#[cfg(...)]` blocks inside those blocks to avoid unused import warnings
- **Trait object references**: Use `box.as_ref()` to get `&dyn Trait` from `Box<dyn Trait>` (avoids clippy `borrowed_box` warning)

## Error Handling

### Library Usage

ruley uses **both** `thiserror` and `anyhow` for different purposes:

**`thiserror`** - Structured Error Types:

- Use for public error types (`src/utils/error.rs`)
- Use when callers need to match on specific error variants
- Use for errors that need structured data (fields, variants)
- Example: `RuleyError` enum in `utils/error.rs`

**`anyhow`** - Error Context and Convenience:

- Use for internal code (`main.rs`, `cli/*.rs`, internal helpers)
- Use when you don't need to match on specific error types
- Use for error context chaining with `.context()` and `.with_context()`
- Use in top-level error handling in `main()` and CLI entry points

### Error Propagation

- Use `?` operator for error propagation
- Public API functions: return `Result<T, RuleyError>` (thiserror types)
- Internal functions: return `Result<T>` (anyhow::Result) for convenience
- Convert between error types using `From` implementations or `.context()`
- Avoid `unwrap()` and `expect()` in production code
- Use `.context()` or `.with_context()` to add error context in internal code

### Retry Strategy

- Implement exponential backoff for rate-limited requests
- Retry on HTTP 429, 500, 502, 503, 504
- Do NOT retry on 400, 401, 403, or context length exceeded
- Use jitter to prevent thundering herd problems

### Security Considerations

- Don't expose API keys or sensitive tokens in error messages
- Use structured logging for error details
- Implement proper error boundaries

## Async Patterns

### Async Architecture

- Use Tokio runtime for all I/O and task management
- Use `tokio::task::spawn_blocking` for `std::fs` operations in async code
- Prefer channels and ownership transfer over shared mutable state
- Use `?` operator and proper error propagation in async contexts
- Implement proper cleanup and graceful shutdown

### Async Function Patterns

```rust
use anyhow::{Context, Result};
use tokio::time::{Duration, timeout};

async fn generate_rules(codebase: &str) -> Result<GeneratedRules> {
    let provider = get_llm_provider()?;
    let messages = build_prompt_messages(codebase)?;

    let response = provider
        .complete(&messages, &CompletionOptions::default())
        .await
        .context("Failed to generate rules")?;

    parse_rules_from_response(&response)
}
```

### Channel Patterns

- Use bounded channels for backpressure
- Use `tokio::sync::mpsc` for async communication
- Use `tokio::sync::oneshot` for request-response patterns
- Drop sender to signal completion to receiver

### Graceful Shutdown

- Implement shutdown signals using `tokio::sync::Notify` or `oneshot::Sender`
- Use `tokio::select!` for concurrent operations and cancellation
- Handle Ctrl+C with `tokio::signal::ctrl_c()`

### Concurrent Processing

- Use `tokio::sync::Semaphore` for bounded concurrency
- Limit concurrent LLM calls to prevent resource exhaustion
- Use `tokio::spawn` for concurrent tasks with proper error handling

## Configuration Management

### Configuration Architecture

ruley uses hierarchical configuration with multiple sources:

1. **Command-line flags** (highest precedence) - parsed by clap
2. **Environment variables** - automatically read by clap via `env` attribute
3. **Configuration file** (`ruley.toml` in project root or `--config` path) - loaded and merged
4. **Embedded defaults** (lowest precedence) - set via clap `default_value` or `Default` trait

### Clap Integration

- Use `#[arg(env = "RULEY_*")]` to automatically read from environment variables
- Use `value_parser` for validation instead of manual checks
- Load config file first, then parse clap args (clap args override config file)
- Use `ArgAction::Count` for verbosity flags, `ArgAction::SetTrue` for boolean flags

### Configuration Structure

Define configuration using serde with TOML format:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub provider: String,
    pub model: Option<String>,
    pub format: Vec<String>,
    pub compress: bool,
    pub chunk_size: usize,
    pub no_confirm: bool,
}
```

### Configuration Validation

- Use clap's `value_parser` for validation
- Implement additional validation after merging with a `validate()` method
- Provide clear error messages for invalid configurations

## CLI Design

### Command Structure

Use clap v4 with derive macros:

```rust
#[derive(Parser)]
#[command(name = "ruley", about = "Make your codebase ruley")]
pub struct Cli {
    /// Path to repository (local path or remote URL)
    #[arg(default_value = ".")]
    pub path: Option<PathBuf>,

    /// LLM provider
    #[arg(short, long, default_value = "anthropic")]
    pub provider: Provider,

    /// Model to use
    #[arg(short, long)]
    pub model: Option<String>,

    /// Output format(s), comma-separated
    #[arg(short, long, default_value = "cursor")]
    pub format: Vec<OutputFormat>,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Enable tree-sitter compression
    #[arg(long)]
    pub compress: bool,

    /// Skip cost confirmation prompt
    #[arg(long)]
    pub no_confirm: bool,

    /// Show what would be processed without calling LLM
    #[arg(long)]
    pub dry_run: bool,
}
```

### Output Formatting

- Support multiple output formats: Cursor (.mdc), Claude (CLAUDE.md), Copilot, Windsurf, Aider, Generic, JSON
- Use `--format` flag with comma-separated values or `all` for all formats
- Respect `NO_COLOR` and `TERM=dumb` for color handling
- Provide clear error messages with actionable suggestions
- Show progress bars for long-running operations using `indicatif`

### Cost Estimation

- Always show estimated cost before LLM calls (unless `--no-confirm`)
- Require user confirmation for expensive operations
- Display token counts and pricing information transparently

## Testing

### Testing Philosophy

Follow the **test proportionality principle**: Keep only tests for critical functionality and real edge cases. Test code should be shorter than implementation.

**Key Principles:**

- Test critical functionality and real edge cases only
- Delete tests for trivial operations, framework behavior, or hypothetical scenarios
- For small projects: aim for \<10 meaningful tests per feature
- Test code should be shorter than implementation

### Testing Architecture

1. **Unit Tests**: Algorithms and core logic only, minimal scope
2. **Integration Tests**: Primary testing approach with minimal mocking
3. **Snapshot Testing**: insta for CLI outputs and generated rules (only for critical outputs)
4. **Property Testing**: proptest for generative testing of edge cases (only when needed)
5. **Performance Testing**: Criterion benchmarks for token counting and compression (only critical paths)

### Test Organization

- Use standard `cargo test` for test execution
- Use `#[tokio::test]` for async runtime testing
- Use `insta` for snapshot testing of CLI outputs and generated rule files
- Use `assert_cmd` for CLI integration testing

### What to Test

**Do test:**

- Critical functionality and real edge cases
- Error conditions and recovery paths
- Token counting and chunking logic
- Retry logic and error handling
- Cost estimation (critical for user experience)
- Compression ratio targets (~70% token reduction) for representative cases

**Don't test:**

- Trivial operations, framework behavior, or hypothetical scenarios
- Every possible provider configuration or format permutation
- Obvious success cases or trivial formatting details

## Performance

### Performance Targets

- **CPU Usage**: \<5% sustained during continuous monitoring
- **Memory Usage**: \<100MB resident under normal operation
- **Process Enumeration**: \<5s for 10,000+ processes
- **Database Operations**: >1,000 records/sec write rate
- **Alert Latency**: \<100ms per detection rule execution

### Async Design

- Use async-first design with Tokio runtime
- Implement proper task spawning and management
- Use connection pooling for database operations
- Implement backpressure handling

### Memory Management

- Use efficient data structures
- Implement proper cleanup and resource management
- Monitor memory usage in production
- Use streaming for large data sets

### Resource Management

- Use bounded concurrency with `tokio::sync::Semaphore`
- Implement proper resource cleanup
- Use `parking_lot::RwLock` for better performance in high-contention scenarios
- Avoid holding locks across await points

## Security

### Core Security Requirements

- **Principle of Least Privilege**: Components run with minimal required permissions
- **Credential Management**: No hardcoded credentials, prefer environment variables
- **Input Validation**: Comprehensive validation with detailed error messages
- **Attack Surface Minimization**: No network listening, outbound-only connections

### Code Safety

- `unsafe_code = "deny"` enforced at package level (not `forbid`, to allow test exceptions)
- Use safe Rust patterns throughout
- Validate all external inputs
- Implement proper error boundaries

### Data Protection

- Optional command-line redaction for privacy
- Configurable field masking in logs
- Secure credential storage (OS keychain integration)
- Don't expose API keys or sensitive tokens in error messages

### Security Testing

- Test privilege escalation scenarios
- Validate input sanitization
- Test error handling for security-sensitive operations
- Verify no sensitive data leakage in logs

## Cargo.toml Standards

### Package Configuration

- Always use **Rust 2024 Edition**
- Single crate structure (not a workspace)
- Enforce lint policy via `[lints.rust]` to forbid unsafe code

### Dependencies

Key dependencies:

- `tokio = { version = "1", features = ["full"] }` - Async runtime
- `clap = { version = "4", features = ["derive", "env"] }` - CLI parsing
- `serde = { version = "1", features = ["derive"] }` - Serialization
- `llm = { version = "0.1" }` - LLM provider abstraction
- `reqwest = { version = "0.12", features = ["json", "stream"] }` - HTTP client
- `async-trait = "0.1"` - Async trait support
- `thiserror = "2"` - Error types
- `anyhow = "1"` - Error context
- `indicatif = "0.17"` - Progress bars
- `tree-sitter = "0.24"` - Code parsing (optional, feature-gated)

### Feature Flags

- **LLM providers** (feature-gated): `anthropic`, `openai`, `ollama`, `openrouter`, `xai`, `groq`, `gemini`
- **Compression languages** (feature-gated): `compression-typescript`, `compression-python`, `compression-rust`, `compression-go`
- **Default features**: `["anthropic", "openai", "compression-typescript"]`
- **All providers**: `all-providers` feature enables all LLM provider features
- **All compression**: `compression-all` feature enables all compression language features

## Documentation

### Code Documentation

- Document all public APIs with rustdoc comments
- Use `///` for public items and `//!` for module-level documentation
- Include examples in documentation where appropriate
- Follow standard rustdoc conventions

### README and Project Documentation

- Keep README.md up to date with current project status
- Include clear installation and usage instructions
- Document the CLI interface and usage examples
- Provide examples of common use cases

### Configuration Documentation

- Document all configuration options
- Include examples of `ruley.toml` configuration files
- Explain the hierarchical configuration precedence
- Document environment variables and their purposes (API keys, etc.)

### API Documentation

- Use comprehensive rustdoc comments for all public interfaces
- Include error conditions and return values
- Provide usage examples for complex APIs (LLM providers, formatters)
- Document performance characteristics where relevant (token counting, compression)

## GitHub Actions (CI/CD)

### Workflow Organization

- Use clear, descriptive workflow names
- Organize workflows by purpose (build, test, deploy, security)
- Use consistent naming conventions (e.g., `build-and-test.yml`, `deploy-prod.yml`)
- Keep workflows focused and modular

### Triggers and Concurrency

- Use appropriate `on` triggers for each workflow purpose
- Implement `concurrency` for critical workflows to prevent race conditions
- Use `workflow_dispatch` for manual triggers with input parameters

### Security

- **NEVER** hardcode secrets in workflow files
- Use `${{ secrets.SECRET_NAME }}` for all sensitive data
- Set explicit `permissions` at workflow level with least privilege principle
- Pin actions to full commit SHA or major version tags (e.g., `@v4`)
- Avoid `main` or `latest` tags

### Performance

- Use `actions/cache@v5` for package manager dependencies
- Design effective cache keys using `hashFiles()` for optimal hit rates
- Use `fetch-depth: 1` for most builds (only latest commit)
- Use matrix strategies for parallel testing across environments

### Testing

- Run unit tests early in the pipeline (fastest feedback)
- Follow with integration tests using `services` for dependencies
- Run E2E tests against staging environment when possible
- Upload comprehensive test reports as artifacts

## Development Workflow

- Use standard `cargo` commands for development
- Run `cargo clippy -- -D warnings` before committing
- Use `cargo test` for running all tests
- Use `cargo build --release` for optimized builds
- Always run linting before committing
- Use `just` commands when available (see justfile-standards.mdc)

### CI Requirements

- `just ci-check` must be completely green - no warnings, not just no errors
- Keep `clippy.toml` MSRV in sync with `Cargo.toml` `rust-version`
- Pre-existing warnings must be fixed, not ignored
- GitHub CI runs clippy with `--all-features`: run `cargo clippy --all-targets --all-features -- -D warnings` before pushing to catch all warnings

## Justfile Standards

### Core Commands

When creating the justfile, include these standard commands:

- `just fmt`: Format all code with `cargo fmt`
- `just fmt-check`: Check formatting without modifying files
- `just lint`: Run rustfmt check and clippy with zero warnings policy
- `just build`: Build the project (`cargo build`)
- `just build-release`: Build optimized release (`cargo build --release`)
- `just check`: Check the project without building (`cargo check`)
- `just test`: Run all tests (`cargo test`)
- `just clippy`: Run clippy with zero warnings (`cargo clippy -- -D warnings`)

### Shell Configuration

- Use `set shell := ["bash", "-eu", "-o", "pipefail", "-c"]` for strict error handling
- Ensure all commands fail fast on errors
- Use proper argument passing with `{{ args }}`

## Key Principles Summary

1. **Zero warnings policy**: All code must pass `cargo clippy -- -D warnings`
2. **No unsafe code**: `unsafe_code = "deny"` enforced at package level
3. **Error handling**: Use `thiserror` for public APIs, `anyhow` for internal code
4. **Async-first**: Use Tokio runtime for all I/O operations
5. **Test proportionality**: Test only critical functionality and real edge cases
6. **Security first**: No hardcoded credentials, validate all inputs, minimal privileges
7. **Performance conscious**: Use bounded concurrency, efficient data structures, proper resource management
8. **Documentation**: Document all public APIs, keep README current, provide examples

## Open Source Quality Standards (OSSF Best Practices)

This project has the OSSF Best Practices passing badge. Maintain these standards:

### Every PR must

- Sign off commits with `git commit -s` (DCO enforced by GitHub App)
- Pass CI (clippy, fmt, tests, CodeQL, cargo audit) before merge
- Include tests for new functionality -- this is policy, not optional
- Be reviewed (human or CodeRabbit) for correctness, safety, and style
- Not introduce `unsafe` code, `unwrap()`/`expect()` in library code, or panics

### Every release must

- Have human-readable release notes via git-cliff (not raw git log)
- Use unique SemVer identifiers (`vX.Y.Z` tags)
- Be built reproducibly (pinned toolchain, committed lock files, cargo-dist)

### Security

- Vulnerabilities go through private reporting (GitHub advisories or <support@evilbitlabs.io>), never public issues
- `cargo audit` and `cargo deny` run daily in CI -- fix findings promptly
- Medium+ severity vulnerabilities: we aim to release a fix within 90 days of confirmation (see SECURITY.md for canonical policy)
- `unsafe_code = "forbid"` is enforced project-wide via workspace lints in `Cargo.toml` -- this is a hardening mechanism, not a suggestion
- `docs/src/security-assurance.md` must be updated when new attack surface is introduced

### Documentation

- Public APIs require rustdoc with examples
- CONTRIBUTING.md documents code review criteria, test policy, DCO, and governance
- SECURITY.md documents vulnerability reporting with scope, safe harbor, and PGP key
- AGENTS.md must accurately reflect implemented features (not aspirational)
- `docs/src/release-verification.md` documents artifact signing for users
