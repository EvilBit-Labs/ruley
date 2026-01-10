# ruley - Project Specification

> Make your codebase ruley. A Rust CLI tool for generating AI IDE rules from codebases.

**Version:** 0.1.0-spec **License:** Apache-2.0 **Status:** Specification Draft **Crate:** `ruley` **Repository:** `github.com/EvilBit-Labs/ruley`

---

## Table of Contents

01. [Project Overview](#project-overview)
02. [Acknowledgments](#acknowledgments)
03. [Goals and Non-Goals](#goals-and-non-goals)
04. [Feature Requirements](#feature-requirements)
05. [Architecture](#architecture)
06. [CLI Interface](#cli-interface)
07. [Configuration](#configuration)
08. [Technical Specifications](#technical-specifications)
09. [Dependencies](#dependencies)
10. [Implementation Plan](#implementation-plan)
11. [Testing Strategy](#testing-strategy)
12. [Distribution](#distribution)
13. [Future Considerations](#future-considerations)

---

## Project Overview

**ruley** (the opposite of _unruly_) is a command-line tool that analyzes codebases and generates AI IDE rule files. It uses Large Language Models to understand project structure, conventions, and patterns, then produces actionable rules that help AI assistants provide better, context-aware code suggestions.

Tame your unruly codebase. Make it _ruley_.

### Key Value Proposition

- **Single binary distribution** - No runtime dependencies (Node.js, Python, etc.)
- **Multi-provider LLM support** - Choose your preferred AI backend
- **Native performance** - Fast codebase analysis using Rust
- **Smart compression** - Tree-sitter based code compression for token efficiency
- **Accurate token counting** - Native tiktoken implementation

---

## Acknowledgments

This project is inspired by [rulefy](https://github.com/niklub/rulefy) by niklub, licensed under MIT. While ruley is a cleanroom implementation with independent architecture decisions, we acknowledge rulefy for pioneering the concept of automated AI IDE rule generation.

Other inspirations:

- [repomix](https://github.com/yamadashy/repomix) - Repository packing concepts
- [awesome-cursorrules](https://github.com/PatrickJS/awesome-cursorrules) - Cursor rules best practices

---

## Goals and Non-Goals

### Goals

1. **Standalone binary** - Zero runtime dependencies
2. **Multi-provider LLM support** - OpenAI, Anthropic, xAI, Ollama, OpenRouter, etc.
3. **Local-first** - Work with local repositories without network for packing
4. **Token-efficient** - Tree-sitter compression to reduce LLM costs
5. **Extensible output formats** - Start with Cursor `.mdc`, design for future formats
6. **Cross-platform** - Linux, macOS, Windows support
7. **Configurable** - TOML/YAML config files, environment variables, CLI flags
8. **Transparent pricing** - Show estimated costs before LLM calls

### Non-Goals

1. **GUI** - CLI only (TUI may be considered later)
2. **IDE plugins** - Out of scope for v1.0
3. **Real-time sync** - Generate once, not continuous
4. **Rule management** - Only generation, not organization/editing
5. **Custom LLM fine-tuning** - Use off-the-shelf models

---

## Feature Requirements

### Core Features (v1.0)

| ID  | Feature                 | Priority | Description                                     |
| --- | ----------------------- | -------- | ----------------------------------------------- |
| F1  | Local repo analysis     | P0       | Analyze local directory codebases               |
| F2  | Remote repo cloning     | P0       | Clone and analyze GitHub/GitLab repos           |
| F3  | Gitignore respect       | P0       | Honor `.gitignore` patterns                     |
| F4  | Include/exclude globs   | P0       | Filter files by glob patterns                   |
| F5  | Multi-provider LLM      | P0       | Support multiple LLM backends                   |
| F6  | Token counting          | P0       | Accurate token estimation before API calls      |
| F7  | Cost estimation         | P0       | Show estimated cost, require confirmation       |
| F8  | Chunked processing      | P0       | Handle large codebases exceeding context limits |
| F9  | Cursor .mdc output      | P0       | Generate valid Cursor rule files                |
| F10 | Progress display        | P1       | Show processing progress                        |
| F11 | Tree-sitter compression | P1       | Reduce tokens by extracting signatures          |
| F12 | Config file support     | P1       | TOML configuration files                        |
| F13 | Custom prompts          | P2       | User-defined prompt templates                   |
| F14 | Retry logic             | P1       | Handle transient API failures                   |
| F15 | Streaming output        | P2       | Stream LLM responses for UX                     |

### Output Formats (v1.0)

A core differentiator from rulefy: **multi-format output** from day one.

| Format         | File                              | Description                        |
| -------------- | --------------------------------- | ---------------------------------- |
| Cursor Rules   | `.cursor/rules/*.mdc`             | Cursor IDE AI rules                |
| Claude Code    | `CLAUDE.md`                       | Claude Code project instructions   |
| GitHub Copilot | `.github/copilot-instructions.md` | GitHub Copilot custom instructions |
| Windsurf       | `.windsurfrules`                  | Windsurf IDE rules                 |
| Aider          | `.aider.conf.yml` + conventions   | Aider AI pair programming          |
| Generic        | `AI_CONTEXT.md`                   | Universal AI assistant context     |
| JSON           | `ai-rules.json`                   | Machine-readable for tooling       |

All formats share the same core analysis - only the output structure differs.

### Output Format Specifications

#### Cursor Rules (.mdc)

```markdown
---
description: When to apply this rule
globs: src/**/*.ts
alwaysApply: false
---

# Rule Title

## Critical Rules

- Actionable directive 1
- Actionable directive 2

## Examples

<example>
Valid usage example
</example>

<example type="invalid">
Invalid usage example
</example>
```

**Characteristics:**

- YAML frontmatter with `description`, `globs`, `alwaysApply`
- Rule types: `auto`, `manual`, `agent`, `always`
- Supports `mdc:` file references
- Located in `.cursor/rules/`

#### Claude Code (CLAUDE.md)

```markdown
# Project: {name}

## Overview

Brief project description and purpose.

## Tech Stack

- Language: TypeScript
- Framework: React
- Testing: Jest

## Architecture

Description of project structure and patterns.

## Conventions

### Naming

- Components: PascalCase
- Functions: camelCase

### Code Style

- Prefer functional components
- Use TypeScript strict mode

## Common Tasks

### Adding a new feature

1. Step one
2. Step two

## Files to Know

- `src/index.ts` - Entry point
- `src/config.ts` - Configuration
```

**Characteristics:**

- Single markdown file at repo root
- Structured sections (Overview, Tech Stack, Architecture, etc.)
- Task-oriented guidance
- No frontmatter required

#### GitHub Copilot Instructions

```markdown
# GitHub Copilot Instructions

## Project Context

This is a TypeScript monorepo using pnpm workspaces.

## Code Style

- Use functional React components
- Prefer named exports
- Use TypeScript strict mode

## Testing

- Write tests using Jest and React Testing Library
- Place tests in `__tests__` directories

## Patterns to Follow

- Use React Query for data fetching
- Use Zustand for state management

## Patterns to Avoid

- Don't use class components
- Avoid default exports
```

**Characteristics:**

- Located at `.github/copilot-instructions.md`
- Plain markdown, no frontmatter
- Focus on dos and don'ts
- Scoped to repository

#### Windsurf Rules

```markdown
# Windsurf Rules

## Project Type

TypeScript React Application

## Key Conventions

- Functional components only
- TailwindCSS for styling
- React Query for server state

## File Structure

src/ components/ hooks/ utils/ pages/

## Coding Standards

- ESLint + Prettier enforced
- No any types
- Comprehensive error handling
```

**Characteristics:**

- Single `.windsurfrules` file at repo root
- Similar structure to Cursor but different location
- Plain markdown format

#### Aider Configuration

```yaml
# .aider.conf.yml
model: claude-sonnet-4-5-20250929
edit-format: diff
auto-commits: false
conventions-file: CONVENTIONS.md
```

```markdown
# CONVENTIONS.md

## Code Style

...

## Architecture

...
```

**Characteristics:**

- YAML config file + conventions markdown
- Split configuration from content
- Aider-specific options in YAML

#### Generic AI Context

```markdown
# AI Assistant Context for {project}

> This document provides context for AI coding assistants.

## Project Summary

{Generated summary}

## Technology Stack

{Detected technologies}

## Project Structure

{Directory tree with descriptions}

## Coding Conventions

{Extracted patterns and conventions}

## Important Files

{Key files with descriptions}

## Common Patterns

{Code patterns found in the project}
```

**Characteristics:**

- Universal format for any AI tool
- Can be copy/pasted into chat interfaces
- Standalone, no tool-specific features

#### JSON Format

```json
{
  "version": "1.0",
  "project": {
    "name": "my-project",
    "type": "typescript-react",
    "description": "..."
  },
  "techStack": {
    "language": "TypeScript",
    "framework": "React",
    "buildTool": "Vite"
  },
  "conventions": {
    "naming": {...},
    "codeStyle": {...}
  },
  "patterns": [...],
  "files": {...}
}
```

**Characteristics:**

- Machine-readable for tooling integration
- Can be transformed to any other format
- Useful for CI/CD pipelines

### Multi-Format Generation

Generate multiple formats in one command:

```bash
# Generate all common formats
ruley --format cursor,claude,copilot

# Or use preset
ruley --format all
```

### Internal Representation

All formats share a common internal representation generated from LLM analysis:

```rust
/// Core rule representation - format-agnostic
pub struct GeneratedRules {
    /// Project metadata
    pub project: ProjectInfo,

    /// Detected technology stack
    pub tech_stack: TechStack,

    /// Code conventions and patterns
    pub conventions: Vec<Convention>,

    /// Important files with descriptions
    pub key_files: Vec<KeyFile>,

    /// Architecture patterns
    pub architecture: ArchitectureInfo,

    /// Common tasks and workflows
    pub tasks: Vec<Task>,

    /// Anti-patterns to avoid
    pub antipatterns: Vec<Antipattern>,

    /// Example code snippets
    pub examples: Vec<Example>,
}

pub struct Convention {
    pub category: String, // "naming", "code-style", "testing", etc.
    pub rule: String,     // The actual convention
    pub rationale: Option<String>,
    pub examples: Vec<Example>,
}

pub struct Example {
    pub description: String,
    pub code: String,
    pub is_valid: bool, // true = good example, false = anti-pattern
}
```

**Workflow:**

```text
┌─────────────┐     ┌──────────────┐     ┌─────────────────────┐
│  Codebase   │────▶│ LLM Analysis │────▶│ GeneratedRules      │
│  (packed)   │     │              │     │ (internal repr)     │
└─────────────┘     └──────────────┘     └─────────────────────┘
                                                   │
                    ┌──────────────────────────────┼──────────────────────────────┐
                    │              │               │               │              │
                    ▼              ▼               ▼               ▼              ▼
             ┌──────────┐  ┌──────────┐    ┌──────────┐    ┌──────────┐   ┌──────────┐
             │  Cursor  │  │  Claude  │    │  Copilot │    │ Windsurf │   │   JSON   │
             │  .mdc    │  │  .md     │    │  .md     │    │  rules   │   │          │
             └──────────┘  └──────────┘    └──────────┘    └──────────┘   └──────────┘
```

This design means:

1. **Single LLM call** - Analyze once, output many formats
2. **Consistent content** - All formats contain the same rules
3. **Easy to add formats** - Just implement `OutputFormatter` trait
4. **Testable** - Can test formatters independently from LLM

---

## Architecture

### High-Level Design

```text
┌─────────────────────────────────────────────────────────────────┐
│                           CLI Layer                             │
│                     (clap argument parsing)                     │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                        Orchestrator                             │
│              (coordinates pipeline execution)                   │
└─────────────────────────────────────────────────────────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
┌───────────────┐      ┌─────────────────┐      ┌─────────────────┐
│  Repo Packer  │      │  LLM Generator  │      │ Output Formatter│
│               │      │                 │      │                 │
│ • Git clone   │      │ • Multi-provider│      │ • Cursor .mdc   │
│ • File walker │      │ • Chunking      │      │ • Claude .md    │
│ • Gitignore   │      │ • Token count   │      │ • JSON          │
│ • Compression │      │ • Retry logic   │      │                 │
└───────────────┘      └─────────────────┘      └─────────────────┘
        │                        │
        ▼                        ▼
┌───────────────┐      ┌─────────────────┐
│  Tree-sitter  │      │   LLM Clients   │
│  (optional)   │      │                 │
│               │      │ • OpenAI        │
│ • TypeScript  │      │ • Anthropic     │
│ • Python      │      │ • xAI           │
│ • Rust        │      │ • Ollama        │
│ • Go          │      │ • OpenRouter    │
│ • etc.        │      │ • etc.          │
└───────────────┘      └─────────────────┘
```

### Module Structure

```text
ruley/                         # "Make your codebase ruley"
├── Cargo.toml
├── LICENSE                    # Apache-2.0
├── README.md
├── CHANGELOG.md
│
├── src/
│   ├── main.rs               # Entry point
│   ├── lib.rs                # Library root (for use as crate)
│   │
│   ├── cli/
│   │   ├── mod.rs
│   │   ├── args.rs           # Clap argument definitions
│   │   └── config.rs         # Config file parsing
│   │
│   ├── packer/
│   │   ├── mod.rs
│   │   ├── walker.rs         # File discovery
│   │   ├── gitignore.rs      # Ignore pattern handling
│   │   ├── git.rs            # Git operations (clone, etc.)
│   │   ├── compress.rs       # Tree-sitter compression
│   │   └── output.rs         # Pack format (XML, etc.)
│   │
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── provider.rs       # Provider trait
│   │   ├── client.rs         # Unified client
│   │   ├── chunker.rs        # Token-aware chunking
│   │   ├── tokenizer.rs      # Tiktoken wrapper
│   │   └── providers/
│   │       ├── mod.rs
│   │       ├── anthropic.rs
│   │       ├── openai.rs
│   │       ├── ollama.rs
│   │       └── openrouter.rs
│   │
│   ├── generator/
│   │   ├── mod.rs
│   │   ├── prompts.rs        # Prompt templates
│   │   └── rules.rs          # Rule generation logic
│   │
│   ├── output/
│   │   ├── mod.rs
│   │   ├── cursor.rs         # Cursor .mdc format
│   │   ├── claude.rs         # CLAUDE.md format
│   │   ├── copilot.rs        # GitHub Copilot instructions
│   │   ├── windsurf.rs       # Windsurf rules
│   │   ├── aider.rs          # Aider config + conventions
│   │   ├── generic.rs        # Universal AI context
│   │   └── json.rs           # JSON format
│   │
│   └── utils/
│       ├── mod.rs
│       ├── progress.rs       # Progress bars
│       └── error.rs          # Error types
│
├── prompts/
│   ├── base.md               # Shared analysis prompt
│   ├── cursor.md             # Cursor .mdc specific
│   ├── claude.md             # CLAUDE.md specific
│   ├── copilot.md            # GitHub Copilot specific
│   ├── windsurf.md           # Windsurf specific
│   ├── aider.md              # Aider specific
│   └── generic.md            # Universal format
│
└── tests/
    ├── integration/
    └── fixtures/
```

### Key Traits

```rust
/// Provider-agnostic LLM interface
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Generate a completion from messages
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse>;

    /// Stream a completion (optional)
    async fn complete_stream(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<impl Stream<Item = Result<StreamChunk>>>;

    /// Get the model name
    fn model(&self) -> &str;

    /// Get pricing info for cost estimation
    fn pricing(&self) -> Pricing;
}

/// Output format interface
pub trait OutputFormatter {
    /// Format generated rules into target format
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String>;

    /// File extension for this format
    fn extension(&self) -> &str;
}

/// Compression strategy interface
pub trait Compressor {
    /// Compress source code, returning signature-only version
    fn compress(&self, source: &str, language: Language) -> Result<String>;

    /// Estimated token reduction ratio
    fn compression_ratio(&self) -> f32;
}
```

---

## CLI Interface

### Command Structure

```text
ruley [OPTIONS] [PATH]

ARGUMENTS:
  [PATH]  Path to repository (local path or remote URL) [default: .]

OPTIONS:
  -p, --provider <PROVIDER>    LLM provider [default: anthropic]
                               [possible values: anthropic, openai, ollama,
                                openrouter, xai, groq, gemini]

  -m, --model <MODEL>          Model to use [default: provider-specific]

  -o, --output <PATH>          Output file path [default: <repo-name>.rules.mdc]

  -f, --format <FORMAT>        Output format(s), comma-separated [default: cursor]
                               [possible values: cursor, claude, copilot, windsurf,
                                aider, generic, json, all]

      --description <TEXT>     Focus area for rule generation

      --rule-type <TYPE>       Cursor rule type [default: agent]
                               [possible values: auto, manual, agent, always]

  -c, --config <PATH>          Config file path [default: ruley.toml]

      --include <GLOB>         Include only matching files (repeatable)

      --exclude <GLOB>         Exclude matching files (repeatable)

      --compress               Enable tree-sitter compression

      --chunk-size <TOKENS>    Max tokens per LLM chunk [default: 100000]

      --no-confirm             Skip cost confirmation prompt

      --dry-run                Show what would be processed without calling LLM

  -v, --verbose                Increase verbosity (-v, -vv, -vvv)

  -q, --quiet                  Suppress non-essential output

  -h, --help                   Print help

  -V, --version                Print version

ENVIRONMENT VARIABLES:
  ANTHROPIC_API_KEY     API key for Anthropic
  OPENAI_API_KEY        API key for OpenAI
  OPENROUTER_API_KEY    API key for OpenRouter
  XAI_API_KEY           API key for xAI
  GROQ_API_KEY          API key for Groq
  GEMINI_API_KEY        API key for Google Gemini
  OLLAMA_HOST           Ollama server URL [default: http://localhost:11434]
```

### Usage Examples

```bash
# Analyze current directory with defaults (Anthropic Claude)
ruley

# Use OpenAI GPT-4o
ruley --provider openai --model gpt-4o

# Analyze specific directory with compression
ruley ./my-project --compress

# Clone and analyze remote repository
ruley https://github.com/user/repo

# Focus on specific area
ruley --description "authentication and security patterns"

# Use OpenRouter for model flexibility
ruley --provider openrouter --model anthropic/claude-3.5-sonnet

# Include only TypeScript files
ruley --include "**/*.ts" --include "**/*.tsx"

# Dry run to see what would be processed
ruley --dry-run

# Generate Claude Code format
ruley --format claude --output CLAUDE.md

# Generate multiple formats at once
ruley --format cursor,claude,copilot

# Generate all supported formats
ruley --format all

# Use local Ollama model
ruley --provider ollama --model llama3.1:70b
```

---

## Configuration

### Config File (ruley.toml)

```toml
# ruley.toml - Project-level configuration

[general]
# Default LLM provider
provider = "anthropic"

# Default model (provider-specific)
model = "claude-sonnet-4-5-20250929"

# Output format
format = "cursor"

# Enable compression by default
compress = true

# Token chunk size
chunk_size = 100000

# Skip cost confirmation
no_confirm = false

[output]
# Default formats to generate
formats = ["cursor"]

# Format-specific output paths
[output.paths]
cursor = ".cursor/rules/{name}.rules.mdc"
claude = "CLAUDE.md"
copilot = ".github/copilot-instructions.md"
windsurf = ".windsurfrules"
aider = "CONVENTIONS.md"
generic = "AI_CONTEXT.md"
json = "ai-rules.json"

[include]
# Glob patterns to include
patterns = [
  "**/*.ts",
  "**/*.tsx",
  "**/*.js",
  "**/*.jsx",
  "**/*.py",
  "**/*.rs",
  "**/*.go",
]

[exclude]
# Glob patterns to exclude (in addition to .gitignore)
patterns = [
  "**/node_modules/**",
  "**/target/**",
  "**/dist/**",
  "**/.git/**",
  "**/vendor/**",
]

[providers.anthropic]
model = "claude-sonnet-4-5-20250929"
max_tokens = 8192

[providers.openai]
model = "gpt-4o"
max_tokens = 4096

[providers.ollama]
host = "http://localhost:11434"
model = "llama3.1:70b"

[providers.openrouter]
model = "anthropic/claude-3.5-sonnet"
fallback_models = ["openai/gpt-4o", "mistral/mistral-large"]
```

### Environment Variables

| Variable             | Description            | Required                        |
| -------------------- | ---------------------- | ------------------------------- |
| `ANTHROPIC_API_KEY`  | Anthropic API key      | If using Anthropic              |
| `OPENAI_API_KEY`     | OpenAI API key         | If using OpenAI                 |
| `OPENROUTER_API_KEY` | OpenRouter API key     | If using OpenRouter             |
| `XAI_API_KEY`        | xAI API key            | If using xAI                    |
| `GROQ_API_KEY`       | Groq API key           | If using Groq                   |
| `GEMINI_API_KEY`     | Google Gemini API key  | If using Gemini                 |
| `OLLAMA_HOST`        | Ollama server URL      | Optional, defaults to localhost |
| `RULEY_CONFIG`       | Path to config file    | Optional                        |
| `RULEY_NO_CONFIRM`   | Skip cost confirmation | Optional                        |

---

## Technical Specifications

### Token Counting

Use `tiktoken-rs` for accurate token counting:

| Provider        | Tokenizer                   |
| --------------- | --------------------------- |
| OpenAI (GPT-4o) | `o200k_base`                |
| OpenAI (GPT-4)  | `cl100k_base`               |
| Anthropic       | `cl100k_base` (approximate) |
| Others          | `cl100k_base` (fallback)    |

### Chunking Strategy

```text
┌─────────────────────────────────────────────────────┐
│                  Full Codebase                      │
│                  (500k tokens)                      │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ Chunk 1  │ │ Chunk 2  │ │ Chunk 3  │ │ Chunk 4  │ │ Chunk 5  │
│ 100k     │ │ 100k     │ │ 100k     │ │ 100k     │ │ 100k     │
└──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘
     │            │            │            │            │
     ▼            ▼            ▼            ▼            ▼
┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐
│ LLM Call │ │ LLM Call │ │ LLM Call │ │ LLM Call │ │ LLM Call │
│ Generate │ │ Enhance  │ │ Enhance  │ │ Enhance  │ │ Finalize │
└──────────┘ └──────────┘ └──────────┘ └──────────┘ └──────────┘
     │            │            │            │            │
     └────────────┴────────────┴────────────┴────────────┘
                                │
                                ▼
                    ┌──────────────────────┐
                    │   Final Rules File   │
                    └──────────────────────┘
```

### Tree-sitter Compression

Compression extracts signatures and structure while removing implementation details:

**Before (TypeScript):**

```typescript
export async function processUser(
  userId: string,
  options: ProcessOptions,
): Promise<UserResult> {
  const user = await db.users.findById(userId);
  if (!user) {
    throw new NotFoundError("User not found");
  }

  const validated = validateOptions(options);
  const result = await performProcessing(user, validated);

  await audit.log("user_processed", { userId, result });

  return result;
}
```

**After (compressed):**

```typescript
export async function processUser(
  userId: string,
  options: ProcessOptions
): Promise<UserResult>
⋮----
```

**Compression targets ~70% token reduction** while preserving:

- Function/method signatures
- Type definitions
- Class structures
- Import statements
- Export declarations

### Supported Languages (Tree-sitter)

| Language              | Parser Crate                       | Priority |
| --------------------- | ---------------------------------- | -------- |
| TypeScript/JavaScript | `tree-sitter-typescript`           | P0       |
| Python                | `tree-sitter-python`               | P0       |
| Rust                  | `tree-sitter-rust`                 | P0       |
| Go                    | `tree-sitter-go`                   | P1       |
| Java                  | `tree-sitter-java`                 | P1       |
| C/C++                 | `tree-sitter-c`, `tree-sitter-cpp` | P2       |
| Ruby                  | `tree-sitter-ruby`                 | P2       |
| PHP                   | `tree-sitter-php`                  | P2       |

### Error Handling

```rust
/// Ruley-specific errors - for when your codebase gets unruly
#[derive(Debug, thiserror::Error)]
pub enum RuleyError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Repository error: {0}")]
    Repository(#[from] git2::Error),

    #[error("File system error: {0}")]
    FileSystem(#[from] std::io::Error),

    #[error("LLM provider error: {provider} - {message}")]
    Provider { provider: String, message: String },

    #[error("Rate limited by {provider}, retry after {retry_after:?}")]
    RateLimited {
        provider: String,
        retry_after: Option<Duration>,
    },

    #[error("Token limit exceeded: {tokens} tokens > {limit} limit")]
    TokenLimitExceeded { tokens: usize, limit: usize },

    #[error("Compression error for {language}: {message}")]
    Compression { language: String, message: String },

    #[error("Output format error: {0}")]
    OutputFormat(String),
}
```

### Retry Strategy

```rust
pub struct RetryConfig {
    /// Maximum retry attempts
    pub max_retries: u32, // default: 3

    /// Initial backoff duration
    pub initial_backoff: Duration, // default: 1s

    /// Maximum backoff duration
    pub max_backoff: Duration, // default: 60s

    /// Backoff multiplier
    pub multiplier: f64, // default: 2.0

    /// Add jitter to prevent thundering herd
    pub jitter: bool, // default: true
}
```

Retry on:

- HTTP 429 (Rate Limited)
- HTTP 500, 502, 503, 504 (Server errors)
- Network timeouts
- Connection refused (for Ollama)

Do NOT retry on:

- HTTP 400 (Bad Request)
- HTTP 401 (Unauthorized)
- HTTP 403 (Forbidden)
- Context length exceeded

---

## Dependencies

### Cargo.toml

```toml
[package]
name = "ruley"
version = "0.1.0"
edition = "2024"
authors = ["UncleSp1d3r <unclesp1d3r@evilbitlabs.io>"]
license = "Apache-2.0"
description = "Make your codebase ruley - generate AI IDE rules from codebases"
repository = "https://github.com/EvilBit-Labs/ruley"
homepage = "https://github.com/EvilBit-Labs/ruley"
keywords = ["ai", "llm", "cursor", "claude", "copilot", "ide", "rules"]
categories = ["command-line-utilities", "development-tools"]

[dependencies]
# CLI
clap = { version = "4", features = ["derive", "env"] }

# Async runtime
tokio = { version = "1", features = ["full"] }

# LLM providers (feature-gated)
llm = { version = "0.1", optional = true }
# OR use multi-llm
# multi-llm = { version = "1", optional = true }

# Alternative: individual provider SDKs
reqwest = { version = "0.12", features = ["json", "stream"] }
async-trait = "0.1"
futures = "0.3"

# Token counting
tiktoken-rs = "0.6"

# Git operations
git2 = "0.19"

# File matching
ignore = "0.4"  # ripgrep's gitignore library
globset = "0.4"

# Tree-sitter (feature-gated per language)
tree-sitter = "0.24"
tree-sitter-typescript = { version = "0.23", optional = true }
tree-sitter-python = { version = "0.23", optional = true }
tree-sitter-rust = { version = "0.23", optional = true }
tree-sitter-go = { version = "0.23", optional = true }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Progress display
indicatif = "0.17"
console = "0.15"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Misc
once_cell = "1"
regex = "1"
chrono = "0.4"

[features]
default = ["anthropic", "openai", "compression-typescript"]

# LLM Providers
anthropic = []
openai = []
ollama = []
openrouter = []
xai = []
groq = []
gemini = []
all-providers = [
  "anthropic",
  "openai",
  "ollama",
  "openrouter",
  "xai",
  "groq",
  "gemini",
]

# Compression languages
compression-typescript = ["tree-sitter-typescript"]
compression-python = ["tree-sitter-python"]
compression-rust = ["tree-sitter-rust"]
compression-go = ["tree-sitter-go"]
compression-all = [
  "compression-typescript",
  "compression-python",
  "compression-rust",
  "compression-go",
]

[profile.release]
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

---

## Implementation Plan

### Phase 1: Foundation (Week 1)

| Day | Tasks                                                    |
| --- | -------------------------------------------------------- |
| 1-2 | Project setup, CLI skeleton with clap, basic error types |
| 3-4 | File walker with gitignore support using `ignore` crate  |
| 5   | Git clone support with `git2`                            |

**Deliverable:** Can walk local/remote repos and list files

### Phase 2: LLM Integration (Week 1-2)

| Day | Tasks                                        |
| --- | -------------------------------------------- |
| 6-7 | LLM provider trait, Anthropic implementation |
| 8   | OpenAI implementation                        |
| 9   | Token counting with tiktoken-rs              |
| 10  | Chunking logic, cost estimation              |

**Deliverable:** Can send codebase to LLM and get response

### Phase 3: Output & Polish (Week 2)

| Day | Tasks                                |
| --- | ------------------------------------ |
| 11  | Cursor .mdc output formatter         |
| 12  | Prompt templates, rule type handling |
| 13  | Progress bars, UX polish             |
| 14  | Config file support                  |

**Deliverable:** Fully functional v0.1.0

### Phase 4: Compression (Week 3)

| Day   | Tasks                                     |
| ----- | ----------------------------------------- |
| 15-16 | Tree-sitter integration for TypeScript    |
| 17    | Python, Rust parser support               |
| 18-19 | Compression logic, testing                |
| 20    | Additional providers (Ollama, OpenRouter) |

**Deliverable:** v0.2.0 with compression

### Phase 5: Hardening (Week 3-4)

| Day   | Tasks                             |
| ----- | --------------------------------- |
| 21-22 | Integration tests, edge cases     |
| 23    | Documentation, README             |
| 24    | CI/CD setup, release automation   |
| 25    | Binary releases for all platforms |

**Deliverable:** v1.0.0 release

---

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gitignore_pattern_matching() {
        let ignorer = GitIgnorer::new(".gitignore").unwrap();
        assert!(ignorer.is_ignored("node_modules/foo.js"));
        assert!(!ignorer.is_ignored("src/main.rs"));
    }

    #[test]
    fn test_token_counting() {
        let counter = TokenCounter::new("cl100k_base");
        let tokens = counter.count("Hello, world!");
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_chunking() {
        let chunker = Chunker::new(1000);
        let chunks = chunker.chunk(&large_text, 100);
        assert!(chunks.iter().all(|c| c.tokens <= 100));
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_local_repo_analysis() {
    let result = analyze_repo("tests/fixtures/sample-ts-project", &Config::default()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().files.len() > 0);
}

#[tokio::test]
async fn test_remote_repo_clone() {
    let result = clone_repo("https://github.com/test/small-repo").await;
    assert!(result.is_ok());
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_llm_generation() {
    let provider = AnthropicProvider::from_env().unwrap();
    let result = provider
        .complete(&test_messages(), &default_options())
        .await;
    assert!(result.is_ok());
}
```

### Test Fixtures

```text
tests/
├── fixtures/
│   ├── sample-ts-project/
│   │   ├── src/
│   │   │   ├── index.ts
│   │   │   └── utils.ts
│   │   ├── package.json
│   │   └── .gitignore
│   │
│   ├── sample-python-project/
│   │   ├── src/
│   │   │   └── main.py
│   │   └── pyproject.toml
│   │
│   └── expected-outputs/
│       ├── sample-ts.rules.mdc
│       └── sample-python.rules.mdc
```

---

## Distribution

### Binary Releases

Build for all major platforms using [GoReleaser](https://goreleaser.com/customization/builds/rust/):

| Platform    | Target                      |
| ----------- | --------------------------- |
| Linux x64   | `x86_64-unknown-linux-gnu`  |
| Linux ARM64 | `aarch64-unknown-linux-gnu` |
| Linux musl  | `x86_64-unknown-linux-musl` |
| macOS x64   | `x86_64-apple-darwin`       |
| macOS ARM64 | `aarch64-apple-darwin`      |
| Windows x64 | `x86_64-pc-windows-gnu`     |

### GoReleaser Configuration

`.goreleaser.yaml`:

```yaml
version: 2

project_name: ruley

builds:
  - builder: rust
    binary: ruley
    dir: .
    targets:
      - x86_64-unknown-linux-gnu
      - x86_64-unknown-linux-musl
      - aarch64-unknown-linux-gnu
      - x86_64-apple-darwin
      - aarch64-apple-darwin
      - x86_64-pc-windows-gnu
    flags:
      - --release
    env:
      - CGO_ENABLED=0

archives:
  - format: tar.gz
    name_template: '{{ .ProjectName }}_{{ .Version }}_{{ .Os }}_{{ .Arch }}'
    format_overrides:
      - goos: windows
        format: zip
    files:
      - LICENSE
      - README.md

checksum:
  name_template: checksums.txt

changelog:
  sort: asc
  filters:
    exclude:
      - '^docs:'
      - '^test:'
      - '^chore:'

brews:
  - name: ruley
    homepage: https://github.com/EvilBit-Labs/ruley
    description: Make your codebase ruley - generate AI IDE rules from codebases
    license: Apache-2.0
    repository:
      owner: EvilBit-Labs
      name: homebrew-tap
    folder: Formula
    install: |
      bin.install "ruley"

# Publish to crates.io on release
after:
  hooks:
    - cmd: cargo publish {{ if .IsSnapshot }}--dry-run{{ end }} --quiet 
        --no-verify
```

### Prerequisites for GoReleaser Rust Builds

```bash
# Install required tools
rustup target add x86_64-unknown-linux-gnu
rustup target add aarch64-unknown-linux-gnu
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
rustup target add x86_64-pc-windows-gnu

# Install cargo-zigbuild for cross-compilation
cargo install cargo-zigbuild

# Install zig (required by cargo-zigbuild)
# macOS
brew install zig
# Linux
# See https://ziglang.org/download/
```

### Installation Methods

```bash
# Cargo (crates.io)
cargo install ruley

# Homebrew (macOS/Linux)
brew install EvilBit-Labs/tap/ruley

# Binary download (auto-detect platform)
curl -fsSL https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley_$(uname -s)_$(uname -m).tar.gz | tar xz
sudo mv ruley /usr/local/bin/

# Windows (PowerShell)
irm https://github.com/EvilBit-Labs/ruley/releases/latest/download/ruley_Windows_x86_64.zip -OutFile ruley.zip
Expand-Archive ruley.zip -DestinationPath .
Move-Item ruley.exe $env:USERPROFILE\bin\

# Nix
nix-env -iA nixpkgs.ruley
```

### CI/CD (GitHub Actions with GoReleaser)

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    tags:
      - v*

permissions:
  contents: write

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Set up Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Zig
        uses: goto-bus-stop/setup-zig@v2

      - name: Install cargo-zigbuild
        run: cargo install cargo-zigbuild

      - name: Add Rust targets
        run: |
          rustup target add x86_64-unknown-linux-gnu
          rustup target add x86_64-unknown-linux-musl
          rustup target add aarch64-unknown-linux-gnu
          rustup target add x86_64-apple-darwin
          rustup target add aarch64-apple-darwin
          rustup target add x86_64-pc-windows-gnu

      - name: Run GoReleaser
        uses: goreleaser/goreleaser-action@v6
        with:
          version: ~> v2
          args: release --clean
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: binaries
          path: dist/*
```

---

## Future Considerations

### v1.1 Roadmap

- [ ] CLAUDE.md output format for Claude Code
- [ ] Watch mode for development (`--watch`)
- [ ] Incremental updates (only regenerate on changes)
- [ ] Plugin system for custom formatters

### v1.2 Roadmap

- [ ] TUI mode with interactive provider/model selection
- [ ] Cost tracking and budgets
- [ ] Team/org configuration sharing
- [ ] Rule merging from multiple sources

### v2.0 Roadmap

- [ ] IDE extensions (VS Code, JetBrains)
- [ ] Rule validation and linting
- [ ] A/B testing of different rule sets
- [ ] Analytics on rule effectiveness

### Community

- [ ] Curated prompt library
- [ ] Shared rule templates
- [ ] Language-specific best practices

---

## License

```text
Copyright 2025 EvilBit Labs

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```

---

## References

- [Cursor Rules Documentation](https://docs.cursor.com/context/rules-for-ai)
- [Claude Code Project Instructions](https://docs.anthropic.com/en/docs/claude-code)
- [GitHub Copilot Custom Instructions](https://docs.github.com/en/copilot/customizing-copilot/adding-custom-instructions-for-github-copilot)
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
- [tiktoken](https://github.com/openai/tiktoken)
- [rulefy (inspiration)](https://github.com/niklub/rulefy)
- [repomix (inspiration)](https://github.com/yamadashy/repomix)

---

_ruley: Because unruly codebases need rules too._
