//! Unit tests for repomix parsing.
//!
//! Tests parsing of markdown, XML, and JSON repomix formats.
//! Verifies correct file extraction, error handling, and fallback behavior.

use ruley::packer::repomix::{RepomixFormat, detect_format, parse_repomix};
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Helper to create temp directory for tests.
fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Helper to write file content to temp directory.
async fn write_test_file(dir: &TempDir, filename: &str, content: &str) -> PathBuf {
    let path = dir.path().join(filename);
    fs::write(&path, content)
        .await
        .expect("Failed to write test file");
    path
}

#[tokio::test]
async fn test_parse_repomix_markdown_valid() {
    let temp_dir = create_temp_dir();
    let content = r#"# Repository Overview

## File: src/main.rs
```rust
fn main() {
    println!("Hello, world!");
}
```

## File: src/lib.rs
```rust
pub fn greet() -> &'static str {
    "Hello!"
}
```"#;

    let path = write_test_file(&temp_dir, "repomix.md", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should parse valid markdown");

    assert_eq!(codebase.files.len(), 2, "Should extract 2 files");
    assert_eq!(
        codebase.metadata.total_files, 2,
        "Metadata should reflect 2 files"
    );

    // Verify file paths are correct
    let paths: Vec<String> = codebase
        .files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();
    assert!(
        paths.contains(&"src/main.rs".to_string()),
        "main.rs should be parsed"
    );
    assert!(
        paths.contains(&"src/lib.rs".to_string()),
        "lib.rs should be parsed"
    );
}

#[tokio::test]
async fn test_parse_repomix_markdown_multiple_files() {
    let temp_dir = create_temp_dir();
    let content = r#"## File: src/module1.ts
```typescript
export const config = { debug: true };
```

## File: src/module2.ts
```typescript
export const version = "1.0.0";
```

## File: src/utils.ts
```typescript
export function sum(a: number, b: number): number { return a + b; }
```

## File: README.md
```markdown
# My Project
A simple project.
```"#;

    let path = write_test_file(&temp_dir, "repomix.md", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should parse multiple files");

    assert_eq!(codebase.files.len(), 4, "Should extract 4 files");
    assert_eq!(codebase.metadata.total_files, 4);
}

#[tokio::test]
async fn test_parse_repomix_markdown_empty() {
    let temp_dir = create_temp_dir();
    let content = "# Empty Repository\n\nNo files to process.";

    let path = write_test_file(&temp_dir, "repomix.md", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should handle empty markdown gracefully");

    // Should fall back to raw content
    assert_eq!(codebase.files.len(), 1, "Should have fallback file");
    assert!(
        codebase.files[0]
            .original_content
            .contains("Empty Repository")
    );
}

#[tokio::test]
async fn test_parse_repomix_xml_valid() {
    let temp_dir = create_temp_dir();
    let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<repomix>
    <files>
        <file path="src/index.ts">
            <content><![CDATA[export const version = "1.0.0";]]></content>
        </file>
        <file path="src/config.ts">
            <content><![CDATA[export const config = { debug: true };]]></content>
        </file>
    </files>
</repomix>"#;

    let path = write_test_file(&temp_dir, "repomix.xml", content).await;
    let codebase = parse_repomix(&path).await.expect("Should parse valid XML");

    assert_eq!(codebase.files.len(), 2, "Should extract 2 files from XML");
    assert_eq!(codebase.metadata.total_files, 2);

    let paths: Vec<String> = codebase
        .files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();
    assert!(paths.contains(&"src/index.ts".to_string()));
    assert!(paths.contains(&"src/config.ts".to_string()));
}

#[tokio::test]
async fn test_parse_repomix_xml_cdata() {
    let temp_dir = create_temp_dir();
    let content = r#"<?xml version="1.0" encoding="UTF-8"?>
<repomix>
    <files>
        <file path="src/handler.ts">
            <content><![CDATA[
function handler(event: Event): Promise<void> {
    console.log("Processing event");
    return Promise.resolve();
}
]]></content>
        </file>
    </files>
</repomix>"#;

    let path = write_test_file(&temp_dir, "repomix.xml", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should parse XML with CDATA");

    assert_eq!(codebase.files.len(), 1);
    assert!(
        codebase.files[0].original_content.contains("handler"),
        "CDATA content should be extracted"
    );
}

#[tokio::test]
async fn test_parse_repomix_xml_invalid() {
    let temp_dir = create_temp_dir();
    let content = r#"<?xml version="1.0"?>
<repomix>
    <files>
        <file path="src/broken.ts">
            Missing closing tag
    </files>
</repomix>"#;

    let path = write_test_file(&temp_dir, "repomix.xml", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should handle invalid XML gracefully");

    // Should fall back to raw content
    assert_eq!(codebase.files.len(), 1, "Should have fallback file");
}

#[tokio::test]
async fn test_parse_repomix_json_array_format() {
    let temp_dir = create_temp_dir();
    let content = r#"[
    {
        "path": "src/index.ts",
        "content": "export const version = \"1.0.0\";"
    },
    {
        "path": "src/utils.ts",
        "content": "export function sum(a: number, b: number): number { return a + b; }"
    }
]"#;

    let path = write_test_file(&temp_dir, "repomix.json", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should parse JSON array format");

    assert_eq!(
        codebase.files.len(),
        2,
        "Should extract 2 files from JSON array"
    );
    assert_eq!(codebase.metadata.total_files, 2);

    let paths: Vec<String> = codebase
        .files
        .iter()
        .map(|f| f.path.to_string_lossy().to_string())
        .collect();
    assert!(paths.contains(&"src/index.ts".to_string()));
    assert!(paths.contains(&"src/utils.ts".to_string()));
}

#[tokio::test]
async fn test_parse_repomix_json_map_format() {
    let temp_dir = create_temp_dir();
    let content = r#"{
    "files": {
        "src/module1.ts": "export const mod1 = {};",
        "src/module2.ts": "export const mod2 = { value: 42 };",
        "README.md": "Project Documentation"
    }
}"#;

    let path = write_test_file(&temp_dir, "repomix.json", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should parse JSON map format");

    assert_eq!(
        codebase.files.len(),
        3,
        "Should extract 3 files from JSON map"
    );
    assert_eq!(codebase.metadata.total_files, 3);
}

#[tokio::test]
async fn test_parse_repomix_json_invalid() {
    let temp_dir = create_temp_dir();
    let content = r#"{
    "files": {
        "src/broken.ts": "incomplete json"
    ]
}"#;

    let path = write_test_file(&temp_dir, "repomix.json", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should handle invalid JSON gracefully");

    // Should fall back to raw content
    assert_eq!(codebase.files.len(), 1, "Should have fallback file");
}

#[tokio::test]
async fn test_parse_repomix_json_empty() {
    let temp_dir = create_temp_dir();
    let content = "{}";

    let path = write_test_file(&temp_dir, "repomix.json", content).await;
    let codebase = parse_repomix(&path)
        .await
        .expect("Should handle empty JSON gracefully");

    // Should fall back to raw content
    assert_eq!(codebase.files.len(), 1, "Should have fallback file");
}

#[test]
fn test_detect_format_markdown() {
    let path = PathBuf::from("repomix.md");
    let content = "## File: src/main.rs\n```\ncode\n```";

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Markdown);
}

#[test]
fn test_detect_format_xml() {
    let path = PathBuf::from("repomix.xml");
    let content = "<?xml><files><file path=\"src/main.rs\">code</file></files>";

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Xml);
}

#[test]
fn test_detect_format_json() {
    let path = PathBuf::from("repomix.json");
    let content = r#"{"files": {"src/main.rs": "code"}}"#;

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Json);
}

#[test]
fn test_detect_format_by_content_markdown() {
    let path = PathBuf::from("unknown");
    let content = "## File: src/main.rs\n```\ncode\n```";

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Markdown);
}

#[test]
fn test_detect_format_by_content_xml() {
    let path = PathBuf::from("unknown");
    let content = "<files><file>content</file></files>";

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Xml);
}

#[test]
fn test_detect_format_by_content_json() {
    let path = PathBuf::from("unknown");
    let content = r#"{"files": {}}"#;

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Json);
}

#[test]
fn test_detect_format_unknown() {
    let path = PathBuf::from("unknown.txt");
    let content = "Some random content with no recognizable format";

    let format = detect_format(&path, content);
    assert_eq!(format, RepomixFormat::Unknown);
}
