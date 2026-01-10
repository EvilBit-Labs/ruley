# ruley - Make your codebase ruley
# Task runner for common development operations

# Cross-platform shell configuration
# Use bash with strict error handling on Unix-like systems
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]
# Use PowerShell on Windows
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := true
set export := true

# Default recipe - list available commands
default:
    @just --list

# ==============================================================================
# Formatting
# ==============================================================================

# Format all code
fmt:
    cargo fmt

# Check formatting without modifying files
fmt-check:
    cargo fmt -- --check

# ==============================================================================
# Linting
# ==============================================================================

check: pre-commit lint build-check

# Run clippy with zero warnings policy
clippy:
    cargo clippy -- -D warnings

# Run full lint suite (format check + clippy)
lint: fmt-check clippy

pre-commit:
    pre-commit run --all-files

# ==============================================================================
# Building
# ==============================================================================

# Check project without building
build-check:
    cargo check

# Build the project (debug)
build:
    cargo build

# Build optimized release binary
build-release:
    cargo build --release

# ==============================================================================
# Testing
# ==============================================================================

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run benchmarks
bench:
    cargo bench

# Run specific benchmark
bench-name name:
    cargo bench -- {{ name }}

# ==============================================================================
# Running
# ==============================================================================

# Run the CLI with optional arguments
run args='':
    cargo run -- {{ args }}

# Run the release binary with optional arguments
run-release args='':
    cargo run --release -- {{ args }}

# ==============================================================================
# CI
# ==============================================================================

# Run full pre-commit CI check (lint + test + build)
ci-check: check test build

# Run Github Actions CI check
github-ci-check: lint build test

# Generate changelog from git history
changelog:
    git cliff -o CHANGELOG.md

# ==============================================================================
# Documentation
# ==============================================================================

# Generate documentation
doc:
    cargo doc --no-deps --open --document-private-items

# Generate documentation (without opening browser)
doc-build:
    cargo doc --no-deps --document-private-items

# ==============================================================================
# Dependencies
# ==============================================================================

# Update dependencies
update:
    cargo update

# Check for outdated dependencies
outdated:
    cargo outdated

# ==============================================================================
# Development
# ==============================================================================

# Setup development environment
dev-setup: fmt
    cargo build

# Full development check (format, lint, test, build)
dev-check: lint test build

# ==============================================================================
# Cleaning
# ==============================================================================

# Clean build artifacts
clean:
    cargo clean

# Deep clean with verbose output
clean-all:
    cargo clean --verbose
