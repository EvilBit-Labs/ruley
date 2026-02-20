# Introduction

> Make your codebase ruley. A Rust CLI tool for generating AI IDE rules from codebases.

**ruley** (the opposite of *unruly*) is a command-line tool that analyzes codebases and generates AI IDE rule files. It uses Large Language Models to understand project structure, conventions, and patterns, then produces actionable rules that help AI assistants provide better, context-aware code suggestions.

Tame your unruly codebase. Make it *ruley*.

## Why ruley?

AI coding assistants work best when they understand your project's conventions. Without explicit rules, they fall back to generic patterns that may not match your codebase. ruley bridges this gap by:

1. **Scanning** your repository to understand its structure, languages, and patterns
2. **Compressing** the codebase using tree-sitter for token efficiency
3. **Analyzing** the compressed code with an LLM to extract conventions
4. **Generating** format-specific rule files for your preferred AI IDE tools

The result is a set of rule files that teach AI assistants how your project works -- coding style, architecture patterns, naming conventions, error handling approaches, and more.

## Key Features

- **Single binary distribution** -- No runtime dependencies (Node.js, Python, etc.)
- **Multi-provider LLM support** -- Anthropic, OpenAI, Ollama, OpenRouter
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

## Where to Start

- **New users**: Start with [Installation](./installation.md) and [Quick Start](./quickstart.md)
- **CLI reference**: See [Command-Line Interface](./cli.md) for all options
- **Configuration**: See [Configuration](./configuration.md) for `ruley.toml` setup
- **Contributors**: See [Development Setup](./development.md) to get started
- **Architecture**: See [Architecture Overview](./architecture.md) to understand the internals
