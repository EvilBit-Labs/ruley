# ruley Completion Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Complete the ruley CLI tool - turn existing scaffolding into a working codebase analyzer that generates AI IDE rules

**Architecture:** Modular pipeline: FileWalker -> Packer -> LLM Client -> Rule Parser -> Output Formatters. All LLM providers implement `LLMProvider` trait. All output formats implement `OutputFormatter` trait.

**Tech Stack:** Rust 2024, clap, tokio, reqwest, tiktoken-rs, git2, tree-sitter, serde

**Current State:** Project scaffolding exists with `todo!()` stubs. CLI parses, file walker works, but LLM clients, formatters, and orchestrator need implementation.

---

## Phase 1: Testing Infrastructure & LLM Providers

### Task 1: Add tempfile for tests

**Files:**

- Modify: `Cargo.toml`

**Step 1: Add tempfile dev dependency**

Add to `[dev-dependencies]`:

```toml
tempfile = "3"
```

**Step 2: Verify it compiles**

Run: `cargo build` Expected: Success

**Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore: add tempfile for testing"
```

---

### Task 2: Test file walker

**Files:**

- Modify: `src/packer/walker.rs`

**Step 1: Write test for basic file walking**

Add to end of `src/packer/walker.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_walk_finds_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("file1.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("file2.rs"), "fn test() {}").unwrap();

        let walker = FileWalker::new(tmp.path());
        let files = walker.walk().unwrap();

        assert_eq!(files.len(), 2);
    }
}
```

**Step 2: Run test to verify it passes**

Run: `cargo test packer::walker::tests::test_walk_finds_files` Expected: PASS (walker already works)

**Step 3: Write test for gitignore**

Add to tests module:

```rust
#[test]
fn test_respects_gitignore() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(".gitignore"), "ignored.txt\n").unwrap();
    fs::write(tmp.path().join("keep.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("ignored.txt"), "should skip").unwrap();

    let walker = FileWalker::new(tmp.path());
    let files = walker.walk().unwrap();

    let names: Vec<_> = files
        .iter()
        .filter_map(|p| p.file_name())
        .filter_map(|n| n.to_str())
        .collect();

    assert!(names.contains(&"keep.rs"));
    assert!(!names.contains(&"ignored.txt"));
}
```

**Step 4: Run test**

Run: `cargo test packer::walker::tests::test_respects_gitignore` Expected: PASS

**Step 5: Commit**

```bash
git add src/packer/walker.rs
git commit -m "test: add file walker tests"
```

---

### Task 3: Add glob pattern filtering to walker

**Files:**

- Modify: `src/packer/walker.rs`

**Step 1: Write failing test for include patterns**

Add to tests module:

```rust
#[test]
fn test_include_patterns() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("keep.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("skip.txt"), "text file").unwrap();

    let walker = FileWalker::new(tmp.path()).with_includes(vec!["*.rs".into()]);
    let files = walker.walk().unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].to_str().unwrap().ends_with(".rs"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test packer::walker::tests::test_include_patterns` Expected: FAIL - method `with_includes` not found

**Step 3: Add fields and builder methods**

Update `FileWalker` struct:

```rust
use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct FileWalker {
    root: std::path::PathBuf,
    include_patterns: Vec<String>,
    exclude_patterns: Vec<String>,
}

impl FileWalker {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
        }
    }

    pub fn with_includes(mut self, patterns: Vec<String>) -> Self {
        self.include_patterns = patterns;
        self
    }

    pub fn with_excludes(mut self, patterns: Vec<String>) -> Self {
        self.exclude_patterns = patterns;
        self
    }

    fn build_globset(patterns: &[String]) -> Result<Option<GlobSet>, RuleyError> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern)
                .map_err(|e| RuleyError::Config(format!("Invalid glob '{}': {}", pattern, e)))?;
            builder.add(glob);
        }
        Ok(Some(builder.build().map_err(|e| {
            RuleyError::Config(format!("Failed to build globset: {}", e))
        })?))
    }

    pub fn walk(&self) -> Result<Vec<std::path::PathBuf>, RuleyError> {
        let include_set = Self::build_globset(&self.include_patterns)?;
        let exclude_set = Self::build_globset(&self.exclude_patterns)?;

        let mut files = Vec::new();

        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = result.map_err(|e| {
                RuleyError::FileSystem(std::io::Error::other(format!(
                    "Failed to walk directory: {}",
                    e
                )))
            })?;

            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.path();
            let rel_path = path.strip_prefix(&self.root).unwrap_or(path);
            let rel_str = rel_path.to_string_lossy();

            // Check include patterns (if any, file must match)
            if let Some(ref inc) = include_set {
                if !inc.is_match(&*rel_str) && !inc.is_match(path) {
                    continue;
                }
            }

            // Check exclude patterns
            if let Some(ref exc) = exclude_set {
                if exc.is_match(&*rel_str) || exc.is_match(path) {
                    continue;
                }
            }

            files.push(path.to_path_buf());
        }

        Ok(files)
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test packer::walker::tests::test_include_patterns` Expected: PASS

**Step 5: Add test for exclude patterns**

```rust
#[test]
fn test_exclude_patterns() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("keep.rs"), "fn main() {}").unwrap();
    fs::write(tmp.path().join("skip.test.rs"), "test file").unwrap();

    let walker = FileWalker::new(tmp.path()).with_excludes(vec!["*.test.rs".into()]);
    let files = walker.walk().unwrap();

    assert_eq!(files.len(), 1);
    assert!(files[0].to_str().unwrap().ends_with("keep.rs"));
}
```

**Step 6: Run all walker tests**

Run: `cargo test packer::walker::tests` Expected: PASS

**Step 7: Commit**

```bash
git add src/packer/walker.rs
git commit -m "feat: add glob include/exclude filtering to file walker"
```

---

### Task 4: Implement Anthropic provider

**Files:**

- Modify: `src/llm/providers/anthropic.rs`

**Step 1: Write test for message serialization**

Add to end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let provider =
            AnthropicProvider::new("test-key".into(), "claude-sonnet-4-5-20250929".into());
        assert!(provider.model().contains("claude"));
    }

    #[test]
    fn test_pricing() {
        let provider =
            AnthropicProvider::new("test-key".into(), "claude-sonnet-4-5-20250929".into());
        let pricing = provider.pricing();
        assert!(pricing.input_per_1k > 0.0);
        assert!(pricing.output_per_1k > 0.0);
    }
}
```

**Step 2: Run tests**

Run: `cargo test llm::providers::anthropic::tests` Expected: PASS (these don't hit the API)

**Step 3: Implement the complete method**

Replace the `anthropic.rs` file contents:

```rust
use crate::llm::provider::{
    CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing, Role,
};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct Usage {
    input_tokens: usize,
    output_tokens: usize,
}

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| RuleyError::Config("ANTHROPIC_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, "claude-sonnet-4-5-20250929".to_string()))
    }
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        let mut system = None;
        let mut api_messages = Vec::new();

        for msg in messages {
            match msg.role {
                Role::System => {
                    system = Some(msg.content.clone());
                }
                Role::User => {
                    api_messages.push(AnthropicMessage {
                        role: "user".into(),
                        content: msg.content.clone(),
                    });
                }
                Role::Assistant => {
                    api_messages.push(AnthropicMessage {
                        role: "assistant".into(),
                        content: msg.content.clone(),
                    });
                }
            }
        }

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: options.max_tokens.unwrap_or(8192),
            messages: api_messages,
            system,
        };

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RuleyError::Provider {
                provider: "anthropic".into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(RuleyError::RateLimited {
                    provider: "anthropic".into(),
                    retry_after: None,
                });
            }

            return Err(RuleyError::Provider {
                provider: "anthropic".into(),
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let api_response: AnthropicResponse =
            response.json().await.map_err(|e| RuleyError::Provider {
                provider: "anthropic".into(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let content = api_response
            .content
            .into_iter()
            .map(|c| c.text)
            .collect::<Vec<_>>()
            .join("");

        Ok(CompletionResponse {
            content,
            input_tokens: api_response.usage.input_tokens,
            output_tokens: api_response.usage.output_tokens,
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 3.0,
            output_per_1k: 15.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let provider =
            AnthropicProvider::new("test-key".into(), "claude-sonnet-4-5-20250929".into());
        assert!(provider.model().contains("claude"));
    }

    #[test]
    fn test_pricing() {
        let provider =
            AnthropicProvider::new("test-key".into(), "claude-sonnet-4-5-20250929".into());
        let pricing = provider.pricing();
        assert!(pricing.input_per_1k > 0.0);
        assert!(pricing.output_per_1k > 0.0);
    }
}
```

**Step 4: Check provider.rs has Role enum**

Read `src/llm/provider.rs` and ensure `Role` enum exists. If not, add it:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    System,
    User,
    Assistant,
}
```

**Step 5: Run tests**

Run: `cargo test llm::providers::anthropic::tests` Expected: PASS

**Step 6: Commit**

```bash
git add src/llm/providers/anthropic.rs
git commit -m "feat: implement Anthropic API client"
```

---

### Task 5: Implement OpenAI provider

**Files:**

- Modify: `src/llm/providers/openai.rs`

**Step 1: Read current state**

Read `src/llm/providers/openai.rs` to see existing structure.

**Step 2: Write tests**

Add to end of file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let provider = OpenAIProvider::new("test-key".into(), "gpt-4o".into());
        assert!(provider.model().contains("gpt"));
    }
}
```

**Step 3: Implement complete method**

Replace file with full implementation:

```rust
use crate::llm::provider::{
    CompletionOptions, CompletionResponse, LLMProvider, Message, Pricing, Role,
};
use crate::utils::error::RuleyError;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
        }
    }

    pub fn from_env() -> Result<Self, RuleyError> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| RuleyError::Config("OPENAI_API_KEY not set".to_string()))?;
        Ok(Self::new(api_key, "gpt-4o".to_string()))
    }
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError> {
        let api_messages: Vec<OpenAIMessage> = messages
            .iter()
            .map(|m| OpenAIMessage {
                role: match m.role {
                    Role::System => "system".into(),
                    Role::User => "user".into(),
                    Role::Assistant => "assistant".into(),
                },
                content: m.content.clone(),
            })
            .collect();

        let request = OpenAIRequest {
            model: self.model.clone(),
            messages: api_messages,
            max_tokens: options.max_tokens,
        };

        let response = self
            .client
            .post(OPENAI_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| RuleyError::Provider {
                provider: "openai".into(),
                message: e.to_string(),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.as_u16() == 429 {
                return Err(RuleyError::RateLimited {
                    provider: "openai".into(),
                    retry_after: None,
                });
            }

            return Err(RuleyError::Provider {
                provider: "openai".into(),
                message: format!("HTTP {}: {}", status, body),
            });
        }

        let api_response: OpenAIResponse =
            response.json().await.map_err(|e| RuleyError::Provider {
                provider: "openai".into(),
                message: format!("Failed to parse response: {}", e),
            })?;

        let content = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(CompletionResponse {
            content,
            input_tokens: api_response.usage.prompt_tokens,
            output_tokens: api_response.usage.completion_tokens,
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn pricing(&self) -> Pricing {
        Pricing {
            input_per_1k: 2.5,
            output_per_1k: 10.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let provider = OpenAIProvider::new("test-key".into(), "gpt-4o".into());
        assert!(provider.model().contains("gpt"));
    }
}
```

**Step 4: Run tests**

Run: `cargo test llm::providers::openai::tests` Expected: PASS

**Step 5: Commit**

```bash
git add src/llm/providers/openai.rs
git commit -m "feat: implement OpenAI API client"
```

---

### Task 6: Implement token-aware chunker

**Files:**

- Modify: `src/llm/chunker.rs`
- Modify: `src/llm/tokenizer.rs`

**Step 1: Check tokenizer implementation**

Read `src/llm/tokenizer.rs` to understand current state.

**Step 2: Write test for chunker**

Add to `src/llm/chunker.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text_single_chunk() {
        let chunker = Chunker::new(1000);
        let text = "Hello, world!";
        let chunks = chunker.chunk(text).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_large_text_multiple_chunks() {
        let chunker = Chunker::new(10); // Very small limit
        let text = "This is a test string that should be split into multiple chunks because it exceeds the token limit.";
        let chunks = chunker.chunk(text).unwrap();
        assert!(chunks.len() > 1);
    }
}
```

**Step 3: Run test to verify it fails**

Run: `cargo test llm::chunker::tests` Expected: FAIL - todo!() panic

**Step 4: Implement chunker**

Replace `src/llm/chunker.rs`:

```rust
use crate::llm::tokenizer::TokenCounter;
use crate::utils::error::RuleyError;

pub struct Chunker {
    max_tokens: usize,
    counter: TokenCounter,
}

impl Chunker {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            counter: TokenCounter::default(),
        }
    }

    pub fn chunk(&self, text: &str) -> Result<Vec<String>, RuleyError> {
        let total_tokens = self.counter.count(text);

        if total_tokens <= self.max_tokens {
            return Ok(vec![text.to_string()]);
        }

        // Split by double newlines (paragraphs/sections)
        let sections: Vec<&str> = text.split("\n\n").collect();
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut current_tokens = 0;

        for section in sections {
            let section_tokens = self.counter.count(section);

            // If single section exceeds limit, split by lines
            if section_tokens > self.max_tokens {
                if !current_chunk.is_empty() {
                    chunks.push(current_chunk);
                    current_chunk = String::new();
                    current_tokens = 0;
                }

                for line in section.lines() {
                    let line_tokens = self.counter.count(line);

                    if current_tokens + line_tokens > self.max_tokens && !current_chunk.is_empty() {
                        chunks.push(current_chunk);
                        current_chunk = String::new();
                        current_tokens = 0;
                    }

                    if !current_chunk.is_empty() {
                        current_chunk.push('\n');
                    }
                    current_chunk.push_str(line);
                    current_tokens += line_tokens;
                }
                continue;
            }

            if current_tokens + section_tokens > self.max_tokens && !current_chunk.is_empty() {
                chunks.push(current_chunk);
                current_chunk = String::new();
                current_tokens = 0;
            }

            if !current_chunk.is_empty() {
                current_chunk.push_str("\n\n");
            }
            current_chunk.push_str(section);
            current_tokens += section_tokens;
        }

        if !current_chunk.is_empty() {
            chunks.push(current_chunk);
        }

        Ok(chunks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text_single_chunk() {
        let chunker = Chunker::new(1000);
        let text = "Hello, world!";
        let chunks = chunker.chunk(text).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[test]
    fn test_large_text_multiple_chunks() {
        let chunker = Chunker::new(10);
        let text = "This is a test string that should be split into multiple chunks because it exceeds the token limit.";
        let chunks = chunker.chunk(text).unwrap();
        assert!(chunks.len() > 1);
    }
}
```

**Step 5: Ensure TokenCounter has Default**

Check/update `src/llm/tokenizer.rs` to ensure `TokenCounter::default()` works.

**Step 6: Run tests**

Run: `cargo test llm::chunker::tests` Expected: PASS

**Step 7: Commit**

```bash
git add src/llm/chunker.rs src/llm/tokenizer.rs
git commit -m "feat: implement token-aware text chunking"
```

---

## Phase 2: Packer & Content Generation

### Task 7: Implement pack output format

**Files:**

- Modify: `src/packer/output.rs`

**Step 1: Read current state**

Read `src/packer/output.rs` to see what exists.

**Step 2: Write test for XML output**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_single_file() {
        let mut packer = PackOutput::new("test-project");
        packer.add_file("src/main.rs", "fn main() {}");

        let output = packer.to_xml();
        assert!(output.contains("<file path=\"src/main.rs\">"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("</file>"));
    }
}
```

**Step 3: Implement PackOutput**

```rust
use std::collections::HashMap;

pub struct PackOutput {
    project_name: String,
    files: HashMap<String, String>,
}

impl PackOutput {
    pub fn new(project_name: impl Into<String>) -> Self {
        Self {
            project_name: project_name.into(),
            files: HashMap::new(),
        }
    }

    pub fn add_file(&mut self, path: impl Into<String>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    pub fn to_xml(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("<repository name=\"{}\">\n", self.project_name));

        let mut paths: Vec<_> = self.files.keys().collect();
        paths.sort();

        for path in paths {
            let content = &self.files[path];
            output.push_str(&format!("<file path=\"{}\">\n", path));
            output.push_str(content);
            if !content.ends_with('\n') {
                output.push('\n');
            }
            output.push_str("</file>\n");
        }

        output.push_str("</repository>\n");
        output
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_single_file() {
        let mut packer = PackOutput::new("test-project");
        packer.add_file("src/main.rs", "fn main() {}");

        let output = packer.to_xml();
        assert!(output.contains("<file path=\"src/main.rs\">"));
        assert!(output.contains("fn main() {}"));
        assert!(output.contains("</file>"));
    }

    #[test]
    fn test_pack_multiple_files_sorted() {
        let mut packer = PackOutput::new("test");
        packer.add_file("z.rs", "z");
        packer.add_file("a.rs", "a");

        let output = packer.to_xml();
        let a_pos = output.find("a.rs").unwrap();
        let z_pos = output.find("z.rs").unwrap();
        assert!(a_pos < z_pos);
    }
}
```

**Step 4: Run tests**

Run: `cargo test packer::output::tests` Expected: PASS

**Step 5: Commit**

```bash
git add src/packer/output.rs
git commit -m "feat: implement XML pack output format"
```

---

### Task 8: Create packer orchestration

**Files:**

- Modify: `src/packer/mod.rs`

**Step 1: Write test for full pack workflow**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_directory() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("lib.rs"), "pub fn hello() {}").unwrap();

        let pack = pack_directory(tmp.path(), &PackOptions::default()).unwrap();

        assert_eq!(pack.file_count(), 2);
        let xml = pack.to_xml();
        assert!(xml.contains("main.rs"));
        assert!(xml.contains("lib.rs"));
    }
}
```

**Step 2: Implement pack_directory function**

Add to `src/packer/mod.rs`:

```rust
pub mod compress;
pub mod git;
pub mod gitignore;
pub mod output;
pub mod walker;

pub use output::PackOutput;
pub use walker::FileWalker;

use crate::utils::error::RuleyError;
use std::fs;
use std::path::Path;

#[derive(Default)]
pub struct PackOptions {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub compress: bool,
}

pub fn pack_directory(path: &Path, options: &PackOptions) -> Result<PackOutput, RuleyError> {
    let project_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    let mut walker = FileWalker::new(path);

    if !options.include_patterns.is_empty() {
        walker = walker.with_includes(options.include_patterns.clone());
    }
    if !options.exclude_patterns.is_empty() {
        walker = walker.with_excludes(options.exclude_patterns.clone());
    }

    let files = walker.walk()?;
    let mut pack = PackOutput::new(project_name);

    for file_path in files {
        let content = fs::read_to_string(&file_path).map_err(RuleyError::FileSystem)?;

        let relative = file_path
            .strip_prefix(path)
            .unwrap_or(&file_path)
            .to_string_lossy()
            .to_string();

        pack.add_file(relative, content);
    }

    Ok(pack)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_pack_directory() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("lib.rs"), "pub fn hello() {}").unwrap();

        let pack = pack_directory(tmp.path(), &PackOptions::default()).unwrap();

        assert_eq!(pack.file_count(), 2);
        let xml = pack.to_xml();
        assert!(xml.contains("main.rs"));
        assert!(xml.contains("lib.rs"));
    }
}
```

**Step 3: Run tests**

Run: `cargo test packer::tests` Expected: PASS

**Step 4: Commit**

```bash
git add src/packer/mod.rs
git commit -m "feat: add pack_directory orchestration function"
```

---

## Phase 3: Rule Generation

### Task 9: Define prompt templates

**Files:**

- Modify: `src/generator/prompts.rs`

**Step 1: Read existing prompts**

Check `prompts/` directory and `src/generator/prompts.rs`.

**Step 2: Implement prompt loading**

```rust
use std::collections::HashMap;

const BASE_PROMPT: &str = include_str!("../../prompts/base.md");

pub struct PromptTemplates {
    templates: HashMap<String, String>,
}

impl PromptTemplates {
    pub fn new() -> Self {
        let mut templates = HashMap::new();
        templates.insert("base".into(), BASE_PROMPT.into());
        Self { templates }
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.templates.get(name).map(|s| s.as_str())
    }

    pub fn build_analysis_prompt(&self, codebase: &str) -> String {
        let base = self.get("base").unwrap_or("");
        format!("{}\n\n<codebase>\n{}\n</codebase>", base, codebase)
    }
}

impl Default for PromptTemplates {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_prompt_loaded() {
        let prompts = PromptTemplates::new();
        let base = prompts.get("base");
        assert!(base.is_some());
        assert!(!base.unwrap().is_empty());
    }

    #[test]
    fn test_build_analysis_prompt() {
        let prompts = PromptTemplates::new();
        let prompt = prompts.build_analysis_prompt("<file>test</file>");
        assert!(prompt.contains("<codebase>"));
        assert!(prompt.contains("<file>test</file>"));
    }
}
```

**Step 3: Run tests**

Run: `cargo test generator::prompts::tests` Expected: PASS

**Step 4: Commit**

```bash
git add src/generator/prompts.rs
git commit -m "feat: implement prompt template loading"
```

---

### Task 10: Implement rule generation from LLM

**Files:**

- Modify: `src/generator/rules.rs`
- Modify: `src/generator/mod.rs`

**Step 1: Write test for JSON parsing**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rules_from_json() {
        let json = r#"{
            "project": {"name": "test", "description": "A test project"},
            "tech_stack": {"language": "Rust", "framework": null, "build_tool": "cargo"},
            "conventions": [],
            "key_files": [],
            "architecture": {"description": "Simple CLI"},
            "tasks": [],
            "antipatterns": [],
            "examples": []
        }"#;

        let rules: GeneratedRules = serde_json::from_str(json).unwrap();
        assert_eq!(rules.project.name, "test");
    }
}
```

**Step 2: Run test**

Run: `cargo test generator::rules::tests::test_parse_rules_from_json` Expected: PASS (serde derive already exists)

**Step 3: Implement parse function**

Add to `src/generator/rules.rs`:

````rust
pub fn parse_rules_from_response(response: &str) -> Result<GeneratedRules, RuleyError> {
    // Try to extract JSON from markdown code blocks
    let json_str = if response.contains("```json") {
        response
            .split("```json")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
    } else if response.contains("```") {
        response
            .split("```")
            .nth(1)
            .and_then(|s| s.split("```").next())
            .unwrap_or(response)
    } else {
        response
    };

    serde_json::from_str(json_str.trim()).map_err(|e| {
        RuleyError::OutputFormat(format!("Failed to parse LLM response as JSON: {}", e))
    })
}
````

**Step 4: Add test for extraction from markdown**

````rust
    #[test]
    fn test_parse_rules_from_markdown() {
        let response = r#"Here are the rules:

```json
{
    "project": {"name": "test", "description": "A test"},
    "tech_stack": {"language": "Rust", "framework": null, "build_tool": null},
    "conventions": [],
    "key_files": [],
    "architecture": {"description": "Simple"},
    "tasks": [],
    "antipatterns": [],
    "examples": []
}
````

That's all!"#;

```
    let rules = parse_rules_from_response(response).unwrap();
    assert_eq!(rules.project.name, "test");
}
```

````

**Step 5: Run all generator tests**

Run: `cargo test generator::rules::tests`
Expected: PASS

**Step 6: Commit**

```bash
git add src/generator/rules.rs
git commit -m "feat: implement LLM response parsing for rules"
````

---

## Phase 4: Output Formatters

### Task 11: Implement Cursor formatter

**Files:**

- Modify: `src/output/cursor.rs`

**Step 1: Write test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    fn sample_rules() -> GeneratedRules {
        GeneratedRules {
            project: ProjectInfo {
                name: "test-project".into(),
                description: "A test project".into(),
            },
            tech_stack: TechStack {
                language: Some("Rust".into()),
                framework: None,
                build_tool: Some("cargo".into()),
            },
            conventions: vec![Convention {
                category: "naming".into(),
                rule: "Use snake_case for functions".into(),
                rationale: None,
                examples: vec![],
            }],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: "Simple CLI tool".into(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        }
    }

    #[test]
    fn test_cursor_format_has_frontmatter() {
        let formatter = CursorFormatter;
        let metadata = Metadata::default();
        let output = formatter.format(&sample_rules(), &metadata).unwrap();

        assert!(output.starts_with("---"));
        assert!(output.contains("description:"));
        assert!(output.contains("globs:"));
    }
}
```

**Step 2: Run test to see it fail**

Run: `cargo test output::cursor::tests` Expected: FAIL - todo!()

**Step 3: Implement formatter**

```rust
use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct CursorFormatter;

impl OutputFormatter for CursorFormatter {
    fn format(&self, rules: &GeneratedRules, metadata: &Metadata) -> Result<String, RuleyError> {
        let mut output = String::new();

        // Frontmatter
        output.push_str("---\n");
        output.push_str(&format!(
            "description: AI rules for {}\n",
            rules.project.name
        ));
        output.push_str(&format!(
            "globs: {}\n",
            metadata.globs.as_deref().unwrap_or("**/*")
        ));
        output.push_str(&format!(
            "alwaysApply: {}\n",
            metadata.always_apply.unwrap_or(false)
        ));
        output.push_str("---\n\n");

        // Title
        output.push_str(&format!("# {}\n\n", rules.project.name));

        // Description
        output.push_str(&format!("{}\n\n", rules.project.description));

        // Tech Stack
        if rules.tech_stack.language.is_some() || rules.tech_stack.framework.is_some() {
            output.push_str("## Tech Stack\n\n");
            if let Some(ref lang) = rules.tech_stack.language {
                output.push_str(&format!("- **Language:** {}\n", lang));
            }
            if let Some(ref framework) = rules.tech_stack.framework {
                output.push_str(&format!("- **Framework:** {}\n", framework));
            }
            if let Some(ref build) = rules.tech_stack.build_tool {
                output.push_str(&format!("- **Build Tool:** {}\n", build));
            }
            output.push('\n');
        }

        // Architecture
        if !rules.architecture.description.is_empty() {
            output.push_str("## Architecture\n\n");
            output.push_str(&format!("{}\n\n", rules.architecture.description));
        }

        // Conventions
        if !rules.conventions.is_empty() {
            output.push_str("## Conventions\n\n");
            for conv in &rules.conventions {
                output.push_str(&format!("### {}\n\n", conv.category));
                output.push_str(&format!("- {}\n", conv.rule));
                if let Some(ref rationale) = conv.rationale {
                    output.push_str(&format!("  - *Rationale:* {}\n", rationale));
                }
                output.push('\n');
            }
        }

        // Antipatterns
        if !rules.antipatterns.is_empty() {
            output.push_str("## Antipatterns\n\n");
            for anti in &rules.antipatterns {
                output.push_str(&format!("- {}\n", anti.description));
            }
            output.push('\n');
        }

        Ok(output)
    }

    fn extension(&self) -> &str {
        "mdc"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    fn sample_rules() -> GeneratedRules {
        GeneratedRules {
            project: ProjectInfo {
                name: "test-project".into(),
                description: "A test project".into(),
            },
            tech_stack: TechStack {
                language: Some("Rust".into()),
                framework: None,
                build_tool: Some("cargo".into()),
            },
            conventions: vec![Convention {
                category: "naming".into(),
                rule: "Use snake_case for functions".into(),
                rationale: None,
                examples: vec![],
            }],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: "Simple CLI tool".into(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        }
    }

    #[test]
    fn test_cursor_format_has_frontmatter() {
        let formatter = CursorFormatter;
        let metadata = Metadata::default();
        let output = formatter.format(&sample_rules(), &metadata).unwrap();

        assert!(output.starts_with("---"));
        assert!(output.contains("description:"));
        assert!(output.contains("globs:"));
    }

    #[test]
    fn test_cursor_format_has_conventions() {
        let formatter = CursorFormatter;
        let metadata = Metadata::default();
        let output = formatter.format(&sample_rules(), &metadata).unwrap();

        assert!(output.contains("## Conventions"));
        assert!(output.contains("snake_case"));
    }
}
```

**Step 4: Update output/mod.rs Metadata**

Ensure `Metadata` has needed fields:

```rust
#[derive(Default)]
pub struct Metadata {
    pub globs: Option<String>,
    pub always_apply: Option<bool>,
}
```

**Step 5: Run tests**

Run: `cargo test output::cursor::tests` Expected: PASS

**Step 6: Commit**

```bash
git add src/output/cursor.rs src/output/mod.rs
git commit -m "feat: implement Cursor .mdc output formatter"
```

---

### Task 12: Implement Claude formatter

**Files:**

- Modify: `src/output/claude.rs`

**Step 1: Write test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    #[test]
    fn test_claude_format() {
        let formatter = ClaudeFormatter;
        let rules = GeneratedRules {
            project: ProjectInfo {
                name: "my-app".into(),
                description: "My application".into(),
            },
            tech_stack: TechStack {
                language: Some("TypeScript".into()),
                framework: Some("React".into()),
                build_tool: Some("Vite".into()),
            },
            conventions: vec![],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: "Frontend SPA".into(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        };

        let output = formatter.format(&rules, &Metadata::default()).unwrap();

        assert!(output.contains("# Project: my-app"));
        assert!(output.contains("## Tech Stack"));
        assert!(output.contains("TypeScript"));
    }
}
```

**Step 2: Implement formatter**

```rust
use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct ClaudeFormatter;

impl OutputFormatter for ClaudeFormatter {
    fn format(&self, rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        let mut output = String::new();

        output.push_str(&format!("# Project: {}\n\n", rules.project.name));

        output.push_str("## Overview\n\n");
        output.push_str(&format!("{}\n\n", rules.project.description));

        // Tech Stack
        output.push_str("## Tech Stack\n\n");
        if let Some(ref lang) = rules.tech_stack.language {
            output.push_str(&format!("- Language: {}\n", lang));
        }
        if let Some(ref framework) = rules.tech_stack.framework {
            output.push_str(&format!("- Framework: {}\n", framework));
        }
        if let Some(ref build) = rules.tech_stack.build_tool {
            output.push_str(&format!("- Build Tool: {}\n", build));
        }
        output.push('\n');

        // Architecture
        output.push_str("## Architecture\n\n");
        output.push_str(&format!("{}\n\n", rules.architecture.description));

        // Conventions
        if !rules.conventions.is_empty() {
            output.push_str("## Conventions\n\n");
            let mut current_category = String::new();
            for conv in &rules.conventions {
                if conv.category != current_category {
                    output.push_str(&format!("### {}\n\n", conv.category));
                    current_category = conv.category.clone();
                }
                output.push_str(&format!("- {}\n", conv.rule));
            }
            output.push('\n');
        }

        // Key Files
        if !rules.key_files.is_empty() {
            output.push_str("## Files to Know\n\n");
            for file in &rules.key_files {
                output.push_str(&format!("- `{}` - {}\n", file.path, file.description));
            }
            output.push('\n');
        }

        // Tasks
        if !rules.tasks.is_empty() {
            output.push_str("## Common Tasks\n\n");
            for task in &rules.tasks {
                output.push_str(&format!("### {}\n\n", task.name));
                for (i, step) in task.steps.iter().enumerate() {
                    output.push_str(&format!("{}. {}\n", i + 1, step));
                }
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn extension(&self) -> &str {
        "md"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    #[test]
    fn test_claude_format() {
        let formatter = ClaudeFormatter;
        let rules = GeneratedRules {
            project: ProjectInfo {
                name: "my-app".into(),
                description: "My application".into(),
            },
            tech_stack: TechStack {
                language: Some("TypeScript".into()),
                framework: Some("React".into()),
                build_tool: Some("Vite".into()),
            },
            conventions: vec![],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: "Frontend SPA".into(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        };

        let output = formatter.format(&rules, &Metadata::default()).unwrap();

        assert!(output.contains("# Project: my-app"));
        assert!(output.contains("## Tech Stack"));
        assert!(output.contains("TypeScript"));
    }
}
```

**Step 3: Run tests**

Run: `cargo test output::claude::tests` Expected: PASS

**Step 4: Commit**

```bash
git add src/output/claude.rs
git commit -m "feat: implement Claude CLAUDE.md output formatter"
```

---

### Task 13: Implement JSON formatter

**Files:**

- Modify: `src/output/json.rs`

**Step 1: Write test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    #[test]
    fn test_json_format_valid() {
        let formatter = JsonFormatter;
        let rules = GeneratedRules {
            project: ProjectInfo {
                name: "test".into(),
                description: "Test".into(),
            },
            tech_stack: TechStack::default(),
            conventions: vec![],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: String::new(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        };

        let output = formatter.format(&rules, &Metadata::default()).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["project"]["name"], "test");
    }
}
```

**Step 2: Implement formatter**

```rust
use crate::generator::rules::GeneratedRules;
use crate::output::{Metadata, OutputFormatter};
use crate::utils::error::RuleyError;

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
    fn format(&self, rules: &GeneratedRules, _metadata: &Metadata) -> Result<String, RuleyError> {
        serde_json::to_string_pretty(rules)
            .map_err(|e| RuleyError::OutputFormat(format!("Failed to serialize to JSON: {}", e)))
    }

    fn extension(&self) -> &str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::rules::*;

    #[test]
    fn test_json_format_valid() {
        let formatter = JsonFormatter;
        let rules = GeneratedRules {
            project: ProjectInfo {
                name: "test".into(),
                description: "Test".into(),
            },
            tech_stack: TechStack {
                language: None,
                framework: None,
                build_tool: None,
            },
            conventions: vec![],
            key_files: vec![],
            architecture: ArchitectureInfo {
                description: String::new(),
            },
            tasks: vec![],
            antipatterns: vec![],
            examples: vec![],
        };

        let output = formatter.format(&rules, &Metadata::default()).unwrap();

        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["project"]["name"], "test");
    }
}
```

**Step 3: Run tests**

Run: `cargo test output::json::tests` Expected: PASS

**Step 4: Commit**

```bash
git add src/output/json.rs
git commit -m "feat: implement JSON output formatter"
```

---

## Phase 5: Main Orchestrator

### Task 14: Implement the run orchestrator

**Files:**

- Modify: `src/lib.rs`

**Step 1: Update lib.rs with full orchestration**

```rust
pub mod cli;
pub mod generator;
pub mod llm;
pub mod output;
pub mod packer;
pub mod utils;

use anyhow::Result;
use std::path::Path;

use cli::args::{Args, OutputFormat, Provider};
use generator::prompts::PromptTemplates;
use generator::rules::parse_rules_from_response;
use llm::provider::{CompletionOptions, LLMProvider, Message, Role};
use llm::providers::{AnthropicProvider, OpenAIProvider};
use output::claude::ClaudeFormatter;
use output::cursor::CursorFormatter;
use output::json::JsonFormatter;
use output::{Metadata, OutputFormatter};
use packer::{PackOptions, pack_directory};
use utils::error::RuleyError;

pub async fn run() -> Result<()> {
    let args = cli::args::parse();
    let _config = cli::config::load(&args)?;

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env().add_directive(match args.verbose {
                0 => tracing::Level::WARN.into(),
                1 => tracing::Level::INFO.into(),
                2 => tracing::Level::DEBUG.into(),
                _ => tracing::Level::TRACE.into(),
            }),
        )
        .init();

    tracing::info!("ruley v{}", env!("CARGO_PKG_VERSION"));

    // Determine path
    let path = Path::new(&args.path);
    if !path.exists() {
        return Err(RuleyError::Config(format!("Path does not exist: {}", args.path)).into());
    }

    // Dry run mode
    if args.dry_run {
        let pack_options = PackOptions {
            include_patterns: args.include.clone(),
            exclude_patterns: args.exclude.clone(),
            compress: args.compress,
        };
        let pack = pack_directory(path, &pack_options)?;
        println!("Dry run - would process {} files", pack.file_count());
        return Ok(());
    }

    // Pack the codebase
    tracing::info!("Packing codebase from {}", args.path);
    let pack_options = PackOptions {
        include_patterns: args.include.clone(),
        exclude_patterns: args.exclude.clone(),
        compress: args.compress,
    };
    let pack = pack_directory(path, &pack_options)?;
    let codebase_xml = pack.to_xml();

    tracing::info!("Packed {} files", pack.file_count());

    // Build prompt
    let prompts = PromptTemplates::new();
    let prompt = prompts.build_analysis_prompt(&codebase_xml);

    // Create LLM provider
    let provider: Box<dyn LLMProvider> = match args.provider {
        Provider::Anthropic => Box::new(AnthropicProvider::from_env()?),
        Provider::Openai => Box::new(OpenAIProvider::from_env()?),
        _ => {
            return Err(RuleyError::Config(format!(
                "Provider {:?} not yet implemented",
                args.provider
            ))
            .into());
        }
    };

    tracing::info!(
        "Using {} with model {}",
        format!("{:?}", args.provider),
        provider.model()
    );

    // Call LLM
    let messages = vec![Message {
        role: Role::User,
        content: prompt,
    }];

    let options = CompletionOptions {
        max_tokens: Some(8192),
        ..Default::default()
    };

    tracing::info!("Calling LLM...");
    let response = provider.complete(&messages, &options).await?;

    tracing::info!(
        "LLM response: {} input tokens, {} output tokens",
        response.input_tokens,
        response.output_tokens
    );

    // Parse rules
    let rules = parse_rules_from_response(&response.content)?;

    // Format output
    let metadata = Metadata {
        globs: None,
        always_apply: Some(false),
    };

    let formatter: Box<dyn OutputFormatter> = match args.format {
        OutputFormat::Cursor => Box::new(CursorFormatter),
        OutputFormat::Claude => Box::new(ClaudeFormatter),
        OutputFormat::Json => Box::new(JsonFormatter),
        _ => {
            return Err(RuleyError::Config(format!(
                "Format {:?} not yet implemented",
                args.format
            ))
            .into());
        }
    };

    let output = formatter.format(&rules, &metadata)?;

    // Determine output path
    let output_path = args
        .output
        .unwrap_or_else(|| format!("{}.rules.{}", rules.project.name, formatter.extension()));

    std::fs::write(&output_path, &output)?;
    println!("Generated rules written to: {}", output_path);

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build` Expected: Success (may need to fix imports)

**Step 3: Test with dry run**

Run: `cargo run -- --dry-run` Expected: Shows file count

**Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "feat: implement main orchestrator pipeline"
```

---

## Phase 6: Polish & Remaining Formatters

### Task 15: Implement remaining formatters

Implement `copilot.rs`, `windsurf.rs`, `aider.rs`, `generic.rs` following the same pattern as Tasks 11-13.

### Task 16: Add progress display

Use `indicatif` crate for progress bars during packing and LLM calls.

### Task 17: Add cost confirmation

Before LLM call, estimate cost using token count and pricing, prompt user for confirmation unless `--no-confirm`.

### Task 18: Add retry logic

Implement exponential backoff for rate limit and transient errors.

### Task 19: Integration tests

Create full integration tests that mock LLM responses.

### Task 20: CI/CD setup

Add GitHub Actions for testing, linting, and release builds.

---

## Execution Summary

This plan completes the ruley implementation in 20 tasks across 6 phases:

1. **Phase 1** (Tasks 1-6): Testing infrastructure, LLM providers
2. **Phase 2** (Tasks 7-8): Packer enhancements
3. **Phase 3** (Tasks 9-10): Rule generation
4. **Phase 4** (Tasks 11-13): Output formatters
5. **Phase 5** (Task 14): Main orchestrator
6. **Phase 6** (Tasks 15-20): Polish and CI/CD

Each task follows TDD with bite-sized steps (write test, verify fail, implement, verify pass, commit).

---

**Plan complete and saved to `docs/plans/2026-01-10-ruley-completion.md`. Two execution options:**

**1. Subagent-Driven (this session)** - I dispatch fresh subagent per task, review between tasks, fast iteration

**2. Parallel Session (separate)** - Open new session with executing-plans, batch execution with checkpoints

**Which approach?**
