# Installation

## Pre-built Binaries (Recommended)

Pre-built binaries are available for Linux (x86_64, ARM64), macOS (ARM64), and Windows (x86_64) on the [releases page](https://github.com/EvilBit-Labs/ruley/releases).

### macOS / Linux

```bash
curl -fsSL https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley-installer.sh | sh
```

### Windows

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley-installer.ps1 | iex"
```

## Homebrew

```bash
brew install EvilBit-Labs/tap/ruley
```

## Cargo (crates.io)

```bash
cargo install ruley
```

This builds from source with default features (Anthropic, OpenAI, TypeScript compression).

### With All Features

```bash
cargo install ruley --all-features
```

### Minimal Install

```bash
cargo install ruley --no-default-features --features anthropic
```

## cargo-binstall

If you have [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) installed:

```bash
cargo binstall ruley
```

## Building from Source

```bash
git clone https://github.com/EvilBit-Labs/ruley.git
cd ruley
cargo build --release
```

The binary will be at `./target/release/ruley`.

## System Requirements

- **Operating system**: Linux (x86_64, ARM64), macOS (ARM64), Windows (x86_64)
- **Rust** (build from source only): 1.91 or newer (see `rust-version` in `Cargo.toml`)
- **Network**: Required for LLM API calls (except Ollama which runs locally)

## Feature Flags

ruley uses Cargo feature flags to control which LLM providers and compression languages are compiled in:

| Feature                  | Description                     | Default |
| ------------------------ | ------------------------------- | ------- |
| `anthropic`              | Anthropic Claude provider       | Yes     |
| `openai`                 | OpenAI GPT provider             | Yes     |
| `ollama`                 | Ollama local model provider     | No      |
| `openrouter`             | OpenRouter multi-model provider | No      |
| `all-providers`          | All LLM providers               | No      |
| `compression-typescript` | TypeScript tree-sitter grammar  | Yes     |
| `compression-python`     | Python tree-sitter grammar      | No      |
| `compression-rust`       | Rust tree-sitter grammar        | No      |
| `compression-go`         | Go tree-sitter grammar          | No      |
| `compression-all`        | All compression languages       | No      |

## Verifying Releases

All release artifacts are signed via [Sigstore](https://www.sigstore.dev/) using GitHub Attestations:

```bash
gh attestation verify <artifact> --repo EvilBit-Labs/ruley
```

See [Release Verification](./release-verification.md) for details.
