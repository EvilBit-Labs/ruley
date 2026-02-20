# ruley

[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/EvilBit-Labs/ruley/badge)](https://scorecard.dev/viewer/?uri=github.com/EvilBit-Labs/ruley) [![Crates.io](https://img.shields.io/crates/v/ruley)](https://crates.io/crates/ruley) [![License](https://img.shields.io/crates/l/ruley)](https://github.com/EvilBit-Labs/ruley/blob/main/LICENSE) [![CI](https://github.com/EvilBit-Labs/ruley/actions/workflows/ci.yml/badge.svg)](https://github.com/EvilBit-Labs/ruley/actions/workflows/ci.yml) [![codecov](https://codecov.io/gh/EvilBit-Labs/ruley/graph/badge.svg)](https://codecov.io/gh/EvilBit-Labs/ruley)

> Make your codebase ruley. A Rust CLI tool for generating AI IDE rules from codebases.

**ruley** (the opposite of _unruly_) is a command-line tool that analyzes codebases and generates AI IDE rule files. It uses Large Language Models to understand project structure, conventions, and patterns, then produces actionable rules that help AI assistants provide better, context-aware code suggestions.

Tame your unruly codebase. Make it _ruley_.

## Project Status

**v1.0.0** -- Production ready. Published on [crates.io](https://crates.io/crates/ruley).

## Features

- **Single binary distribution** -- No runtime dependencies (Node.js, Python, etc.)
- **Multi-provider LLM support** -- Choose your preferred AI backend
- **Multi-format output** -- Generate rules for 7 different AI IDE formats in a single run
- **Native performance** -- Fast codebase analysis built with Rust
- **Smart compression** -- Tree-sitter-based code compression for token efficiency (~70% reduction)
- **Accurate token counting** -- Native tiktoken implementation for precise cost estimation
- **Cost transparency** -- Shows estimated cost before LLM calls, requires confirmation
- **Configurable** -- TOML configuration file, environment variables, and CLI flags

## Supported Formats

| Format       | Output File                       | Description                      |
| ------------ | --------------------------------- | -------------------------------- |
| **Cursor**   | `.cursor/rules/*.mdc`             | Cursor IDE rules                 |
| **Claude**   | `CLAUDE.md`                       | Claude Code project instructions |
| **Copilot**  | `.github/copilot-instructions.md` | GitHub Copilot instructions      |
| **Windsurf** | `.windsurfrules`                  | Windsurf IDE rules               |
| **Aider**    | `.aider.conf.yml`                 | Aider conventions                |
| **Generic**  | `.ai-rules.md`                    | Generic markdown rules           |
| **JSON**     | `.ai-rules.json`                  | Machine-readable JSON            |

## Supported Providers

| Provider       | Environment Variable   |
| -------------- | ---------------------- |
| **Anthropic**  | `ANTHROPIC_API_KEY`    |
| **OpenAI**     | `OPENAI_API_KEY`       |
| **Ollama**     | (local, no key needed) |
| **OpenRouter** | `OPENROUTER_API_KEY`   |

## Installation

### Cargo (crates.io)

```bash
cargo install ruley
```

### Homebrew

```bash
brew install EvilBit-Labs/tap/ruley
```

### Binary Download

Pre-built binaries are available for Linux (x86_64, ARM64), macOS (ARM64), and Windows (x86_64) on the [releases page](https://github.com/EvilBit-Labs/ruley/releases).

```bash
# macOS / Linux
curl -fsSL https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley-installer.sh | sh
```

```powershell
# Windows
powershell -ExecutionPolicy Bypass -c "irm https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley-installer.ps1 | iex"
```

### Verifying Releases

All release artifacts are signed via [Sigstore](https://www.sigstore.dev/) using GitHub Attestations:

```bash
gh attestation verify <artifact> --repo EvilBit-Labs/ruley
```

## Quick Start

```bash
# Analyze current directory with defaults (Anthropic Claude)
ruley

# Use OpenAI GPT-4o
ruley --provider openai --model gpt-4o

# Analyze a specific directory with compression
ruley ./my-project --compress

# Generate multiple formats at once
ruley --format cursor,claude,copilot

# Generate all supported formats
ruley --format all

# Dry run (show what would be processed without calling the LLM)
ruley --dry-run

# Skip cost confirmation prompt
ruley --no-confirm

# Use a local Ollama model
ruley --provider ollama --model llama3.1
```

## Configuration

ruley supports hierarchical configuration with the following precedence (highest to lowest):

1. **Command-line flags**
2. **Environment variables**
3. **Configuration file** (`ruley.toml`)
4. **Built-in defaults**

### Example `ruley.toml`

```toml
[general]
provider = "anthropic"
model = "claude-sonnet-4-5-20250929"
format = ["cursor", "claude"]
compress = true
no_confirm = false
```

### Environment Variables

Set your API key for your chosen provider:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
```

## How It Works

ruley scans your codebase, compresses it for token efficiency, sends it to an LLM for analysis, and writes format-specific rule files. Your API keys are never stored -- they're read from environment variables and used only for the duration of the run.

For vulnerability reporting, see [SECURITY.md](SECURITY.md). For architecture details, see [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding guidelines, and submission process.

## License

Licensed under the Apache License 2.0 -- see [LICENSE](LICENSE) for details.

## Support

- [Documentation](https://github.com/EvilBit-Labs/ruley/wiki)
- [GitHub Issues](https://github.com/EvilBit-Labs/ruley/issues)
- [GitHub Discussions](https://github.com/EvilBit-Labs/ruley/discussions)

## Acknowledgments

- The Rust community for excellent tooling and ecosystem
- [tree-sitter](https://tree-sitter.github.io/) for code parsing and compression
- [tiktoken](https://github.com/openai/tiktoken) for accurate token counting
