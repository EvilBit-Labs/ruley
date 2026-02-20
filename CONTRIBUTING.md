# Contributing to ruley

Thank you for your interest in contributing to ruley! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Architecture](#project-architecture)
- [Environment Variables](#environment-variables)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Documentation](#documentation)
- [Submitting Changes](#submitting-changes)
- [Style Guidelines](#style-guidelines)
- [Project Governance](#project-governance)

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- **Rust** (see `rust-version` in `Cargo.toml` for the minimum supported version)
- **Cargo** (comes with Rust)
- **Git** for version control
- **mise** (recommended) for development toolchain management

### Quick Start

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

## Development Setup

### Recommended Tools

- **rust-analyzer**: IDE support for Rust
- **just**: Task runner for development workflows (`mise install` provides this)
- **cargo-nextest**: Faster test runner
- **mdbook**: Documentation building (`cargo install mdbook`)

### Development Commands

```bash
# Using Just (recommended) — run `just` to see all available recipes
just test            # Run tests with nextest (all features)
just lint            # Run rustfmt check + clippy (all features)
just clippy-min      # Run clippy with no default features
just check           # Quick check: pre-commit + lint + build-check
just ci-check        # Full CI suite: lint, test, build, audit, coverage
just build           # Debug build
just build-release   # Release build (all features, LTO)
just fmt             # Format code
just coverage        # Generate LCOV coverage report
just audit           # Run cargo audit
just deny            # Run cargo deny checks
just outdated        # Check for outdated dependencies
just doc             # Generate and open rustdoc
just docs-serve      # Serve mdbook docs locally with live reload
just run <args>      # Run the CLI with arguments
just changelog       # Generate CHANGELOG.md from git history
```

### Building Documentation

```bash
# Serve mdbook documentation locally
just docs-serve

# Generate rustdoc
just doc
```

## Project Architecture

ruley is a single-crate Rust CLI tool. See [AGENTS.md](AGENTS.md) for comprehensive architecture documentation.

### Module Overview

| Module       | Purpose                                                          |
| ------------ | ---------------------------------------------------------------- |
| `cli/`       | Command-line interface with clap argument parsing                |
| `packer/`    | Repository packing (file discovery, gitignore, compression)      |
| `llm/`       | Multi-provider LLM integration (Anthropic, OpenAI, Ollama, etc.) |
| `generator/` | Rule generation logic and prompt templates                       |
| `output/`    | Multi-format output formatters (Cursor, Claude, Copilot, etc.)   |
| `utils/`     | Shared utilities (error types, progress bars, formatting)        |

## Environment Variables

### Provider API Keys

| Variable             | Provider   | Required                                     |
| -------------------- | ---------- | -------------------------------------------- |
| `ANTHROPIC_API_KEY`  | Anthropic  | When using `--provider anthropic`            |
| `OPENAI_API_KEY`     | OpenAI     | When using `--provider openai`               |
| `OLLAMA_HOST`        | Ollama     | Optional (default: `http://localhost:11434`) |
| `OPENROUTER_API_KEY` | OpenRouter | When using `--provider openrouter`           |

### CLI Configuration Overrides

All CLI flags can be set via `RULEY_*` environment variables. CLI flags take precedence over environment variables.

| Variable             | CLI Flag         | Description                    |
| -------------------- | ---------------- | ------------------------------ |
| `RULEY_PROVIDER`     | `--provider`     | LLM provider                   |
| `RULEY_MODEL`        | `--model`        | Model name                     |
| `RULEY_FORMAT`       | `--format`       | Output format(s)               |
| `RULEY_OUTPUT`       | `--output`       | Output file path               |
| `RULEY_CONFIG`       | `--config`       | Config file path               |
| `RULEY_COMPRESS`     | `--compress`     | Enable tree-sitter compression |
| `RULEY_CHUNK_SIZE`   | `--chunk-size`   | Token chunk size               |
| `RULEY_NO_CONFIRM`   | `--no-confirm`   | Skip cost confirmation         |
| `RULEY_DRY_RUN`      | `--dry-run`      | Show plan without calling LLM  |
| `RULEY_DESCRIPTION`  | `--description`  | Rule description               |
| `RULEY_RULE_TYPE`    | `--rule-type`    | Rule type                      |
| `RULEY_ON_CONFLICT`  | `--on-conflict`  | Conflict resolution strategy   |
| `RULEY_REPOMIX_FILE` | `--repomix-file` | Pre-packed repomix file        |

## Making Changes

### Branching Strategy

1. Create a feature branch from `main`:

   ```bash
   git checkout -b feat/your-feature-name
   ```

2. Use conventional commit prefixes:

   - `feat:` - New features
   - `fix:` - Bug fixes
   - `docs:` - Documentation changes
   - `refactor:` - Code refactoring
   - `test:` - Test additions/changes
   - `perf:` - Performance improvements
   - `build:` - Build system changes
   - `ci:` - CI/CD changes
   - `chore:` - Maintenance tasks

### Commit Standards

Follow [Conventional Commits](https://www.conventionalcommits.org):

```text
<type>[(<scope>)]: <description>
```

- **Scope** (optional but recommended): `cli`, `packer`, `llm`, `generator`, `output`, `utils`, `config`, `deps`, etc.
- **Description**: imperative mood ("add", not "added"), no period, \<=72 characters
- **Body** (optional): explain what/why, not how
- **Footer** (optional): `Closes #123` or `BREAKING CHANGE:`

Examples:

- `feat(output): add Windsurf format support`
- `fix(llm): prevent credential leakage in error messages`
- `docs(readme): update installation instructions`
- `refactor(packer): simplify gitignore handling`

### Code Quality Requirements

Before submitting changes, ensure:

1. **All tests pass**: `just test`
2. **No clippy warnings**: `just clippy` (all features) and `just clippy-min` (no default features)
3. **Code is formatted**: `just fmt`
4. **Documentation builds**: `just doc-build`
5. **Full CI suite passes**: `just ci-check`

### Safety Requirements

This project **denies unsafe code**. The following lint is enforced:

```rust
unsafe_code = "deny"
```

Tests may use `#[allow(unsafe_code)]` where necessary (e.g., environment variable manipulation in Rust 2024). If you believe unsafe code is necessary elsewhere, open an issue for discussion first.

## Testing

### Running Tests

```bash
# Run all tests (uses cargo-nextest)
just test

# Run tests with output
just test-verbose

# Run a specific test (using cargo directly)
cargo test test_name

# Run tests for a specific module
cargo test packer::

# Generate coverage report
just coverage
```

### Writing Tests

- Place unit tests in the same file as the code being tested using `#[cfg(test)]` modules
- Place integration tests in the `tests/` directory
- Test critical functionality and real edge cases
- Follow test proportionality: test code should be shorter than implementation
- See the Testing section in [AGENTS.md](AGENTS.md) for detailed guidelines

## Documentation

### Types of Documentation

1. **Rustdoc**: API documentation in source code (`///` comments)
2. **mdbook**: User guide in `docs/`
3. **README.md**: Project overview and quick start
4. **AGENTS.md**: Architecture and development guidelines

### Rustdoc Guidelines

- Document all public items with `///` comments
- Include `# Examples` sections with runnable code
- Add `# Errors` sections for fallible functions
- Use `# Panics` sections if applicable

## Submitting Changes

### Pull Request Process

1. **Update documentation** for any API changes
2. **Add tests** for new functionality
3. **Run the full check suite** locally: `just ci-check` (must be completely green)
4. **Sign off commits** with `git commit -s` (DCO required)
5. **Create a pull request** with a clear description
6. **Address review feedback** promptly

### Code Review Requirements

All pull requests require review before merging. Reviewers check for:

- **Correctness**: Does the code do what it claims? Are edge cases handled?
- **Safety**: No unsafe code, proper error handling, no panics in library code
- **Tests**: New functionality has tests, existing tests still pass
- **Style**: Follows project conventions, passes `cargo fmt` and `cargo clippy`
- **Documentation**: Public APIs have rustdoc, AGENTS.md updated if architecture changes

CI must pass before merge. This includes formatting, linting, tests, security audit, and CodeQL analysis. Branch protection enforces these checks on the `main` branch.

### Developer Certificate of Origin (DCO)

This project requires all contributors to sign off on their commits, certifying that they have the right to submit the code under the project's license. This is enforced by the [DCO GitHub App](https://github.com/apps/dco).

To sign off, add `-s` to your commit command:

```bash
git commit -s -m "feat: add new feature"
```

This adds a `Signed-off-by` line to your commit message:

```text
Signed-off-by: Your Name <your.email@example.com>
```

By signing off, you agree to the [Developer Certificate of Origin](https://developercertificate.org/).

## Style Guidelines

### Rust Style

This project uses `rustfmt` with the project's configuration. Run `cargo fmt` before committing.

Key conventions:

- Rust 2024 Edition features and idioms
- Prefer `&str` over `String` when ownership isn't needed
- Use `?` operator for error propagation, avoid `unwrap()` in production code
- Use `thiserror` for public error types, `anyhow` for internal error handling
- See [AGENTS.md](AGENTS.md) for comprehensive Rust style guidelines

### Naming Conventions

| Item      | Convention           | Example                                |
| --------- | -------------------- | -------------------------------------- |
| Types     | PascalCase           | `CompressedFile`, `RuleyError`         |
| Functions | snake_case           | `create_llm_client`, `estimate_tokens` |
| Constants | SCREAMING_SNAKE_CASE | `MAX_FILES_PER_LANGUAGE`               |
| Modules   | snake_case           | `packer`, `generator`                  |

### Error Handling

- Use `Result<T, E>` for fallible operations
- Create specific error types with `thiserror` for public APIs
- Use `anyhow` with `.context()` for internal error handling
- Provide context in error messages
- Avoid `unwrap()` and `expect()` in production code

### Unicode and Emoji Policy

**Extended Unicode emoji characters are prohibited in source code, documentation, and comments.** This means no literal emoji codepoints (e.g., the actual characters at `U+2705`, `U+1F680`, `U+26A0`).

This prohibition **does not** apply to:

- **Gitmoji** shortcodes in commit messages (e.g., `:bug:`, `:sparkles:`, `:memo:`) -- these are ASCII strings rendered by Git hosting platforms
- **GitHub Flavored Markdown (GFM) emoji syntax** (e.g., `:warning:`, `:white_check_mark:`) -- these are ASCII shortcodes rendered by the Markdown engine
- **Code that processes or handles emoji** (e.g., Unicode normalization, text sanitization, display rendering) -- the prohibition is on decorative use, not functional use

The rationale: literal Unicode emoji render inconsistently across terminals, editors, and platforms. ASCII shortcodes are portable and degrade gracefully to readable text.

## Project Governance

### Decision-Making

ruley uses a **maintainer-driven** governance model. Decisions are made by the project maintainers through consensus on GitHub issues and pull requests.

### Roles

| Role            | Responsibilities                                                           | Current                                                                                        |
| --------------- | -------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| **Maintainer**  | Merge PRs, manage releases, set project direction, review security reports | [@unclesp1d3r](https://github.com/unclesp1d3r), [@KryptoKat08](https://github.com/KryptoKat08) |
| **Contributor** | Submit issues, PRs, and participate in discussions                         | Anyone following this guide                                                                    |

### How Decisions Are Made

- **Bug fixes and minor changes**: Any maintainer can review and merge
- **New features**: Discussed in a GitHub issue before implementation; maintainer approval required
- **Architecture changes**: Require agreement from both maintainers
- **Breaking changes**: Discussed in a GitHub issue with community input; require agreement from both maintainers
- **Releases**: Prepared by any maintainer, following the [release process](RELEASING.md)

### Becoming a Maintainer

As the project grows, active contributors who demonstrate sustained, high-quality contributions and alignment with project goals may be invited to become maintainers.

## Getting Help

- **Issues**: For bug reports and feature requests
- **Discussions**: For questions and ideas
- **Documentation**: Check [docs/](docs/) and [AGENTS.md](AGENTS.md) for detailed guides

---

Thank you for contributing to ruley!
