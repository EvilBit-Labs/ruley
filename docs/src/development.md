# Development Setup

[TOC]

This chapter covers setting up a development environment for contributing to ruley.

## Prerequisites

- **Rust** 1.91+ (see `rust-version` in `Cargo.toml` for minimum supported version)
- **Git** for version control
- **mise** (recommended) for development toolchain management

## Quick Start

```bash
# Clone the repository
git clone https://github.com/EvilBit-Labs/ruley.git
cd ruley

# Install development tools (mise handles everything via mise.toml)
just setup

# Build the project
just build

# Run tests
just test

# Run the CLI
just run --help
```

## Toolchain Management

ruley uses [mise](https://mise.jdx.dev/) to manage the development toolchain. The `mise.toml` file at the project root defines all required tools and versions:

- **Rust** 1.93.1 with rustfmt and clippy components
- **cargo-nextest** for faster test execution
- **cargo-llvm-cov** for code coverage
- **cargo-audit** and **cargo-deny** for security auditing
- **mdbook** and plugins for documentation
- **git-cliff** for changelog generation
- **pre-commit** for pre-commit hooks
- **actionlint** for GitHub Actions linting

Run `mise install` to install all tools, or let `just setup` handle it.

### Without mise

If you prefer not to use mise, install Rust via [rustup](https://rustup.rs/) and install individual tools with `cargo install`:

```bash
rustup toolchain install 1.93.1 --profile default -c rustfmt,clippy
cargo install cargo-nextest cargo-llvm-cov cargo-audit cargo-deny
```

## Development Commands

ruley uses [just](https://github.com/casey/just) as its task runner. Run `just` to see all available recipes:

| Command              | Description                                       |
| -------------------- | ------------------------------------------------- |
| `just test`          | Run tests with nextest (all features)             |
| `just test-verbose`  | Run tests with output                             |
| `just lint`          | Run rustfmt check + clippy (all features)         |
| `just clippy-min`    | Run clippy with no default features               |
| `just check`         | Quick check: pre-commit + lint + build-check      |
| `just ci-check`      | Full CI suite: lint, test, build, audit, coverage |
| `just build`         | Debug build                                       |
| `just build-release` | Release build (all features, LTO)                 |
| `just fmt`           | Format code                                       |
| `just coverage`      | Generate LCOV coverage report                     |
| `just audit`         | Run cargo audit                                   |
| `just deny`          | Run cargo deny checks                             |
| `just outdated`      | Check for outdated dependencies                   |
| `just doc`           | Generate and open rustdoc                         |
| `just docs-serve`    | Serve mdbook docs locally with live reload        |
| `just run <args>`    | Run the CLI with arguments                        |
| `just changelog`     | Generate CHANGELOG.md from git history            |

## IDE Setup

### rust-analyzer

ruley works well with [rust-analyzer](https://rust-analyzer.github.io/). Recommended VS Code settings:

```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": [
    "--all-features"
  ]
}
```

## Project Structure

```text
ruley/
  src/
    cli/          # CLI argument parsing and config management
    packer/       # File discovery, gitignore, compression
    llm/          # LLM providers, tokenization, chunking
    generator/    # Prompt templates and rule parsing
    output/       # Format writers and conflict resolution
    utils/        # Errors, progress, caching, validation
    lib.rs        # Pipeline orchestration (10-stage pipeline)
    main.rs       # Entry point
  tests/          # Integration tests
  benches/        # Criterion benchmarks
  prompts/        # LLM prompt templates (markdown)
  docs/           # mdbook documentation (this book)
  examples/       # Example configuration files
```

## Code Quality

Before submitting changes, ensure:

1. **All tests pass**: `just test`
2. **No clippy warnings**: `just lint` (includes all features) and `just clippy-min` (no default features)
3. **Code is formatted**: `just fmt`
4. **Full CI suite passes**: `just ci-check`

### Lint Policy

ruley enforces a zero-warnings policy. Key lint rules:

- `unsafe_code = "deny"` -- No unsafe code in production (tests may use `#[allow(unsafe_code)]`)
- `unwrap_used = "deny"` -- No `unwrap()` in production code
- `panic = "deny"` -- No `panic!()` in production code
- `pedantic`, `nursery`, `cargo` -- Clippy lint groups at `warn` level

See `[workspace.lints.clippy]` in `Cargo.toml` for the full lint configuration.

## Commit Standards

Follow [Conventional Commits](https://www.conventionalcommits.org):

```text
<type>[(<scope>)]: <description>
```

- **Types**: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`
- **Scope** (optional): `cli`, `packer`, `llm`, `generator`, `output`, `utils`, `config`, `deps`
- **DCO**: Always sign off with `git commit -s`

See [CONTRIBUTING.md](https://github.com/EvilBit-Labs/ruley/blob/main/CONTRIBUTING.md) for full contribution guidelines.
