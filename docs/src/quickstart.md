# Quick Start

This guide walks you through generating your first set of AI IDE rules with ruley.

## Prerequisites

1. ruley installed (see [Installation](./installation.md))
2. An API key for at least one LLM provider

## Step 1: Set Your API Key

Set the environment variable for your chosen provider:

{{#tabs }} {{#tab name="Anthropic" }}

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

{{#endtab }} {{#tab name="OpenAI" }}

```bash
export OPENAI_API_KEY="sk-..."
```

{{#endtab }} {{#tab name="Ollama" }}

```bash
# No API key needed -- just ensure Ollama is running
ollama serve
```

{{#endtab }} {{#tab name="OpenRouter" }}

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

{{#endtab }} {{#endtabs }}

## Step 2: Generate Rules

Navigate to your project directory and run ruley:

```bash
cd /path/to/your/project
ruley
```

By default, ruley uses Anthropic Claude and generates Cursor format rules.

## Step 3: Review the Output

ruley shows you:

1. **Scan results** -- How many files were discovered
2. **Compression stats** -- Token reduction from tree-sitter compression
3. **Cost estimate** -- Estimated LLM cost before proceeding
4. **Confirmation prompt** -- You must approve before the LLM call is made
5. **Generated files** -- Where the rule files were written

## Common Variations

### Use a Different Provider

```bash
ruley --provider openai --model gpt-4o
```

### Generate Multiple Formats

```bash
ruley --format cursor,claude,copilot
```

### Generate All Formats at Once

```bash
ruley --format all
```

### Enable Tree-Sitter Compression

```bash
ruley --compress
```

### Analyze a Specific Directory

```bash
ruley ./my-project --compress
```

### Dry Run (Preview Without Calling LLM)

```bash
ruley --dry-run
```

This shows what would be processed (file count, token estimate, cost) without making any LLM calls. Useful for checking costs before committing.

### Skip Cost Confirmation

```bash
ruley --no-confirm
```

### Use a Local Ollama Model

```bash
ruley --provider ollama --model llama3.1
```

## What Happens Next

The generated rule files are placed in your project directory at the standard locations for each format. Your AI IDE tools will automatically pick them up:

- **Cursor**: `.cursor/rules/*.mdc` -- loaded automatically by Cursor IDE
- **Claude**: `CLAUDE.md` -- read by Claude Code as project context
- **Copilot**: `.github/copilot-instructions.md` -- loaded by GitHub Copilot
- **Windsurf**: `.windsurfrules` -- loaded by Windsurf IDE
- **Aider**: `.aider.conf.yml` -- loaded by Aider CLI

Commit the generated files to your repository so your whole team benefits from consistent AI assistance.

## Next Steps

- [Command-Line Interface](./cli.md) -- Full reference for all CLI options
- [Configuration](./configuration.md) -- Set up a `ruley.toml` for your project
- [LLM Providers](./providers.md) -- Compare providers and choose the best fit
- [Output Formats](./output-formats.md) -- Understand what each format produces
