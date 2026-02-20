# Configuration

[TOC]

ruley supports hierarchical configuration from multiple sources. This page documents the configuration file format and precedence rules.

## Configuration Precedence

Configuration is resolved in this order (highest to lowest precedence):

1. **CLI flags** -- Explicitly provided command-line arguments
2. **Environment variables** -- `RULEY_*` prefix (handled by clap's `env` attribute)
3. **Config files** -- Loaded and merged in discovery order (see below)
4. **Built-in defaults** -- Hardcoded in the CLI parser

When a CLI flag is explicitly provided, it always wins. When it's not provided (using the default), the config file value is used instead.

## Config File Discovery

Config files are discovered and merged in this order (later overrides earlier):

1. `~/.config/ruley/config.toml` -- User-level global config
2. `ruley.toml` in the git repository root -- Project-level config
3. `./ruley.toml` in the current directory -- Working directory config
4. Explicit `--config <path>` -- If provided, overrides all above

All discovered files are merged. Duplicate keys in later files override earlier ones.

## Configuration File Format

Configuration files use TOML format. All sections are optional.

### Complete Example

```toml
[general]
provider = "anthropic"
model = "claude-sonnet-4-5-20250929"
format = ["cursor", "claude"]
compress = true
chunk_size = 100000
no_confirm = false
rule_type = "auto"

[output]
formats = ["cursor", "claude"]
on_conflict = "prompt"

[output.paths]
cursor = ".cursor/rules/project-rules.mdc"
claude = "CLAUDE.md"

[include]
patterns = ["**/*.rs", "**/*.toml"]

[exclude]
patterns = ["**/target/**", "**/node_modules/**"]

[chunking]
chunk_size = 100000
overlap = 10000

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
max_tokens = 8192

[validation]
enabled = true
retry_on_failure = false
max_retries = 3

[validation.semantic]
check_file_paths = true
check_contradictions = true
check_consistency = true
check_reality = true

[finalization]
enabled = true
deconflict = true
normalize_formatting = true
inject_metadata = true
```

### `[general]` Section

Core settings for the pipeline.

| Key          | Type     | Default              | Description                    |
| ------------ | -------- | -------------------- | ------------------------------ |
| `provider`   | string   | `"anthropic"`        | LLM provider name              |
| `model`      | string   | *(provider default)* | Model to use                   |
| `format`     | string[] | `["cursor"]`         | Output formats                 |
| `compress`   | bool     | `false`              | Enable tree-sitter compression |
| `chunk_size` | int      | `100000`             | Max tokens per LLM chunk       |
| `no_confirm` | bool     | `false`              | Skip cost confirmation         |
| `rule_type`  | string   | `"auto"`             | Cursor rule type               |

### `[output]` Section

Output format and path configuration.

| Key              | Type     | Default            | Description                     |
| ---------------- | -------- | ------------------ | ------------------------------- |
| `formats`        | string[] | `[]`               | Alternative to `general.format` |
| `on_conflict`    | string   | `"prompt"`         | Conflict resolution strategy    |
| `paths.<format>` | string   | *(format default)* | Custom output path per format   |

### `[include]` / `[exclude]` Sections

File filtering using glob patterns.

| Key        | Type     | Default | Description                     |
| ---------- | -------- | ------- | ------------------------------- |
| `patterns` | string[] | `[]`    | Glob patterns for file matching |

### `[chunking]` Section

Controls how large codebases are split for LLM processing.

| Key          | Type | Default           | Description                  |
| ------------ | ---- | ----------------- | ---------------------------- |
| `chunk_size` | int  | `100000`          | Max tokens per chunk         |
| `overlap`    | int  | `chunk_size / 10` | Token overlap between chunks |

### `[providers]` Section

Provider-specific configuration. Each provider has its own subsection.

**`[providers.anthropic]`** / **`[providers.openai]`** / **`[providers.openrouter]`**:

| Key          | Type   | Description         |
| ------------ | ------ | ------------------- |
| `model`      | string | Model name override |
| `max_tokens` | int    | Max output tokens   |

**`[providers.ollama]`**:

| Key     | Type   | Description       |
| ------- | ------ | ----------------- |
| `host`  | string | Ollama server URL |
| `model` | string | Model name        |

### `[validation]` Section

Controls validation of generated rules.

| Key                | Type | Default | Description             |
| ------------------ | ---- | ------- | ----------------------- |
| `enabled`          | bool | `true`  | Enable validation       |
| `retry_on_failure` | bool | `false` | Auto-retry with LLM fix |
| `max_retries`      | int  | `3`     | Max auto-fix attempts   |

**`[validation.semantic]`** -- Semantic validation checks:

| Key                    | Type | Default | Description                          |
| ---------------------- | ---- | ------- | ------------------------------------ |
| `check_file_paths`     | bool | `true`  | Verify referenced file paths exist   |
| `check_contradictions` | bool | `true`  | Detect contradictory rules           |
| `check_consistency`    | bool | `true`  | Cross-format consistency check       |
| `check_reality`        | bool | `true`  | Verify language/framework references |

### `[finalization]` Section

Controls post-processing of generated rules.

| Key                    | Type | Default | Description                                 |
| ---------------------- | ---- | ------- | ------------------------------------------- |
| `enabled`              | bool | `true`  | Enable finalization                         |
| `deconflict`           | bool | `true`  | LLM-based deconfliction with existing rules |
| `normalize_formatting` | bool | `true`  | Normalize line endings and whitespace       |
| `inject_metadata`      | bool | `true`  | Add timestamp/version/provider headers      |
