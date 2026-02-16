# ruley

> Make your codebase ruley. A Rust CLI tool for generating AI IDE rules from codebases.

**ruley** (the opposite of _unruly_) is a command-line tool that analyzes codebases and generates AI IDE rule files. It uses Large Language Models to understand project structure, conventions, and patterns, then produces actionable rules that help AI assistants provide better, context-aware code suggestions.

Tame your unruly codebase. Make it _ruley_.

## Features

- **Single binary distribution** - No runtime dependencies (Node.js, Python, etc.)
- **Multi-provider LLM support** - Choose your preferred AI backend
- **Native performance** - Fast codebase analysis using Rust
- **Smart compression** - Tree-sitter based code compression for token efficiency
- **Accurate token counting** - Native tiktoken implementation

## Quick Start

```bash
# Analyze current directory with defaults (Anthropic Claude)
ruley

# Use OpenAI GPT-4o
ruley --provider openai --model gpt-4o

# Analyze specific directory with compression
ruley ./my-project --compress

# Generate multiple formats
ruley --format cursor,claude,copilot
```

## Installation

```bash
# Cargo (crates.io)
cargo install ruley

# Homebrew
brew install EvilBit-Labs/tap/ruley
```
