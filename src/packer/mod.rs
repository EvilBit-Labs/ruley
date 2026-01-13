pub mod compress;
pub mod git;
pub mod gitignore;
pub mod output;
pub mod walker;

use compress::Language;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// Enumeration of supported compression methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMethod {
    /// Tree-sitter based compression
    TreeSitter,
    /// Whitespace and line-break normalization
    Whitespace,
    /// No compression applied
    None,
}

impl fmt::Display for CompressionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TreeSitter => write!(f, "tree-sitter"),
            Self::Whitespace => write!(f, "whitespace"),
            Self::None => write!(f, "none"),
        }
    }
}

/// Represents a single compressed file in the codebase.
#[derive(Debug, Clone)]
pub struct CompressedFile {
    /// Path to the file relative to repository root
    pub path: PathBuf,
    /// Original uncompressed content
    pub original_content: String,
    /// Compressed content
    pub compressed_content: String,
    /// Method used for compression
    pub compression_method: CompressionMethod,
    /// Size of original content in bytes
    pub original_size: usize,
    /// Size of compressed content in bytes
    pub compressed_size: usize,
    /// Detected programming language
    pub language: Option<Language>,
}

/// Metadata about the entire compressed codebase.
#[derive(Debug, Clone)]
pub struct CodebaseMetadata {
    /// Total number of files processed
    pub total_files: usize,
    /// Total size of original content in bytes
    pub total_original_size: usize,
    /// Total size of compressed content in bytes
    pub total_compressed_size: usize,
    /// File count breakdown by language
    pub languages: HashMap<Language, usize>,
    /// Overall compression ratio (0.0 to 1.0, where lower is better)
    pub compression_ratio: f32,
}

impl CodebaseMetadata {
    /// Calculate compression ratio from sizes.
    pub fn calculate_compression_ratio(original_size: usize, compressed_size: usize) -> f32 {
        if original_size == 0 {
            return 0.0;
        }
        (compressed_size as f32) / (original_size as f32)
    }
}

/// Represents a complete compressed codebase with metadata.
#[derive(Debug, Clone)]
pub struct CompressedCodebase {
    /// All compressed files in the codebase
    pub files: Vec<CompressedFile>,
    /// Metadata about the compression
    pub metadata: CodebaseMetadata,
}

impl CompressedCodebase {
    /// Create a new compressed codebase and auto-calculate metadata.
    pub fn new(files: Vec<CompressedFile>) -> Self {
        let total_files = files.len();
        let total_original_size: usize = files.iter().map(|f| f.original_size).sum();
        let total_compressed_size: usize = files.iter().map(|f| f.compressed_size).sum();

        let mut languages = HashMap::new();
        for file in &files {
            if let Some(lang) = file.language {
                *languages.entry(lang).or_insert(0) += 1;
            }
        }

        let compression_ratio = CodebaseMetadata::calculate_compression_ratio(
            total_original_size,
            total_compressed_size,
        );

        let metadata = CodebaseMetadata {
            total_files,
            total_original_size,
            total_compressed_size,
            languages,
            compression_ratio,
        };

        Self { files, metadata }
    }
}

pub use walker::{FileEntry, scan_files};
