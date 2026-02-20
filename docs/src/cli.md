# Command-Line Interface

[TOC]

## Usage

```text
ruley [OPTIONS] [PATH]
```

**PATH**: Path to repository (local path or remote URL). Defaults to `.` (current directory).

## Options

### Core Options

| Flag                     | Env Variable     | Default              | Description                                                  |
| ------------------------ | ---------------- | -------------------- | ------------------------------------------------------------ |
| `-p, --provider <NAME>`  | `RULEY_PROVIDER` | `anthropic`          | LLM provider (`anthropic`, `openai`, `ollama`, `openrouter`) |
| `-m, --model <NAME>`     | `RULEY_MODEL`    | *(provider default)* | Model to use                                                 |
| `-f, --format <FORMATS>` | `RULEY_FORMAT`   | `cursor`             | Output format(s), comma-separated                            |
| `-o, --output <PATH>`    | `RULEY_OUTPUT`   | *(format default)*   | Output file path (single format only)                        |
| `-c, --config <PATH>`    | `RULEY_CONFIG`   | `ruley.toml`         | Config file path                                             |

### Generation Options

| Flag                    | Env Variable         | Default  | Description                                                      |
| ----------------------- | -------------------- | -------- | ---------------------------------------------------------------- |
| `--description <TEXT>`  | `RULEY_DESCRIPTION`  | *(none)* | Focus area for rule generation                                   |
| `--rule-type <TYPE>`    | `RULEY_RULE_TYPE`    | `auto`   | Cursor rule type (`auto`, `always`, `manual`, `agent-requested`) |
| `--compress`            | `RULEY_COMPRESS`     | `false`  | Enable tree-sitter compression                                   |
| `--chunk-size <N>`      | `RULEY_CHUNK_SIZE`   | `100000` | Max tokens per LLM chunk                                         |
| `--repomix-file <PATH>` | `RULEY_REPOMIX_FILE` | *(none)* | Use pre-packed repomix file as input                             |

### Filtering Options

| Flag                  | Description                              |
| --------------------- | ---------------------------------------- |
| `--include <PATTERN>` | Include only matching files (repeatable) |
| `--exclude <PATTERN>` | Exclude matching files (repeatable)      |

### Behavior Options

| Flag                            | Env Variable        | Default  | Description                                                        |
| ------------------------------- | ------------------- | -------- | ------------------------------------------------------------------ |
| `--no-confirm`                  | `RULEY_NO_CONFIRM`  | `false`  | Skip cost confirmation prompt                                      |
| `--dry-run`                     | `RULEY_DRY_RUN`     | `false`  | Show plan without calling LLM                                      |
| `--on-conflict <STRATEGY>`      | `RULEY_ON_CONFLICT` | `prompt` | Conflict resolution (`prompt`, `overwrite`, `skip`, `smart-merge`) |
| `--retry-on-validation-failure` |                     | `false`  | Auto-retry with LLM fix on validation failure                      |
| `--no-deconflict`               |                     | `false`  | Disable LLM-based deconfliction with existing rules                |
| `--no-semantic-validation`      |                     | `false`  | Disable all semantic validation checks                             |

### Output Options

| Flag        | Description                                      |
| ----------- | ------------------------------------------------ |
| `-v`        | Increase verbosity (`-v` = DEBUG, `-vv` = TRACE) |
| `-q`        | Suppress non-essential output                    |
| `--version` | Print version information                        |
| `--help`    | Print help information                           |

## Environment Variables

All CLI flags can be set via `RULEY_*` environment variables. CLI flags take precedence over environment variables, which take precedence over config file values.

### Provider API Keys

| Variable             | Provider   | Required                                     |
| -------------------- | ---------- | -------------------------------------------- |
| `ANTHROPIC_API_KEY`  | Anthropic  | When using `--provider anthropic`            |
| `OPENAI_API_KEY`     | OpenAI     | When using `--provider openai`               |
| `OLLAMA_HOST`        | Ollama     | Optional (default: `http://localhost:11434`) |
| `OPENROUTER_API_KEY` | OpenRouter | When using `--provider openrouter`           |

## Examples

### Basic Usage

```bash
# Analyze current directory with defaults
ruley

# Analyze a specific project
ruley /path/to/project
```

### Provider Selection

```bash
# Use OpenAI with a specific model
ruley --provider openai --model gpt-4o

# Use local Ollama
ruley --provider ollama --model llama3.1

# Use OpenRouter with Claude
ruley --provider openrouter --model anthropic/claude-3.5-sonnet
```

### Format Control

```bash
# Generate Cursor rules (default)
ruley --format cursor

# Generate multiple formats
ruley --format cursor,claude,copilot

# Generate all formats
ruley --format all

# Write to a specific path (single format only)
ruley --format claude --output ./docs/CLAUDE.md
```

### Compression and Performance

```bash
# Enable tree-sitter compression (~70% token reduction)
ruley --compress

# Adjust chunk size for large codebases
ruley --chunk-size 200000

# Use a pre-packed repomix file
ruley --repomix-file ./codebase.xml
```

### Cost Management

```bash
# Preview without calling the LLM
ruley --dry-run

# Skip the cost confirmation prompt
ruley --no-confirm
```

### Conflict Resolution

```bash
# Overwrite existing rule files
ruley --on-conflict overwrite

# Skip if files already exist
ruley --on-conflict skip

# Use LLM to smart-merge with existing rules
ruley --on-conflict smart-merge
```

### Filtering Files

```bash
# Only include Rust files
ruley --include "**/*.rs"

# Exclude test directories
ruley --exclude "**/tests/**" --exclude "**/benches/**"
```
