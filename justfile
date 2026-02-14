# ruley - Make your codebase ruley
# Cross-platform justfile using OS annotations

set windows-shell := ["powershell.exe", "-c"]
set shell := ["bash", "-c"]
set dotenv-load := true
set ignore-comments := true

# Use mise to manage all dev tools (cargo, pre-commit, git-cliff, etc.)
# See mise.toml for tool versions
mise_exec := "mise exec --"

root := justfile_dir()

# =============================================================================
# GENERAL COMMANDS
# =============================================================================

default:
    @just --list

# Development setup - mise handles all tool installation via mise.toml
setup:
    mise install

# =============================================================================
# FORMATTING AND LINTING
# =============================================================================

alias format := fmt

# Format all code
fmt:
    @{{ mise_exec }} cargo fmt

# Check formatting without modifying files
fmt-check:
    @{{ mise_exec }} cargo fmt -- --check

# Run clippy with zero warnings (default features)
lint-rust: fmt-check
    @{{ mise_exec }} cargo clippy --all-targets --all-features -- -D warnings

# Run clippy with fixes
fix:
    @{{ mise_exec }} cargo clippy --fix --allow-dirty --allow-staged

# Main lint recipe - calls all sub-linters
lint: lint-rust lint-actions

# Lint GitHub Actions workflow files
lint-actions:
    @{{ mise_exec }} actionlint

# Quick development check
check: pre-commit-run lint

[private]
pre-commit-run:
    @{{ mise_exec }} pre-commit run -a

# =============================================================================
# BUILDING AND TESTING
# =============================================================================

build:
    @{{ mise_exec }} cargo build

build-release:
    @{{ mise_exec }} cargo build --release

# Check project without building
build-check:
    @{{ mise_exec }} cargo check

test:
    @{{ mise_exec }} cargo test

# Run tests with output
test-verbose:
    @{{ mise_exec }} cargo test -- --nocapture

# Run benchmarks
bench:
    @{{ mise_exec }} cargo bench

# Run specific benchmark
bench-name name:
    @{{ mise_exec }} cargo bench -- {{ name }}

# =============================================================================
# RUNNING
# =============================================================================

# Run the CLI with optional arguments
run args='':
    @{{ mise_exec }} cargo run -- {{ args }}

# Run the release binary with optional arguments
run-release args='':
    @{{ mise_exec }} cargo run --release -- {{ args }}

# =============================================================================
# SECURITY AND AUDITING
# =============================================================================

audit:
    @{{ mise_exec }} cargo audit

# =============================================================================
# CI AND QUALITY ASSURANCE
# =============================================================================

# Full local CI parity check
ci-check: pre-commit-run fmt-check lint-rust test build-release audit

# Run GitHub Actions CI check (no pre-commit or audit)
github-ci-check: lint build test

# =============================================================================
# DOCUMENTATION
# =============================================================================

# Generate documentation and open in browser
doc:
    @{{ mise_exec }} cargo doc --no-deps --open --document-private-items

# Generate documentation (without opening browser)
doc-build:
    @{{ mise_exec }} cargo doc --no-deps --document-private-items

# =============================================================================
# DEPENDENCIES
# =============================================================================

# Update dependencies
update:
    @{{ mise_exec }} cargo update

# Check for outdated dependencies
outdated:
    @{{ mise_exec }} cargo outdated

# =============================================================================
# RELEASE MANAGEMENT
# =============================================================================

# Generate changelog from git history
changelog:
    @{{ mise_exec }} git cliff -o CHANGELOG.md

# =============================================================================
# CLEANING
# =============================================================================

# Clean build artifacts
clean:
    @{{ mise_exec }} cargo clean

# Deep clean with verbose output
clean-all:
    @{{ mise_exec }} cargo clean --verbose
