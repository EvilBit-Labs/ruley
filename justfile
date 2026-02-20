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
    @{{ mise_exec }} cargo clippy --all-targets --all-features -- -D warnings

# Run clippy with no default features
clippy-min:
    @{{ mise_exec }} cargo clippy --all-targets --no-default-features -- -D warnings

# Run full lint suite (format check + clippy with all features)
lint-rust: fmt-check clippy

# Run full lint suite (alias for lint-rust)
lint: lint-rust

# Run pre-commit hooks
pre-commit:
    @{{ mise_exec }} pre-commit run --all-files

# Run clippy with fixes
fix:
    @{{ mise_exec }} cargo clippy --fix --allow-dirty --allow-staged

# ==============================================================================
# Building
# ==============================================================================

# Check project without building
build-check:
    @{{ mise_exec }} cargo check

build:
    @{{ mise_exec }} cargo build

build-release:
    @{{ mise_exec }} cargo build --release --all-features

# Run all tests using nextest
test:
    @{{ mise_exec }} cargo nextest run --all-features

# Run tests with standard cargo test (fallback)
test-cargo:
    @{{ mise_exec }} cargo test --all-features

# Run tests with output
test-verbose:
    @{{ mise_exec }} cargo nextest run --all-features --no-capture

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
    @{{ mise_exec }} cargo llvm-cov --all-features --no-report
    @{{ mise_exec }} cargo llvm-cov report --lcov --output-path lcov.info

# Generate HTML coverage report for local viewing
[unix]
coverage-report:
    @{{ mise_exec }} cargo llvm-cov --all-features --html --open

# Show coverage summary
coverage-summary:
    @{{ mise_exec }} cargo llvm-cov --all-features

# ==============================================================================
# Security & Auditing
# ==============================================================================

# Run dependency audit
audit:
    @{{ mise_exec }} cargo audit

# Run cargo deny checks
deny:
    @{{ mise_exec }} cargo deny check

# Check for outdated dependencies (fails in CI if any found)
outdated:
    @{{ mise_exec }} cargo outdated --depth=1 --exit-code=1

# ==============================================================================
# Distribution
# ==============================================================================

# Run dist plan (dry run)
dist-plan:
    @{{ mise_exec }} cargo dist plan

# Run dist check
dist-check:
    @{{ mise_exec }} cargo dist check

# Build dist artifacts
dist:
    @{{ mise_exec }} cargo dist build

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
    cd docs && {{ mise_exec }} mdbook build

# Serve documentation locally with live reload
[unix]
docs-serve:
    cd docs && {{ mise_exec }} mdbook serve --open

# ==============================================================================
# CI Simulation (act dry-runs)
# ==============================================================================

# Dry-run all GitHub Actions workflows locally with act
ci-dry-run: ci-dry-run-ci ci-dry-run-docs

# Dry-run CI workflow (all jobs)
ci-dry-run-ci:
    @echo "=== CI workflow ==="
    @act --workflows .github/workflows/ci.yml --container-architecture linux/amd64 -n 2>&1 | grep -E '(Job|⭐|✅|❌|🏁)' || true

# Dry-run docs workflow
ci-dry-run-docs:
    @echo "=== Docs workflow ==="
    @act -j build --workflows .github/workflows/docs.yml --container-architecture linux/amd64 -n 2>&1 | grep -E '(Job|⭐|✅|❌|🏁)' || true

# Dry-run a specific workflow (e.g., just ci-dry-run-workflow ci.yml)
ci-dry-run-workflow workflow:
    @act --workflows .github/workflows/{{ workflow }} --container-architecture linux/amd64 -n

# Dry-run a specific job (e.g., just ci-dry-run-job ci.yml quality)
ci-dry-run-job workflow job:
    @act -j {{ job }} --workflows .github/workflows/{{ workflow }} --container-architecture linux/amd64 -n

# ==============================================================================
# Development
# ==============================================================================

# Setup development environment
dev-setup:
    mise install
    @{{ mise_exec }} cargo build

# Generate changelog from git history
changelog:
    @{{ mise_exec }} git cliff -o CHANGELOG.md

# ==============================================================================
# Dependencies
# ==============================================================================

# Update dependencies
update:
    @{{ mise_exec }} cargo update

# =============================================================================
# CLEANING
# =============================================================================

# Clean build artifacts
clean:
    @{{ mise_exec }} cargo clean

# Deep clean with verbose output
clean-all:
    @{{ mise_exec }} cargo clean --verbose
