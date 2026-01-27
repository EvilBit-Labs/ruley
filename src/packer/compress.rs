use crate::{MergedConfig, utils::error::RuleyError};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    /// TypeScript with JSX syntax (.tsx files)
    Tsx,
    JavaScript,
    /// JavaScript with JSX syntax (.jsx files)
    Jsx,
    Python,
    Rust,
    Go,
    Java,
    C,
    Cpp,
    Ruby,
    Php,
}

pub trait Compressor {
    fn compress(&self, source: &str, language: Language) -> Result<String, RuleyError>;
    fn compression_ratio(&self) -> f32;
}

/// Regex for normalizing whitespace (single space replacement)
static WHITESPACE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[ \t]+").expect("Invalid whitespace regex"));

/// Whitespace compressor: removes extra whitespace and blank lines
pub struct WhitespaceCompressor;

impl Compressor for WhitespaceCompressor {
    fn compress(&self, source: &str, _language: Language) -> Result<String, RuleyError> {
        let lines: Vec<&str> = source
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect();

        let mut result = String::new();
        for line in lines {
            let compressed_line = WHITESPACE_REGEX.replace_all(line, " ");
            result.push_str(&compressed_line);
            result.push('\n');
        }

        Ok(result)
    }

    fn compression_ratio(&self) -> f32 {
        0.6 // Target ~40% size reduction
    }
}

pub struct TreeSitterCompressor;

#[cfg(feature = "compression-typescript")]
impl TreeSitterCompressor {
    /// Compress TypeScript/TSX source code using tree-sitter
    ///
    /// # Arguments
    /// * `source` - The TypeScript/TSX source code to compress
    /// * `is_tsx` - Whether this is a TSX file (contains JSX syntax). Determined by file extension.
    pub fn compress_typescript(source: &str, is_tsx: bool) -> Result<String, RuleyError> {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        // Use the appropriate grammar based on file extension
        let language = if is_tsx {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        };

        parser
            .set_language(&language)
            .map_err(|_| RuleyError::Compression {
                language: "TypeScript".to_string(),
                message: "Failed to set tree-sitter language".to_string(),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| RuleyError::Compression {
                language: "TypeScript".to_string(),
                message: "Failed to parse TypeScript source code".to_string(),
            })?;

        let root_node = tree.root_node();

        // If the parsed tree contains syntax errors, signal failure so callers
        // can fall back to a simpler compression strategy.
        if root_node.has_error() {
            return Err(RuleyError::Compression {
                language: "TypeScript".to_string(),
                message: "TypeScript source contains syntax errors".to_string(),
            });
        }

        let mut result = String::new();
        let mut cursor = tree.walk();

        extract_typescript_nodes(source, root_node, &mut cursor, &mut result);

        // If we couldn't extract any meaningful nodes, treat this as a
        // compression failure so higher-level callers can fall back.
        if result.trim().is_empty() {
            return Err(RuleyError::Compression {
                language: "TypeScript".to_string(),
                message: "Failed to extract TypeScript structure for compression".to_string(),
            });
        }

        Ok(result)
    }
}

#[cfg(not(feature = "compression-typescript"))]
impl TreeSitterCompressor {
    /// Compression for TypeScript is not available (feature disabled)
    pub fn compress_typescript(source: &str, _is_tsx: bool) -> Result<String, RuleyError> {
        let _ = source;
        Err(RuleyError::Compression {
            language: "TypeScript".to_string(),
            message: "TypeScript compression feature is not enabled. Enable 'compression-typescript' feature.".to_string(),
        })
    }
}

#[cfg(feature = "compression-python")]
impl TreeSitterCompressor {
    /// Compress Python source code using tree-sitter
    pub fn compress_python(source: &str) -> Result<String, RuleyError> {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_python::LANGUAGE.into();

        parser
            .set_language(&language)
            .map_err(|_| RuleyError::Compression {
                language: "Python".to_string(),
                message: "Failed to set tree-sitter language".to_string(),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| RuleyError::Compression {
                language: "Python".to_string(),
                message: "Failed to parse Python source code".to_string(),
            })?;

        let mut result = String::new();
        let mut cursor = tree.walk();
        let root_node = tree.root_node();

        extract_python_nodes(source, root_node, &mut cursor, &mut result);

        Ok(result)
    }
}

#[cfg(not(feature = "compression-python"))]
impl TreeSitterCompressor {
    /// Compression for Python is not available (feature disabled)
    pub fn compress_python(source: &str) -> Result<String, RuleyError> {
        let _ = source;
        Err(RuleyError::Compression {
            language: "Python".to_string(),
            message:
                "Python compression feature is not enabled. Enable 'compression-python' feature."
                    .to_string(),
        })
    }
}

#[cfg(feature = "compression-rust")]
impl TreeSitterCompressor {
    /// Compress Rust source code using tree-sitter
    pub fn compress_rust(source: &str) -> Result<String, RuleyError> {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();

        parser
            .set_language(&language)
            .map_err(|_| RuleyError::Compression {
                language: "Rust".to_string(),
                message: "Failed to set tree-sitter language".to_string(),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| RuleyError::Compression {
                language: "Rust".to_string(),
                message: "Failed to parse Rust source code".to_string(),
            })?;

        let mut result = String::new();
        let mut cursor = tree.walk();
        let root_node = tree.root_node();

        extract_rust_nodes(source, root_node, &mut cursor, &mut result);

        Ok(result)
    }
}

#[cfg(not(feature = "compression-rust"))]
impl TreeSitterCompressor {
    /// Compression for Rust is not available (feature disabled)
    pub fn compress_rust(source: &str) -> Result<String, RuleyError> {
        let _ = source;
        Err(RuleyError::Compression {
            language: "Rust".to_string(),
            message: "Rust compression feature is not enabled. Enable 'compression-rust' feature."
                .to_string(),
        })
    }
}

#[cfg(feature = "compression-go")]
impl TreeSitterCompressor {
    /// Compress Go source code using tree-sitter
    pub fn compress_go(source: &str) -> Result<String, RuleyError> {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        let language: tree_sitter::Language = tree_sitter_go::LANGUAGE.into();

        parser
            .set_language(&language)
            .map_err(|_| RuleyError::Compression {
                language: "Go".to_string(),
                message: "Failed to set tree-sitter language".to_string(),
            })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| RuleyError::Compression {
                language: "Go".to_string(),
                message: "Failed to parse Go source code".to_string(),
            })?;

        let mut result = String::new();
        let mut cursor = tree.walk();
        let root_node = tree.root_node();

        extract_go_nodes(source, root_node, &mut cursor, &mut result);

        Ok(result)
    }
}

#[cfg(not(feature = "compression-go"))]
impl TreeSitterCompressor {
    /// Compression for Go is not available (feature disabled)
    pub fn compress_go(source: &str) -> Result<String, RuleyError> {
        let _ = source;
        Err(RuleyError::Compression {
            language: "Go".to_string(),
            message: "Go compression feature is not enabled. Enable 'compression-go' feature."
                .to_string(),
        })
    }
}

impl Compressor for TreeSitterCompressor {
    fn compress(&self, source: &str, language: Language) -> Result<String, RuleyError> {
        match language {
            Language::TypeScript => Self::compress_typescript(source, false),
            Language::Tsx => Self::compress_typescript(source, true),
            Language::JavaScript | Language::Jsx => {
                // JavaScript/JSX uses the same tree-sitter grammar as TypeScript/TSX
                // JSX syntax is handled by TSX grammar
                let is_jsx = matches!(language, Language::Jsx);
                Self::compress_typescript(source, is_jsx)
            }
            Language::Python => Self::compress_python(source),
            Language::Rust => Self::compress_rust(source),
            Language::Go => Self::compress_go(source),
            _ => Err(RuleyError::Compression {
                language: format!("{:?}", language),
                message: "Tree-sitter compression not available for this language".to_string(),
            }),
        }
    }

    fn compression_ratio(&self) -> f32 {
        0.7 // Target ~70% token reduction
    }
}

// Helper functions for node extraction (feature-gated implementations)

#[cfg(feature = "compression-typescript")]
fn extract_typescript_nodes(
    source: &str,
    node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
    result: &mut String,
) {
    let kind = node.kind();

    match kind {
        "function_declaration" => {
            // Extract signature: from start to just before body
            // Try by field name first, then search for statement_block child
            let mut body = node.child_by_field_name("body");
            if body.is_none() {
                let child_count: u32 = node.child_count().try_into().unwrap_or(0);
                let mut i = 0u32;
                while i < child_count {
                    if let Some(child) = node.child(i) {
                        let kind = child.kind();
                        if kind == "statement_block" || kind == "block" {
                            body = Some(child);
                            break;
                        }
                    }
                    i += 1;
                }
            }

            if let Some(body) = body {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("{ /* ... */ }\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "arrow_function" => {
            // Extract signature: from start to just before body
            let mut body = node.child_by_field_name("body");
            if body.is_none() {
                let child_count: u32 = node.child_count().try_into().unwrap_or(0);
                let mut i = 0u32;
                while i < child_count {
                    if let Some(child) = node.child(i) {
                        let kind = child.kind();
                        if kind == "statement_block" || kind == "block" {
                            body = Some(child);
                            break;
                        }
                    }
                    i += 1;
                }
            }

            if let Some(body) = body {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("=> { /* ... */ }\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "method_definition" => {
            // Extract method signature only
            // Try by field name first, then search for statement_block child
            let mut body = node.child_by_field_name("body");
            if body.is_none() {
                let child_count: u32 = node.child_count().try_into().unwrap_or(0);
                let mut i = 0u32;
                while i < child_count {
                    if let Some(child) = node.child(i) {
                        let kind = child.kind();
                        if kind == "statement_block" || kind == "block" {
                            body = Some(child);
                            break;
                        }
                    }
                    i += 1;
                }
            }

            if let Some(body) = body {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("{ /* ... */ }\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "type_alias_declaration" | "interface_declaration" | "enum_declaration" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "field_definition" | "public_field_definition" => {
            // Extract field declaration, keeping the type but optionally removing initializer
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "class_declaration" => {
            // Extract class header and method signatures only. We avoid using the
            // tree cursor here and instead recurse using the node API directly
            // to keep behaviour stable across tree-sitter versions.
            if let Some(body) = node.child_by_field_name("body") {
                // Class header up to the opening brace
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("{\n");

                // Walk children of the class body to extract method and field
                // signatures. We intentionally drop method bodies to achieve a
                // strong compression ratio while preserving structure.
                let child_count: u32 = body.child_count().try_into().unwrap_or(0);
                let mut body_cursor = body.walk();
                for i in 0..child_count {
                    if let Some(child) = body.child(i) {
                        match child.kind() {
                            "method_definition"
                            | "method_declaration"
                            | "public_field_definition"
                            | "field_definition" => {
                                result.push_str("  ");
                                extract_typescript_nodes(source, child, &mut body_cursor, result);
                            }
                            _ => {
                                // Recurse into nested nodes to pick up any
                                // method-like constructs we care about.
                                extract_typescript_nodes(source, child, &mut body_cursor, result);
                            }
                        }
                    }
                }

                result.push_str("}\n");
            } else {
                // Fallback: if there is no explicit body field, keep the
                // declaration as-is.
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "import_statement" | "export_statement" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "comment" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        _ => {
            // Default: recurse into children using the node API.
            let child_count = node.child_count();
            let mut node_cursor = node.walk();
            for i in 0..child_count {
                if let Some(child) = node.child(i.try_into().unwrap()) {
                    extract_typescript_nodes(source, child, &mut node_cursor, result);
                }
            }
        }
    }
}

#[cfg(not(feature = "compression-typescript"))]
#[allow(dead_code)]
fn extract_typescript_nodes(
    _source: &str,
    _node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
    _result: &mut String,
) {
}

#[cfg(feature = "compression-python")]
fn extract_python_nodes(
    source: &str,
    node: tree_sitter::Node,
    cursor: &mut tree_sitter::TreeCursor,
    result: &mut String,
) {
    let kind = node.kind();

    match kind {
        "function_definition" => {
            // Extract signature: from 'def' to just before body
            if let Some(body) = node.child_by_field_name("body") {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("pass\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "decorated_definition" => {
            // For decorated functions, extract decorators and signature
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32) {
                    if child.kind() == "decorator" {
                        result.push_str(&source[child.start_byte()..child.end_byte()]);
                        result.push('\n');
                    } else if child.kind() == "function_definition" {
                        extract_python_nodes(source, child, cursor, result);
                    }
                }
            }
        }
        "class_definition" => {
            // Extract class header and method signatures only
            if let Some(name) = node.child_by_field_name("name") {
                result.push_str("class ");
                result.push_str(&source[name.start_byte()..name.end_byte()]);

                // Add superclasses if present
                if let Some(superclasses) = node.child_by_field_name("superclasses") {
                    result.push_str(&source[superclasses.start_byte()..superclasses.end_byte()]);
                }

                result.push_str(":\n");

                // Extract method signatures from class body
                if let Some(body) = node.child_by_field_name("body") {
                    let mut body_cursor = body.walk();
                    if body_cursor.goto_first_child() {
                        loop {
                            let child = body_cursor.node();
                            if child.kind() == "function_definition"
                                || child.kind() == "decorated_definition"
                            {
                                result.push_str("    ");
                                extract_python_nodes(source, child, &mut body_cursor, result);
                            }
                            if !body_cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                }
            }
        }
        "import_statement" | "import_from_statement" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "comment" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        _ => {
            if node.child_count() > 0 {
                cursor.goto_first_child();
                loop {
                    extract_python_nodes(source, cursor.node(), cursor, result);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
    }
}

#[cfg(not(feature = "compression-python"))]
#[allow(dead_code)]
fn extract_python_nodes(
    _source: &str,
    _node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
    _result: &mut String,
) {
}

#[cfg(feature = "compression-rust")]
fn extract_rust_nodes(
    source: &str,
    node: tree_sitter::Node,
    cursor: &mut tree_sitter::TreeCursor,
    result: &mut String,
) {
    let kind = node.kind();

    match kind {
        "function_item" => {
            // Extract signature: from start to just before body
            if let Some(body) = node.child_by_field_name("body") {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("{ /* ... */ }\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "struct_item" | "enum_item" | "trait_item" => {
            // Keep full definitions for types/enums/traits
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "impl_item" => {
            // Extract impl header and method signatures only
            // Find the opening brace of the impl block
            let impl_header_end = source[node.start_byte()..node.end_byte()]
                .find('{')
                .map(|pos| node.start_byte() + pos)
                .unwrap_or(node.end_byte());

            result.push_str(&source[node.start_byte()..impl_header_end]);
            result.push_str("{\n");

            // Extract method signatures
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32)
                    && child.kind() == "function_item"
                {
                    result.push_str("    ");
                    extract_rust_nodes(source, child, cursor, result);
                }
            }

            result.push_str("}\n");
        }
        "use_declaration" | "mod_item" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "line_comment" | "block_comment" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        _ => {
            if node.child_count() > 0 {
                cursor.goto_first_child();
                loop {
                    extract_rust_nodes(source, cursor.node(), cursor, result);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
    }
}

#[cfg(not(feature = "compression-rust"))]
#[allow(dead_code)]
fn extract_rust_nodes(
    _source: &str,
    _node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
    _result: &mut String,
) {
}

#[cfg(feature = "compression-go")]
fn extract_go_nodes(
    source: &str,
    node: tree_sitter::Node,
    cursor: &mut tree_sitter::TreeCursor,
    result: &mut String,
) {
    let kind = node.kind();

    match kind {
        "function_declaration" | "method_declaration" => {
            // Extract signature: from start to just before body
            if let Some(body) = node.child_by_field_name("body") {
                result.push_str(&source[node.start_byte()..body.start_byte()]);
                result.push_str("{ /* ... */ }\n");
            } else {
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "type_declaration" => {
            // Check if this is a struct/interface type with methods
            let has_methods = source[node.start_byte()..node.end_byte()].contains("struct")
                || source[node.start_byte()..node.end_byte()].contains("interface");

            if has_methods {
                // For struct types, extract the type declaration
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            } else {
                // For simple type aliases, keep full declaration
                result.push_str(&source[node.start_byte()..node.end_byte()]);
                result.push('\n');
            }
        }
        "import_declaration" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        "comment" => {
            result.push_str(&source[node.start_byte()..node.end_byte()]);
            result.push('\n');
        }
        _ => {
            if node.child_count() > 0 {
                cursor.goto_first_child();
                loop {
                    extract_go_nodes(source, cursor.node(), cursor, result);
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
                cursor.goto_parent();
            }
        }
    }
}

#[cfg(not(feature = "compression-go"))]
#[allow(dead_code)]
fn extract_go_nodes(
    _source: &str,
    _node: tree_sitter::Node,
    _cursor: &mut tree_sitter::TreeCursor,
    _result: &mut String,
) {
}

/// Compress a codebase by processing file entries and applying compression
pub async fn compress_codebase(
    entries: Vec<super::walker::FileEntry>,
    config: &MergedConfig,
) -> Result<super::CompressedCodebase, RuleyError> {
    // If compression is disabled, return uncompressed codebase
    if !config.compress {
        let mut files = Vec::new();
        for entry in entries {
            match tokio::fs::read_to_string(&entry.path).await {
                Ok(original_content) => {
                    let original_size = original_content.len();
                    files.push(super::CompressedFile {
                        path: entry.path,
                        original_content: original_content.clone(),
                        compressed_content: original_content,
                        compression_method: super::CompressionMethod::None,
                        original_size,
                        compressed_size: original_size,
                        language: entry.language,
                    });
                }
                Err(e) => {
                    return Err(RuleyError::FileSystem(std::io::Error::new(
                        e.kind(),
                        format!("Failed to read file {}: {}", entry.path.display(), e),
                    )));
                }
            }
        }
        return Ok(super::CompressedCodebase::new(files));
    }

    let tree_sitter_compressor = TreeSitterCompressor;
    let whitespace_compressor = WhitespaceCompressor;
    let mut compressed_files = Vec::new();

    for entry in entries {
        match tokio::fs::read_to_string(&entry.path).await {
            Ok(original_content) => {
                let original_size = original_content.len();
                let (compressed_content, compression_method) = match entry.language {
                    Some(lang) => match tree_sitter_compressor.compress(&original_content, lang) {
                        Ok(compressed) => (compressed, super::CompressionMethod::TreeSitter),
                        Err(e) => {
                            tracing::warn!(
                                "Tree-sitter parse failed for {}, falling back to whitespace compression: {}",
                                entry.path.display(),
                                e
                            );
                            match whitespace_compressor.compress(&original_content, lang) {
                                Ok(compressed) => {
                                    (compressed, super::CompressionMethod::Whitespace)
                                }
                                Err(fallback_err) => {
                                    tracing::warn!(
                                        "Whitespace compression also failed for {}: {}",
                                        entry.path.display(),
                                        fallback_err
                                    );
                                    (original_content.clone(), super::CompressionMethod::None)
                                }
                            }
                        }
                    },
                    None => match whitespace_compressor.compress(&original_content, Language::Java)
                    {
                        Ok(compressed) => (compressed, super::CompressionMethod::Whitespace),
                        Err(_) => (original_content.clone(), super::CompressionMethod::None),
                    },
                };

                let compressed_size = compressed_content.len();

                compressed_files.push(super::CompressedFile {
                    path: entry.path.clone(),
                    original_content,
                    compressed_content,
                    compression_method,
                    original_size,
                    compressed_size,
                    language: entry.language,
                });
            }
            Err(e) => {
                return Err(RuleyError::FileSystem(std::io::Error::new(
                    e.kind(),
                    format!("Failed to read file {}: {}", entry.path.display(), e),
                )));
            }
        }
    }

    let codebase = super::CompressedCodebase::new(compressed_files);

    // Log compression statistics
    let overall_ratio = codebase.metadata.compression_ratio;
    let size_reduction = if codebase.metadata.total_original_size > 0 {
        (1.0 - overall_ratio) * 100.0
    } else {
        0.0
    };

    tracing::info!(
        "Compressed {} files: {:.1}% size reduction",
        codebase.metadata.total_files,
        size_reduction
    );

    // Log breakdown by compression method
    let tree_sitter_count = codebase
        .files
        .iter()
        .filter(|f| f.compression_method == super::CompressionMethod::TreeSitter)
        .count();
    let whitespace_count = codebase
        .files
        .iter()
        .filter(|f| f.compression_method == super::CompressionMethod::Whitespace)
        .count();
    let none_count = codebase
        .files
        .iter()
        .filter(|f| f.compression_method == super::CompressionMethod::None)
        .count();

    tracing::debug!(
        "Compression breakdown: {} tree-sitter, {} whitespace, {} none",
        tree_sitter_count,
        whitespace_count,
        none_count
    );

    // Log language distribution
    if !codebase.metadata.languages.is_empty() {
        tracing::debug!("Language distribution: {:?}", codebase.metadata.languages);
    }

    Ok(codebase)
}
