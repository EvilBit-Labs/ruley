// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use tokio::fs;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::utils::error::RuleyError;

use super::compress::Language;
use super::walker::detect_language;
use super::{CompressedCodebase, CompressedFile, CompressionMethod};

/// Regex for parsing markdown repomix format (## File: path followed by code block)
static MARKDOWN_FILE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)##\s*File:\s*(?P<path>[^\n]+)\n```[^\n]*\n(?P<body>.*?)```")
        .expect("Invalid markdown file regex")
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepomixFormat {
    Markdown,
    Xml,
    Json,
    Unknown,
}

pub fn detect_format(path: &Path, content: &str) -> RepomixFormat {
    if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
        match ext {
            "md" | "markdown" => return RepomixFormat::Markdown,
            "xml" => return RepomixFormat::Xml,
            "json" => return RepomixFormat::Json,
            _ => {}
        }
    }

    if content.contains("<files>") {
        return RepomixFormat::Xml;
    }

    if content.contains("\"files\"") {
        return RepomixFormat::Json;
    }

    if content.contains("## File:") {
        return RepomixFormat::Markdown;
    }

    RepomixFormat::Unknown
}

pub async fn parse_repomix(path: &Path) -> Result<CompressedCodebase, RuleyError> {
    let content = fs::read_to_string(path).await.map_err(|e| {
        RuleyError::FileSystem(std::io::Error::new(
            e.kind(),
            format!("Failed to read repomix file {}: {}", path.display(), e),
        ))
    })?;

    let format = detect_format(path, &content);
    tracing::debug!(?format, file = %path.display(), "Detected repomix format");

    let parsed_files = match format {
        RepomixFormat::Markdown => parse_markdown(&content),
        RepomixFormat::Xml => parse_xml(&content),
        RepomixFormat::Json => parse_json(&content),
        RepomixFormat::Unknown => Err(parse_error(
            "Unknown repomix format; falling back to raw content",
            None,
        )),
    };

    let files = match parsed_files {
        Ok(files) if !files.is_empty() => {
            tracing::info!(file_count = files.len(), ?format, "Parsed repomix file");
            files
        }
        Ok(_) => {
            tracing::warn!(?format, file = %path.display(), "Repomix parser returned no files; using raw content");
            vec![fallback_file(path, &content)]
        }
        Err(err) => {
            tracing::warn!(error = %err, ?format, file = %path.display(), "Failed to parse repomix content; using raw content fallback");
            vec![fallback_file(path, &content)]
        }
    };

    Ok(CompressedCodebase::new(files))
}

fn parse_markdown(content: &str) -> Result<Vec<CompressedFile>, RuleyError> {
    let mut files = Vec::new();
    for caps in MARKDOWN_FILE_REGEX.captures_iter(content) {
        let path = caps["path"].trim();
        let body = caps["body"].to_string();

        let compressed = build_compressed_file(PathBuf::from(path), body);
        files.push(compressed);
    }

    if files.is_empty() {
        return Err(parse_error(
            "No files found in repomix markdown format",
            None,
        ));
    }

    Ok(files)
}

fn parse_xml(content: &str) -> Result<Vec<CompressedFile>, RuleyError> {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut files = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_content = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == b"file" => {
                current_path = None;
                current_content.clear();

                for attr in e.attributes() {
                    let attr = attr.map_err(|err| {
                        parse_error(
                            "Failed to read attribute in repomix XML",
                            Some(Box::new(err)),
                        )
                    })?;

                    if attr.key.as_ref() == b"path" {
                        let value = attr.unescape_value().map_err(|err| {
                            parse_error(
                                "Failed to read file path attribute in repomix XML",
                                Some(Box::new(err)),
                            )
                        })?;
                        current_path = Some(value.to_string());
                    }
                }

                if current_path.is_none() {
                    return Err(parse_error(
                        "Missing path attribute in repomix XML <file> element",
                        None,
                    ));
                }
            }
            Ok(Event::Text(e)) => {
                if current_path.is_some() {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    current_content.push_str(&text);
                }
            }
            Ok(Event::CData(e)) => {
                if current_path.is_some() {
                    let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                    current_content.push_str(&text);
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"file" => {
                if let Some(path) = current_path.take() {
                    let compressed =
                        build_compressed_file(PathBuf::from(path), current_content.clone());
                    files.push(compressed);
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(parse_error(
                    "Failed to parse repomix XML format",
                    Some(Box::new(err)),
                ));
            }
            _ => {}
        }

        buf.clear();
    }

    if files.is_empty() {
        return Err(parse_error(
            "No <file> entries found in repomix XML format",
            None,
        ));
    }

    Ok(files)
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RepomixJson {
    Array(Vec<RepomixFileObject>),
    Map {
        files: std::collections::HashMap<String, String>,
    },
}

#[derive(Debug, Deserialize)]
struct RepomixFileObject {
    path: String,
    content: String,
}

fn parse_json(content: &str) -> Result<Vec<CompressedFile>, RuleyError> {
    let parsed: RepomixJson = serde_json::from_str(content)
        .map_err(|err| parse_error("Failed to parse repomix JSON format", Some(Box::new(err))))?;

    let files = match parsed {
        RepomixJson::Array(file_objects) => {
            if file_objects.is_empty() {
                return Err(parse_error("No files found in repomix JSON array", None));
            }
            file_objects
                .into_iter()
                .map(|obj| build_compressed_file(PathBuf::from(obj.path), obj.content))
                .collect()
        }
        RepomixJson::Map { files: file_map } => {
            if file_map.is_empty() {
                return Err(parse_error("No files found in repomix JSON map", None));
            }
            file_map
                .into_iter()
                .map(|(path, body)| build_compressed_file(PathBuf::from(path), body))
                .collect()
        }
    };

    Ok(files)
}

fn build_compressed_file(path: PathBuf, content: String) -> CompressedFile {
    let size = content.len();
    let language: Option<Language> = detect_language(&path);

    CompressedFile {
        path,
        original_content: content.clone(),
        compressed_content: content,
        compression_method: CompressionMethod::None,
        original_size: size,
        compressed_size: size,
        language,
    }
}

fn fallback_file(path: &Path, content: &str) -> CompressedFile {
    let size = content.len();

    CompressedFile {
        path: path.to_path_buf(),
        original_content: content.to_string(),
        compressed_content: content.to_string(),
        compression_method: CompressionMethod::None,
        original_size: size,
        compressed_size: size,
        language: None,
    }
}

fn parse_error(
    message: impl Into<String>,
    source: Option<Box<dyn Error + Send + Sync>>,
) -> RuleyError {
    RuleyError::ParseError {
        message: message.into(),
        source,
    }
}
