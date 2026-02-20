# LLM Providers

[TOC]

ruley supports multiple LLM providers. Each provider is feature-gated at compile time and requires its own API key (except Ollama).

## Provider Comparison

| Provider       | API Key Required | Local | Default Model                 | Context Window  |
| -------------- | ---------------- | ----- | ----------------------------- | --------------- |
| **Anthropic**  | Yes              | No    | `claude-sonnet-4-5-20250929`  | 200K tokens     |
| **OpenAI**     | Yes              | No    | `gpt-4o`                      | 128K tokens     |
| **Ollama**     | No               | Yes   | `llama3.1:70b`                | ~100K tokens    |
| **OpenRouter** | Yes              | No    | `anthropic/claude-3.5-sonnet` | Varies by model |

## Anthropic

Anthropic's Claude models are the default provider and generally produce excellent rule quality.

### Setup

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Usage

```bash
# Uses default model (Claude Sonnet 4.5)
ruley --provider anthropic

# Specify a model
ruley --provider anthropic --model claude-sonnet-4-5-20250929
```

### Config File

```toml
[general]
provider = "anthropic"

[providers.anthropic]
model = "claude-sonnet-4-5-20250929"
max_tokens = 8192
```

## OpenAI

OpenAI's GPT models provide strong rule generation with fast response times.

### Setup

```bash
export OPENAI_API_KEY="sk-..."
```

### Usage

```bash
ruley --provider openai --model gpt-4o
```

### Config File

```toml
[general]
provider = "openai"

[providers.openai]
model = "gpt-4o"
max_tokens = 4096
```

## Ollama

Ollama runs models locally. No API key is needed, and there are no per-token costs. This is ideal for privacy-sensitive codebases or offline use.

### Setup

1. [Install Ollama](https://ollama.ai/)
2. Pull a model: `ollama pull llama3.1:70b`
3. Start the server: `ollama serve`

### Usage

```bash
ruley --provider ollama --model llama3.1

# Custom Ollama host
OLLAMA_HOST="http://192.168.1.100:11434" ruley --provider ollama
```

### Config File

```toml
[general]
provider = "ollama"

[providers.ollama]
host = "http://localhost:11434"
model = "llama3.1:70b"
```

### Considerations

- Rule quality depends heavily on the model size. Larger models (70B+) produce better results.
- Local models have smaller context windows. Use `--compress` and `--chunk-size` to manage large codebases.
- No cost confirmation is shown since Ollama is free to use.

## OpenRouter

OpenRouter provides access to models from multiple providers through a single API. It fetches dynamic pricing from the OpenRouter API for accurate cost estimation.

### Setup

```bash
export OPENROUTER_API_KEY="sk-or-..."
```

### Usage

```bash
ruley --provider openrouter --model anthropic/claude-3.5-sonnet
```

### Config File

```toml
[general]
provider = "openrouter"

[providers.openrouter]
model = "anthropic/claude-3.5-sonnet"
max_tokens = 8192
```

## Feature Flags

Providers are compiled in via Cargo feature flags. The default build includes `anthropic` and `openai`.

| Feature         | Provider            |
| --------------- | ------------------- |
| `anthropic`     | Anthropic (default) |
| `openai`        | OpenAI (default)    |
| `ollama`        | Ollama              |
| `openrouter`    | OpenRouter          |
| `all-providers` | All of the above    |

To include all providers when building from source:

```bash
cargo install ruley --features all-providers
```

## Choosing a Provider

- **Best quality**: Anthropic Claude (default) -- excellent at understanding code conventions
- **Fastest**: OpenAI GPT-4o -- lower latency per request
- **Free / Private**: Ollama -- no API costs, data stays local
- **Flexible**: OpenRouter -- access to many models through one API
