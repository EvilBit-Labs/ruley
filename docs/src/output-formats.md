# Output Formats

[TOC]

ruley generates rule files in 7 formats. Each format targets a specific AI IDE tool and follows its conventions for file naming, structure, and content.

## Format Overview

| Format     | Output File                       | Description                       |
| ---------- | --------------------------------- | --------------------------------- |
| `cursor`   | `.cursor/rules/*.mdc`             | Cursor IDE rules with frontmatter |
| `claude`   | `CLAUDE.md`                       | Claude Code project instructions  |
| `copilot`  | `.github/copilot-instructions.md` | GitHub Copilot instructions       |
| `windsurf` | `.windsurfrules`                  | Windsurf IDE rules                |
| `aider`    | `.aider.conf.yml`                 | Aider conventions                 |
| `generic`  | `.ai-rules.md`                    | Generic markdown rules            |
| `json`     | `.ai-rules.json`                  | Machine-readable JSON             |

## Selecting Formats

### Single Format (Default)

```bash
# Cursor format (default)
ruley

# Claude format
ruley --format claude
```

### Multiple Formats

```bash
ruley --format cursor,claude,copilot
```

### All Formats

```bash
ruley --format all
```

### Custom Output Path

For a single format, you can override the output path:

```bash
ruley --format claude --output ./docs/CLAUDE.md
```

For multiple formats, use the config file:

```toml
[output.paths]
cursor = ".cursor/rules/project-rules.mdc"
claude = "docs/CLAUDE.md"
```

## Format Details

### Cursor (`.mdc`)

Cursor IDE rules use the `.mdc` (markdown component) format with YAML frontmatter. Rules are placed in `.cursor/rules/` and loaded automatically by Cursor.

The `--rule-type` flag controls the frontmatter `alwaysApply` field:

| Rule Type         | Behavior                            |
| ----------------- | ----------------------------------- |
| `auto`            | LLM decides based on rule content   |
| `always`          | Rules always apply to every file    |
| `manual`          | Rules must be manually activated    |
| `agent-requested` | Rules are requested by the AI agent |

### Claude (`CLAUDE.md`)

A single markdown file at the project root. Claude Code reads this file as project context for all conversations. Content is structured as guidelines and conventions in standard markdown.

### Copilot (`.github/copilot-instructions.md`)

GitHub Copilot's project-level instructions file. Placed in the `.github/` directory. Content is natural language instructions that guide Copilot's suggestions.

### Windsurf (`.windsurfrules`)

Windsurf IDE rules file at the project root. Similar to Cursor rules but without frontmatter. Content is structured as conventions and patterns.

### Aider (`.aider.conf.yml`)

Aider's configuration file in YAML format. Contains conventions and patterns that guide Aider's code generation.

### Generic (`.ai-rules.md`)

A generic markdown format not tied to any specific tool. Useful as a portable set of conventions that can be manually included in any AI assistant's context.

### JSON (`.ai-rules.json`)

Machine-readable JSON format for programmatic consumption. Contains the same convention data in a structured format suitable for integration with custom tools.

## Conflict Resolution

When output files already exist, ruley offers several strategies:

| Strategy      | Behavior                                       |
| ------------- | ---------------------------------------------- |
| `prompt`      | Ask the user what to do (default, interactive) |
| `overwrite`   | Replace existing files (creates backups)       |
| `skip`        | Skip formats where files exist                 |
| `smart-merge` | Use LLM to merge new rules with existing ones  |

Set the strategy via CLI or config:

```bash
ruley --on-conflict smart-merge
```

```toml
[output]
on_conflict = "smart-merge"
```

When `overwrite` is used, ruley creates `.bak` backups of existing files before writing.

## Single Analysis, Multiple Outputs

ruley performs a single LLM analysis of your codebase, then generates format-specific rules through a refinement step. This means:

- The analysis cost is paid once regardless of how many formats you generate
- Each format adds a small refinement LLM call to adapt the analysis to format-specific conventions
- Generating all 7 formats is only marginally more expensive than generating 1
