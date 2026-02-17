# ruley - Make your codebase ruley
# Cross-platform justfile using OS annotations

set windows-shell := ["powershell.exe", "-c"]
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]
set dotenv-load := true
set ignore-comments := true

# Use mise to manage all dev tools (cargo, pre-commit, git-cliff, etc.)
# See mise.toml for tool versions
mise_exec := "mise exec --"

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

# Quick development check
check: pre-commit lint build-check

# Run clippy with zero warnings policy
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Run clippy with no default features
clippy-min:
    cargo clippy --all-targets --no-default-features -- -D warnings

# Run full lint suite (format check + clippy with all features)
lint-rust: fmt-check clippy

# Run full lint suite (alias for lint-rust)
lint: lint-rust

# Run pre-commit hooks
pre-commit:
    pre-commit run --all-files

# Run clippy with fixes
fix:
    cargo clippy --fix --allow-dirty --allow-staged

# ==============================================================================
# Building
# ==============================================================================

# Check project without building
build-check:
    @{{ mise_exec }} cargo check

build:
    @{{ mise_exec }} cargo build

build-release:
    cargo build --release --all-features

# Run all tests using nextest
test:
    cargo nextest run --all-features

# Run tests with standard cargo test (fallback)
test-cargo:
    cargo test --all-features

# Run tests with output
test-verbose:
    cargo nextest run --all-features --no-capture

# Run benchmarks
bench:
    @{{ mise_exec }} cargo bench

# Run specific benchmark
bench-name name:
    @{{ mise_exec }} cargo bench -- {{ name }}

# ==============================================================================
# Coverage
# ==============================================================================

# Generate coverage report
coverage:
    cargo llvm-cov --all-features --no-report
    cargo llvm-cov report --lcov --output-path lcov.info

# Generate HTML coverage report for local viewing
[unix]
coverage-report:
    cargo llvm-cov --all-features --html --open

# Show coverage summary
coverage-summary:
    cargo llvm-cov --all-features

# ==============================================================================
# Security & Auditing
# ==============================================================================

# Run dependency audit
audit:
    cargo audit

# Run cargo deny checks
deny:
    cargo deny check

# Check for outdated dependencies
outdated:
    cargo outdated --depth=1

# ==============================================================================
# Distribution
# ==============================================================================

# Run dist plan (dry run)
dist-plan:
    cargo dist plan

# Run dist check
dist-check:
    cargo dist check

# Build dist artifacts
dist:
    cargo dist build

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

# Run full pre-commit CI check (lint + test + build)
ci-check: pre-commit lint-rust clippy-min test build-release audit coverage dist-plan

# Run Github Actions CI check (lighter)
github-ci-check: lint-rust build test

# =============================================================================
# DOCUMENTATION
# =============================================================================

# Generate rustdoc documentation
doc:
    @{{ mise_exec }} cargo doc --no-deps --open --document-private-items

# Generate rustdoc documentation (without opening browser)
doc-build:
    @{{ mise_exec }} cargo doc --no-deps --document-private-items

# Build mdBook documentation
[unix]
docs-build:
    cd docs && mdbook build

# Serve documentation locally with live reload
[unix]
docs-serve:
    cd docs && mdbook serve --open

# ==============================================================================
# Development
# ==============================================================================

# Setup development environment
dev-setup:
    mise install
    cargo build

# Generate changelog from git history
changelog:
    @{{ mise_exec }} git cliff -o CHANGELOG.md

# ==============================================================================
# Dependencies
# ==============================================================================

# Update dependencies
update:
    @{{ mise_exec }} cargo update

# ==============================================================================
# Cleaning
# ==============================================================================

# Clean build artifacts
clean:
    @{{ mise_exec }} cargo clean

# Deep clean with verbose output
clean-all:
    @{{ mise_exec }} cargo clean --verbose
