# ruley - Make your codebase ruley
# Task runner for common development operations

# Cross-platform shell configuration
# Use bash with strict error handling on Unix-like systems
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]
# Use PowerShell on Windows
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := true
set export := true

# Use mise to manage dev tools (cargo, pre-commit, git-cliff, etc.)
# See mise.toml for tool versions
mise_exec := "mise exec --"

# Default recipe - list available commands
default:
    @just --list

# ==============================================================================
# Formatting
# ==============================================================================

# Format all code
fmt:
    {{ mise_exec }} cargo fmt

# Check formatting without modifying files
fmt-check:
    {{ mise_exec }} cargo fmt -- --check

# ==============================================================================
# Linting
# ==============================================================================

check: pre-commit lint build-check

# Run clippy with zero warnings policy
clippy:
    {{ mise_exec }} cargo clippy -- -D warnings

# Run full lint suite (format check + clippy)
lint: fmt-check clippy

pre-commit:
    {{ mise_exec }} pre-commit run --all-files

# ==============================================================================
# Building
# ==============================================================================

# Check project without building
build-check:
    {{ mise_exec }} cargo check

# Build the project (debug)
build:
    {{ mise_exec }} cargo build

# Build optimized release binary
build-release:
    {{ mise_exec }} cargo build --release

# ==============================================================================
# Testing
# ==============================================================================

# Run all tests
test:
    {{ mise_exec }} cargo test

# Run tests with output
test-verbose:
    {{ mise_exec }} cargo test -- --nocapture

# Run benchmarks
bench:
    {{ mise_exec }} cargo bench

# Run specific benchmark
bench-name name:
    {{ mise_exec }} cargo bench -- {{ name }}

# ==============================================================================
# Running
# ==============================================================================

# Run the CLI with optional arguments
run args='':
    {{ mise_exec }} cargo run -- {{ args }}

# Run the release binary with optional arguments
run-release args='':
    {{ mise_exec }} cargo run --release -- {{ args }}

# ==============================================================================
# CI
# ==============================================================================

# Run full pre-commit CI check (lint + test + build)
ci-check: check test build

# Run Github Actions CI check
github-ci-check: lint build test

# Generate changelog from git history
changelog:
    {{ mise_exec }} git cliff -o CHANGELOG.md

# ==============================================================================
# Documentation
# ==============================================================================

# Generate documentation
doc:
    {{ mise_exec }} cargo doc --no-deps --open --document-private-items

# Generate documentation (without opening browser)
doc-build:
    {{ mise_exec }} cargo doc --no-deps --document-private-items

# ==============================================================================
# Dependencies
# ==============================================================================

# Update dependencies
update:
    {{ mise_exec }} cargo update

# Check for outdated dependencies
outdated:
    {{ mise_exec }} cargo outdated

# ==============================================================================
# Development
# ==============================================================================

# Setup development environment
dev-setup: fmt
    {{ mise_exec }} cargo build

# Full development check (format, lint, test, build)
dev-check: lint test build

# ==============================================================================
# Cleaning
# ==============================================================================

# Clean build artifacts
clean:
    {{ mise_exec }} cargo clean

# Deep clean with verbose output
clean-all:
    {{ mise_exec }} cargo clean --verbose
