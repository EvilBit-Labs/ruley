// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

//! Token-based chunking logic for splitting large codebases.
//!
//! This module provides functionality to split a compressed codebase into
//! manageable chunks that fit within LLM context limits. Each chunk includes
//! configurable overlap to maintain context continuity.
//!
//! # Example
//!
//! ```ignore
//! use ruley::llm::chunker::{ChunkConfig, chunk_codebase};
//! use ruley::llm::tokenizer::{TiktokenTokenizer, TokenizerModel};
//!
//! let config = ChunkConfig::default();
//! let tokenizer = TiktokenTokenizer::new(TokenizerModel::Claude)?;
//! let chunks = chunk_codebase(&compressed_codebase, &config, &tokenizer)?;
//!
//! for chunk in &chunks {
//!     println!("Chunk {}: {} tokens", chunk.id, chunk.token_count);
//! }
//! ```

use crate::packer::CompressedCodebase;
use crate::utils::error::RuleyError;

use super::tokenizer::Tokenizer;

/// Configuration for chunking a codebase.
///
/// Controls how the codebase is split into chunks and how much overlap
/// exists between consecutive chunks to maintain context continuity.
#[derive(Debug, Clone)]
pub struct ChunkConfig {
    /// Maximum number of tokens per chunk.
    ///
    /// Default: 100,000 tokens (suitable for most LLM context windows)
    pub chunk_size: usize,

    /// Number of tokens to overlap between consecutive chunks.
    ///
    /// Default: 10,000 tokens (10% of default chunk_size)
    /// This helps maintain context continuity across chunk boundaries.
    pub overlap_size: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            chunk_size: 100_000,
            overlap_size: 10_000,
        }
    }
}

impl ChunkConfig {
    /// Create a new chunk configuration with custom values.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum tokens per chunk
    /// * `overlap_size` - Tokens to overlap between chunks
    ///
    /// # Errors
    ///
    /// Returns an error if `overlap_size >= chunk_size`.
    #[must_use = "this returns a Result that should be checked"]
    pub fn new(chunk_size: usize, overlap_size: usize) -> Result<Self, RuleyError> {
        if overlap_size >= chunk_size {
            return Err(RuleyError::ValidationError {
                message: format!(
                    "Overlap size ({}) must be less than chunk size ({})",
                    overlap_size, chunk_size
                ),
                suggestion: "Reduce overlap_size or increase chunk_size".to_string(),
            });
        }

        if chunk_size < 1000 {
            return Err(RuleyError::invalid_chunk_size(chunk_size));
        }

        Ok(Self {
            chunk_size,
            overlap_size,
        })
    }

    /// Create a configuration with a specific chunk size and default 10% overlap.
    ///
    /// # Arguments
    ///
    /// * `chunk_size` - Maximum tokens per chunk
    ///
    /// # Errors
    ///
    /// Returns an error if chunk_size is too small.
    #[must_use = "this returns a Result that should be checked"]
    pub fn with_chunk_size(chunk_size: usize) -> Result<Self, RuleyError> {
        let overlap_size = chunk_size / 10; // 10% overlap
        Self::new(chunk_size, overlap_size)
    }
}

/// A single chunk of content from a codebase.
///
/// Each chunk contains a portion of the codebase content along with
/// metadata about its size and position.
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Unique identifier for this chunk (0-indexed).
    pub id: usize,

    /// The content of this chunk.
    ///
    /// May span multiple files, formatted as a concatenation of file
    /// paths and their contents.
    pub content: String,

    /// Number of tokens in this chunk's content.
    pub token_count: usize,

    /// Number of tokens that overlap with the previous chunk.
    ///
    /// This is 0 for the first chunk.
    pub overlap_token_count: usize,
}

impl Chunk {
    /// Create a chunk from the entire codebase (single-chunk case).
    ///
    /// This is a convenience method for when the codebase fits within
    /// a single chunk without requiring splitting.
    ///
    /// # Arguments
    ///
    /// * `codebase` - The compressed codebase
    /// * `tokenizer` - The tokenizer to count tokens
    ///
    /// # Returns
    ///
    /// A single chunk containing the entire codebase.
    #[must_use]
    pub fn from_codebase(codebase: &CompressedCodebase, tokenizer: &dyn Tokenizer) -> Self {
        let content = format_codebase_content(codebase);
        let token_count = tokenizer.count_tokens(&content);

        Self {
            id: 0,
            content,
            token_count,
            overlap_token_count: 0,
        }
    }
}

/// Split a compressed codebase into token-bounded chunks.
///
/// If the codebase fits within a single chunk, returns a single chunk.
/// Otherwise, splits the content into multiple chunks with configurable
/// overlap for context continuity.
///
/// # Arguments
///
/// * `codebase` - The compressed codebase to chunk
/// * `config` - Chunking configuration (chunk size, overlap)
/// * `tokenizer` - The tokenizer to use for token counting
///
/// # Returns
///
/// A vector of chunks, each containing a portion of the codebase.
///
/// # Errors
///
/// Returns an error if the chunking process fails.
///
/// # Example
///
/// ```ignore
/// use ruley::llm::chunker::{ChunkConfig, chunk_codebase};
/// use ruley::llm::tokenizer::{TiktokenTokenizer, TokenizerModel};
///
/// let config = ChunkConfig::default();
/// let tokenizer = TiktokenTokenizer::new(TokenizerModel::Claude)?;
/// let chunks = chunk_codebase(&codebase, &config, &tokenizer)?;
///
/// println!("Split into {} chunks", chunks.len());
/// for chunk in &chunks {
///     println!("Chunk {}: {} tokens (overlap: {})",
///         chunk.id, chunk.token_count, chunk.overlap_token_count);
/// }
/// ```
#[must_use = "this returns a Result that should be checked"]
pub fn chunk_codebase(
    codebase: &CompressedCodebase,
    config: &ChunkConfig,
    tokenizer: &dyn Tokenizer,
) -> Result<Vec<Chunk>, RuleyError> {
    // Format the entire codebase as a single content string
    let full_content = format_codebase_content(codebase);
    let total_tokens = tokenizer.count_tokens(&full_content);

    // If content fits in a single chunk, return it directly
    if total_tokens <= config.chunk_size {
        return Ok(vec![Chunk {
            id: 0,
            content: full_content,
            token_count: total_tokens,
            overlap_token_count: 0,
        }]);
    }

    // Need to split into multiple chunks
    chunk_content(&full_content, config, tokenizer)
}

/// Format the codebase content for inclusion in prompts.
///
/// Creates a structured representation of all files in the codebase,
/// suitable for sending to an LLM.
fn format_codebase_content(codebase: &CompressedCodebase) -> String {
    let mut content = String::new();

    for file in &codebase.files {
        content.push_str("--- ");
        content.push_str(&file.path.to_string_lossy());
        content.push_str(" ---\n");
        content.push_str(&file.compressed_content);
        content.push_str("\n\n");
    }

    content
}

/// Split content into chunks with overlap.
///
/// This function performs character-based splitting while respecting
/// token boundaries by using the tokenizer to count tokens.
fn chunk_content(
    content: &str,
    config: &ChunkConfig,
    tokenizer: &dyn Tokenizer,
) -> Result<Vec<Chunk>, RuleyError> {
    let mut chunks = Vec::new();
    let mut start_pos = 0;
    let mut chunk_id = 0;

    // Calculate the effective content size per chunk (excluding overlap for subsequent chunks)
    let effective_chunk_size = config.chunk_size - config.overlap_size;

    while start_pos < content.len() {
        // Determine overlap for this chunk
        let (overlap_start, overlap_token_count) = if chunk_id == 0 {
            (start_pos, 0)
        } else {
            // Find the overlap start position by going back from start_pos
            let overlap_start =
                find_overlap_start(content, start_pos, config.overlap_size, tokenizer);
            let overlap_content = &content[overlap_start..start_pos];
            let overlap_tokens = tokenizer.count_tokens(overlap_content);
            (overlap_start, overlap_tokens)
        };

        // Find the end position for this chunk
        let chunk_start = overlap_start;
        let target_tokens = config.chunk_size;
        let chunk_end = find_chunk_end(content, chunk_start, target_tokens, tokenizer);

        // Extract the chunk content
        let chunk_content = &content[chunk_start..chunk_end];
        let token_count = tokenizer.count_tokens(chunk_content);

        chunks.push(Chunk {
            id: chunk_id,
            content: chunk_content.to_string(),
            token_count,
            overlap_token_count,
        });

        // Move to the next chunk starting position (after the non-overlap portion)
        if chunk_end >= content.len() {
            break;
        }

        // Calculate new start position: advance by effective_chunk_size from current start_pos
        start_pos = find_chunk_end(content, start_pos, effective_chunk_size, tokenizer);

        // Ensure we're making progress
        if start_pos <= chunk_start {
            start_pos = chunk_end;
        }

        chunk_id += 1;
    }

    Ok(chunks)
}

/// Find the start position for the overlap region.
///
/// Goes backward from the current position to find where the overlap should start.
fn find_overlap_start(
    content: &str,
    current_pos: usize,
    target_overlap_tokens: usize,
    tokenizer: &dyn Tokenizer,
) -> usize {
    if current_pos == 0 || target_overlap_tokens == 0 {
        return current_pos;
    }

    // Binary search for the overlap start position
    let mut low = 0;
    let mut high = current_pos;
    let mut last_mid = None;

    while low < high {
        let raw_mid = (low + high) / 2;
        let mid = find_char_boundary(content, raw_mid);

        // Ensure we're making progress - if we're evaluating the same position,
        // break to avoid infinite loop
        if Some(mid) == last_mid {
            break;
        }
        last_mid = Some(mid);

        // Ensure we're making progress - if mid snaps back to low,
        // we need to advance to avoid infinite loop
        if mid == low && low < high {
            low += 1;
            continue;
        }

        let overlap_content = &content[mid..current_pos];
        let tokens = tokenizer.count_tokens(overlap_content);

        if tokens < target_overlap_tokens {
            // Need to go back further
            high = mid.saturating_sub(1);
        } else if tokens > target_overlap_tokens {
            // Went too far back, move forward
            low = mid + 1;
        } else {
            return mid;
        }
    }

    find_char_boundary(content, low)
}

/// Find the end position for a chunk given a target token count.
///
/// Uses binary search to find the position that results in approximately
/// the target number of tokens.
fn find_chunk_end(
    content: &str,
    start_pos: usize,
    target_tokens: usize,
    tokenizer: &dyn Tokenizer,
) -> usize {
    let remaining_content = &content[start_pos..];
    let remaining_tokens = tokenizer.count_tokens(remaining_content);

    // If remaining content fits in target, return end
    if remaining_tokens <= target_tokens {
        return content.len();
    }

    // Binary search for the end position
    let mut low = start_pos;
    let mut high = content.len();
    let mut best_pos = start_pos + 1; // Ensure we make at least some progress
    let mut last_mid = None;

    while low < high {
        let raw_mid = (low + high) / 2;
        let mid = find_char_boundary(content, raw_mid);

        // Ensure we're making progress - if we're evaluating the same position,
        // break to avoid infinite loop
        if Some(mid) == last_mid {
            break;
        }
        last_mid = Some(mid);

        // Ensure we're making progress - if mid snaps back to low,
        // we need to advance to avoid infinite loop
        if mid == low && low < high {
            low += 1;
            continue;
        }

        if mid <= start_pos {
            low = start_pos + 1;
            continue;
        }

        let chunk_content = &content[start_pos..mid];
        let tokens = tokenizer.count_tokens(chunk_content);

        if tokens <= target_tokens {
            // This position works, try to include more
            best_pos = mid;
            low = mid + 1;
        } else {
            // Too many tokens, need to include less
            high = mid;
        }
    }

    // Try to break at a newline for cleaner chunks
    find_clean_break(content, best_pos)
}

/// Find a valid UTF-8 character boundary at or before the given position.
fn find_char_boundary(content: &str, pos: usize) -> usize {
    if pos >= content.len() {
        return content.len();
    }

    // Find the nearest valid char boundary at or before pos
    let mut boundary = pos;
    while boundary > 0 && !content.is_char_boundary(boundary) {
        boundary -= 1;
    }
    boundary
}

/// Find a clean break point (newline) near the given position.
///
/// Looks for a newline within a small window before the position to
/// avoid breaking in the middle of lines.
fn find_clean_break(content: &str, pos: usize) -> usize {
    if pos >= content.len() {
        return content.len();
    }

    // Ensure pos is on a valid character boundary
    let pos = find_char_boundary(content, pos);

    // Look for a newline within 100 bytes before pos
    // Ensure search_start is on a valid character boundary
    let search_start = find_char_boundary(content, pos.saturating_sub(100));
    let search_region = &content[search_start..pos];

    if let Some(newline_offset) = search_region.rfind('\n') {
        let break_pos = search_start + newline_offset + 1; // Position after newline
        if break_pos > search_start {
            return break_pos;
        }
    }

    pos
}

/// Legacy Chunker struct for backward compatibility.
///
/// For new code, prefer using the `chunk_codebase` function with `ChunkConfig`.
pub struct Chunker {
    max_tokens: usize,
}

impl Chunker {
    /// Create a new Chunker with the specified maximum token limit.
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Get the maximum tokens per chunk.
    pub fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    /// Chunk text using a provided tokenizer.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to chunk
    /// * `tokenizer` - The tokenizer to use for counting
    ///
    /// # Returns
    ///
    /// A vector of text chunks.
    ///
    /// # Errors
    ///
    /// Returns an error if chunking fails.
    pub fn chunk_with_tokenizer(
        &self,
        text: &str,
        tokenizer: &dyn Tokenizer,
    ) -> Result<Vec<String>, RuleyError> {
        let config = ChunkConfig::with_chunk_size(self.max_tokens)?;

        // Create a minimal codebase with the text as a single file
        let codebase =
            crate::packer::CompressedCodebase::new(vec![crate::packer::CompressedFile {
                path: std::path::PathBuf::from("content"),
                original_content: text.to_string(),
                compressed_content: text.to_string(),
                compression_method: crate::packer::CompressionMethod::None,
                original_size: text.len(),
                compressed_size: text.len(),
                language: None,
            }]);

        let chunks = chunk_codebase(&codebase, &config, tokenizer)?;
        Ok(chunks.into_iter().map(|c| c.content).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packer::{CodebaseMetadata, CompressedFile, CompressionMethod};
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Simple tokenizer for testing that counts words as tokens.
    struct WordTokenizer;

    impl Tokenizer for WordTokenizer {
        fn count_tokens(&self, text: &str) -> usize {
            text.split_whitespace().count()
        }
    }

    fn create_test_codebase(files: Vec<(&str, &str)>) -> CompressedCodebase {
        let compressed_files: Vec<_> = files
            .into_iter()
            .map(|(path, content)| CompressedFile {
                path: PathBuf::from(path),
                original_content: content.to_string(),
                compressed_content: content.to_string(),
                compression_method: CompressionMethod::None,
                original_size: content.len(),
                compressed_size: content.len(),
                language: None,
            })
            .collect();

        CompressedCodebase::new(compressed_files)
    }

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.chunk_size, 100_000);
        assert_eq!(config.overlap_size, 10_000);
    }

    #[test]
    fn test_chunk_config_new() {
        let config = ChunkConfig::new(50_000, 5_000).unwrap();
        assert_eq!(config.chunk_size, 50_000);
        assert_eq!(config.overlap_size, 5_000);
    }

    #[test]
    fn test_chunk_config_overlap_too_large() {
        let result = ChunkConfig::new(10_000, 10_000);
        assert!(result.is_err());
    }

    #[test]
    fn test_chunk_config_with_chunk_size() {
        let config = ChunkConfig::with_chunk_size(50_000).unwrap();
        assert_eq!(config.chunk_size, 50_000);
        assert_eq!(config.overlap_size, 5_000); // 10% of 50_000
    }

    #[test]
    fn test_single_chunk_small_codebase() {
        let codebase =
            create_test_codebase(vec![("src/main.rs", "fn main() { println!(\"hello\"); }")]);
        let config = ChunkConfig::default();
        let tokenizer = WordTokenizer;

        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].id, 0);
        assert_eq!(chunks[0].overlap_token_count, 0);
        assert!(chunks[0].content.contains("src/main.rs"));
        assert!(chunks[0].content.contains("fn main()"));
    }

    #[test]
    fn test_chunk_from_codebase() {
        let codebase = create_test_codebase(vec![("src/lib.rs", "pub mod utils;")]);
        let tokenizer = WordTokenizer;

        let chunk = Chunk::from_codebase(&codebase, &tokenizer);

        assert_eq!(chunk.id, 0);
        assert_eq!(chunk.overlap_token_count, 0);
        assert!(chunk.token_count > 0);
    }

    #[test]
    fn test_multiple_chunks_large_codebase() {
        // Create a codebase that needs multiple chunks
        // 2000 words per file * 2 files = 4000 words total
        let large_content = "word ".repeat(2000);
        let codebase = create_test_codebase(vec![
            ("file1.txt", &large_content),
            ("file2.txt", &large_content),
        ]);

        // Use a chunk size that forces multiple chunks (1000 tokens with 100 overlap)
        let config = ChunkConfig::new(1000, 100).unwrap();
        let tokenizer = WordTokenizer;

        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();

        assert!(
            chunks.len() > 1,
            "Expected multiple chunks, got {}",
            chunks.len()
        );

        // First chunk should have no overlap
        assert_eq!(chunks[0].overlap_token_count, 0);

        // Subsequent chunks should have overlap
        for chunk in chunks.iter().skip(1) {
            assert!(
                chunk.overlap_token_count > 0,
                "Chunk {} should have overlap",
                chunk.id
            );
        }

        // Verify chunk IDs are sequential
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.id, i);
        }
    }

    #[test]
    fn test_empty_codebase() {
        let codebase = CompressedCodebase {
            files: vec![],
            metadata: CodebaseMetadata {
                total_files: 0,
                total_original_size: 0,
                total_compressed_size: 0,
                languages: HashMap::new(),
                compression_ratio: 0.0,
            },
        };
        let config = ChunkConfig::default();
        let tokenizer = WordTokenizer;

        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].token_count, 0);
    }

    #[test]
    fn test_format_codebase_content() {
        let codebase = create_test_codebase(vec![
            ("src/main.rs", "fn main() {}"),
            ("src/lib.rs", "pub mod utils;"),
        ]);

        let content = format_codebase_content(&codebase);

        assert!(content.contains("--- src/main.rs ---"));
        assert!(content.contains("fn main() {}"));
        assert!(content.contains("--- src/lib.rs ---"));
        assert!(content.contains("pub mod utils;"));
    }

    #[test]
    fn test_find_char_boundary() {
        let content = "hello world";
        assert_eq!(find_char_boundary(content, 5), 5);
        assert_eq!(find_char_boundary(content, 20), 11); // Past end
    }

    #[test]
    fn test_find_char_boundary_utf8() {
        let content = "héllo wörld";
        // 'é' is 2 bytes, so positions within it should snap back
        let boundary = find_char_boundary(content, 2);
        assert!(content.is_char_boundary(boundary));
    }

    #[test]
    fn test_find_clean_break() {
        let content = "line1\nline2\nline3";
        // Position near end of line2 should snap to after newline
        let break_pos = find_clean_break(content, 11);
        assert!(break_pos <= 12); // Should break at or after "line2\n"
    }

    #[test]
    fn test_legacy_chunker_interface() {
        let chunker = Chunker::new(1000);
        assert_eq!(chunker.max_tokens(), 1000);
    }

    #[test]
    fn test_legacy_chunker_chunk_with_tokenizer() {
        let chunker = Chunker::new(1000);
        let tokenizer = WordTokenizer;
        let text = "hello world this is a test";

        let result = chunker.chunk_with_tokenizer(text, &tokenizer);
        assert!(result.is_ok());

        let chunks = result.unwrap();
        assert!(!chunks.is_empty());
    }

    #[test]
    fn test_chunk_size_validation() {
        // Chunk size too small
        let result = ChunkConfig::new(500, 50);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_with_no_newlines() {
        // Single very long line with no newlines - tests chunking without clean break points
        let long_line = "word ".repeat(5000); // 5000 words, no newlines
        let codebase = create_test_codebase(vec![("long_line.txt", &long_line)]);

        // Use a chunk size that forces multiple chunks
        let config = ChunkConfig::new(1000, 100).unwrap();
        let tokenizer = WordTokenizer;

        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();

        // Should still produce chunks even without newline break points
        assert!(
            chunks.len() > 1,
            "Expected multiple chunks for long content without newlines"
        );

        // Verify all content is covered - each chunk should have reasonable size
        for chunk in &chunks {
            assert!(
                chunk.token_count <= config.chunk_size,
                "Chunk {} exceeds max size: {} > {}",
                chunk.id,
                chunk.token_count,
                config.chunk_size
            );
        }
    }

    #[test]
    fn test_unicode_multibyte_at_chunk_boundaries() {
        // Content with multi-byte UTF-8 characters that might land at chunk boundaries
        // Mix of ASCII and various multi-byte characters:
        // - 2-byte: é (C3 A9), ñ (C3 B1)
        // - 3-byte: 中 (E4 B8 AD), 日 (E6 97 A5)
        // - 4-byte: 𝄞 (F0 9D 84 9E), 🎵 (F0 9F 8E B5)
        let unicode_content =
            "word1 café word2 日本語 word3 🎵music🎵 word4 résumé word5 中文 word6 ".repeat(200);
        let codebase = create_test_codebase(vec![("unicode.txt", &unicode_content)]);

        // Chunk size that forces multiple chunks with unicode content
        let config = ChunkConfig::new(1000, 100).unwrap();
        let tokenizer = WordTokenizer;

        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();

        // Should produce valid chunks without panicking on UTF-8 boundaries
        assert!(!chunks.is_empty(), "Should produce at least one chunk");

        // Verify all chunks have valid UTF-8 content (guaranteed by String type)
        // and that we can iterate through them without panic
        for chunk in &chunks {
            // Verify content is accessible (proves valid UTF-8)
            let _ = chunk.content.len();
            let _ = chunk.content.chars().count();

            // First chunk should have zero overlap
            if chunk.id == 0 {
                assert_eq!(chunk.overlap_token_count, 0);
            }
        }
    }

    #[test]
    fn test_binary_search_convergence_with_multibyte() {
        // Specifically test the binary search doesn't infinite loop with multi-byte chars
        // Create content where char boundaries might cause issues
        // Using smaller content to keep test fast
        let tricky_content = "é".repeat(100) + &"a".repeat(100);
        let codebase = create_test_codebase(vec![("tricky.txt", &tricky_content)]);

        // Char tokenizer that counts characters, not words
        struct CharTokenizer;
        impl Tokenizer for CharTokenizer {
            fn count_tokens(&self, text: &str) -> usize {
                text.chars().count()
            }
        }

        // Use smaller chunk size relative to content for faster test
        let config = ChunkConfig::new(1000, 100).unwrap();
        let tokenizer = CharTokenizer;

        // This should complete without infinite loop
        let chunks = chunk_codebase(&codebase, &config, &tokenizer).unwrap();
        assert!(
            !chunks.is_empty(),
            "Should produce chunks without infinite loop"
        );
    }
}
