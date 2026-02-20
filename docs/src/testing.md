# Testing

[TOC]

This chapter covers ruley's testing philosophy, how to run tests, and guidelines for writing new tests.

## Testing Philosophy

ruley follows the **test proportionality principle**: test critical functionality and real edge cases. Test code should be shorter than implementation.

**Do test:**

- Critical functionality and real edge cases
- Error conditions and recovery paths
- Token counting and chunking logic
- Retry logic and error handling
- Cost estimation
- Compression ratio targets (~70% token reduction)

**Don't test:**

- Trivial operations or framework behavior
- Every possible provider/format permutation
- Obvious success cases or trivial formatting

## Running Tests

### All Tests

```bash
just test
```

This runs all tests with `cargo-nextest` and `--all-features`.

### Verbose Output

```bash
just test-verbose
```

### Specific Tests

```bash
# Run a specific test by name
cargo test test_name

# Run tests in a specific module
cargo test packer::

# Run integration tests only
cargo test --test '*'
```

### Coverage

```bash
just coverage
```

Generates an LCOV coverage report at `lcov.info` using `cargo-llvm-cov`.

## Test Organization

### Unit Tests

Unit tests live in the same file as the code they test, inside `#[cfg(test)]` modules:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // ...
    }
}
```

### Integration Tests

Integration tests live in the `tests/` directory and test the CLI as a black box using `assert_cmd`:

```text
tests/
  common/
    mod.rs        # Shared test utilities
  cli_tests.rs    # CLI integration tests
  ...
```

### Test Utilities

The `tests/common/mod.rs` module provides shared helpers for integration tests:

- **Environment isolation**: Uses a denylist pattern (`env_remove`) to strip sensitive variables from subprocess environments
- **Denylisted variables**: `RULEY_*`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `OPENROUTER_API_KEY`, `OLLAMA_HOST`

> **Important**: The denylist uses `env_remove` (not `env_clear()`) because `env_clear()` breaks coverage instrumentation (`LLVM_PROFILE_FILE`), rustflags, and other tooling-injected variables.

### Async Tests

Use `#[tokio::test]` for async tests:

```rust
#[tokio::test]
async fn test_async_operation() {
    let result = some_async_function().await;
    assert!(result.is_ok());
}
```

### Snapshot Tests

ruley uses `insta` for snapshot testing of CLI outputs and generated rules:

```rust
use insta::assert_snapshot;

#[test]
fn test_output_format() {
    let output = generate_output();
    assert_snapshot!(output);
}
```

Update snapshots with:

```bash
cargo insta review
```

## CI Testing

CI runs the full test suite on every push and pull request:

- **Quality**: `just lint-rust` (formatting + clippy)
- **Tests**: `just test` with all features
- **Cross-platform**: Tests on Linux, macOS, and Windows
- **Feature combinations**: Default features, no features, all features
- **MSRV**: Checks compilation with `stable minus 2 releases`
- **Coverage**: Generates and uploads to Codecov

All CI checks must pass before merge. See `.github/workflows/ci.yml` for the full configuration.

## Writing Tests

### Guidelines

1. **Test the behavior, not the implementation** -- Focus on inputs and outputs
2. **Use descriptive test names** -- `test_chunk_size_exceeds_context_triggers_chunking`
3. **One assertion per concept** -- Multiple `assert!` calls are fine, but each test should verify one logical behavior
4. **Avoid mocking when possible** -- Integration tests with real (but controlled) inputs are preferred
5. **Keep tests fast** -- Use small inputs and avoid network calls in unit tests

### Unsafe Code in Tests

Rust 2024 edition makes `std::env::set_var` unsafe due to data race concerns. Tests that manipulate environment variables need `#[allow(unsafe_code)]`:

```rust
#[test]
#[allow(unsafe_code)]
fn test_env_var_override() {
    unsafe { std::env::set_var("RULEY_PROVIDER", "openai") };
    // ... test logic ...
    unsafe { std::env::remove_var("RULEY_PROVIDER") };
}
```
