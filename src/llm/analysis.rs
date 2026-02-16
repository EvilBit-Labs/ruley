//! Chunk analysis and merge logic for processing large codebases.
//!
//! This module provides functionality to process large codebases that have been
//! split into chunks. It handles sequential analysis of each chunk and merging
//! the results into a coherent final output.
//!
//! # Example
//!
//! ```ignore
//! use ruley::llm::analysis::{analyze_chunked, ChunkResult};
//! use ruley::llm::chunker::Chunk;
//! use ruley::llm::client::LLMClient;
//!
//! let chunks = vec![/* chunks from chunker */];
//! let prompt_template = "Analyze this codebase portion...";
//! let client = LLMClient::new(/* provider */);
//!
//! let result = analyze_chunked(chunks, prompt_template, &client).await?;
//! ```

use crate::llm::chunker::Chunk;
use crate::llm::client::LLMClient;
use crate::llm::provider::{CompletionOptions, Message};
use crate::utils::error::RuleyError;
use tracing::{debug, info};

/// Result of analyzing a single chunk.
///
/// Contains the chunk identifier, the LLM's analysis output, and separate
/// prompt/completion token counts from the provider response.
#[derive(Debug, Clone)]
pub struct ChunkResult {
    /// The chunk ID (0-indexed).
    pub chunk_id: usize,

    /// The analysis output from the LLM for this chunk.
    pub analysis: String,

    /// Number of prompt/input tokens used for this chunk analysis.
    pub prompt_tokens: usize,

    /// Number of completion/output tokens used for this chunk analysis.
    pub completion_tokens: usize,
}

impl ChunkResult {
    /// Create a new chunk result with separate prompt and completion token counts.
    ///
    /// # Arguments
    ///
    /// * `chunk_id` - The ID of the analyzed chunk
    /// * `analysis` - The LLM's analysis output
    /// * `prompt_tokens` - Number of prompt/input tokens reported by the provider
    /// * `completion_tokens` - Number of completion/output tokens reported by the provider
    #[must_use]
    pub fn new(
        chunk_id: usize,
        analysis: String,
        prompt_tokens: usize,
        completion_tokens: usize,
    ) -> Self {
        Self {
            chunk_id,
            analysis,
            prompt_tokens,
            completion_tokens,
        }
    }

    /// Total tokens used (prompt + completion).
    #[must_use]
    pub fn total_tokens(&self) -> usize {
        self.prompt_tokens + self.completion_tokens
    }
}

/// Configuration options for chunk analysis.
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
    /// Maximum tokens for LLM response per chunk.
    pub max_tokens: Option<usize>,

    /// Temperature for LLM generation.
    pub temperature: Option<f32>,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            max_tokens: Some(4096),
            temperature: Some(0.3),
        }
    }
}

impl From<&AnalysisOptions> for CompletionOptions {
    fn from(opts: &AnalysisOptions) -> Self {
        Self {
            max_tokens: opts.max_tokens,
            temperature: opts.temperature,
        }
    }
}

/// Result of a full analysis including per-chunk token counts.
///
/// Contains the merged analysis text along with individual chunk results
/// and merge-step token counts, enabling accurate cost tracking from
/// provider-reported values.
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// The final merged analysis text.
    pub merged_analysis: String,
    /// Per-chunk results with provider-reported token counts.
    pub chunk_results: Vec<ChunkResult>,
    /// Prompt tokens used in the merge step (0 if single chunk).
    pub merge_prompt_tokens: usize,
    /// Completion tokens used in the merge step (0 if single chunk).
    pub merge_completion_tokens: usize,
}

/// Analyze a codebase and return detailed results with per-chunk token counts.
///
/// Like [`analyze_chunked`] but returns an [`AnalysisResult`] with provider-reported
/// token counts for accurate cost tracking.
pub async fn analyze_chunked_with_results(
    chunks: Vec<Chunk>,
    prompt_template: &str,
    client: &LLMClient,
) -> Result<AnalysisResult, RuleyError> {
    if chunks.is_empty() {
        return Err(RuleyError::ValidationError {
            message: "No chunks to analyze".to_string(),
            suggestion: "Ensure the codebase has content before analysis".to_string(),
        });
    }

    let total_chunks = chunks.len();
    let options = AnalysisOptions::default();

    if total_chunks == 1 {
        info!("Analyzing single chunk (no merge required)");
        let chunk = &chunks[0];
        let prompt = build_single_chunk_prompt(prompt_template, &chunk.content);
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
        }];

        let completion_options = CompletionOptions::from(&options);
        let response = client.complete(&messages, &completion_options).await?;

        debug!(
            prompt_tokens = response.prompt_tokens,
            completion_tokens = response.completion_tokens,
            "Single chunk analysis complete"
        );

        let chunk_result = ChunkResult::new(
            chunk.id,
            response.content.clone(),
            response.prompt_tokens,
            response.completion_tokens,
        );

        return Ok(AnalysisResult {
            merged_analysis: response.content,
            chunk_results: vec![chunk_result],
            merge_prompt_tokens: 0,
            merge_completion_tokens: 0,
        });
    }

    info!(total_chunks = total_chunks, "Analyzing multiple chunks");

    let chunk_results =
        analyze_chunks_sequentially(&chunks, prompt_template, client, &options).await?;

    // Merge all chunk results, capturing merge-step token counts
    let merge_prompt = build_merge_prompt(&chunk_results);
    let merge_messages = vec![Message {
        role: "user".to_string(),
        content: merge_prompt,
    }];

    let merge_options = CompletionOptions {
        max_tokens: options.max_tokens.map(|t| t.saturating_mul(2)),
        temperature: options.temperature,
    };

    let merge_response = client.complete(&merge_messages, &merge_options).await?;

    debug!(
        prompt_tokens = merge_response.prompt_tokens,
        completion_tokens = merge_response.completion_tokens,
        "Chunk results merged successfully"
    );

    Ok(AnalysisResult {
        merged_analysis: merge_response.content,
        chunk_results,
        merge_prompt_tokens: merge_response.prompt_tokens,
        merge_completion_tokens: merge_response.completion_tokens,
    })
}

/// Analyze a codebase that has been split into chunks.
///
/// Processes each chunk sequentially, building context-aware prompts that
/// include the chunk's position (N of M). After all chunks are analyzed,
/// merges the results into a coherent final output.
///
/// # Arguments
///
/// * `chunks` - The chunks to analyze (from the chunker module)
/// * `prompt_template` - The base prompt template for analysis
/// * `client` - The LLM client for making requests
///
/// # Returns
///
/// The merged analysis result combining insights from all chunks.
///
/// # Errors
///
/// Returns an error if any LLM call fails or if merging fails.
///
/// # Example
///
/// ```ignore
/// use ruley::llm::analysis::analyze_chunked;
/// use ruley::llm::chunker::{ChunkConfig, chunk_codebase};
///
/// let chunks = chunk_codebase(&codebase, &config, &tokenizer)?;
/// let result = analyze_chunked(chunks, "Analyze this codebase...", &client).await?;
/// println!("Analysis: {}", result);
/// ```
pub async fn analyze_chunked(
    chunks: Vec<Chunk>,
    prompt_template: &str,
    client: &LLMClient,
) -> Result<String, RuleyError> {
    analyze_chunked_with_options(chunks, prompt_template, client, &AnalysisOptions::default()).await
}

/// Analyze a codebase with custom options.
///
/// Like [`analyze_chunked`] but allows customizing LLM parameters.
///
/// # Arguments
///
/// * `chunks` - The chunks to analyze
/// * `prompt_template` - The base prompt template for analysis
/// * `client` - The LLM client for making requests
/// * `options` - Custom analysis options
///
/// # Returns
///
/// The merged analysis result combining insights from all chunks.
///
/// # Errors
///
/// Returns an error if any LLM call fails or if merging fails.
pub async fn analyze_chunked_with_options(
    chunks: Vec<Chunk>,
    prompt_template: &str,
    client: &LLMClient,
    options: &AnalysisOptions,
) -> Result<String, RuleyError> {
    if chunks.is_empty() {
        return Err(RuleyError::ValidationError {
            message: "No chunks to analyze".to_string(),
            suggestion: "Ensure the codebase has content before analysis".to_string(),
        });
    }

    let total_chunks = chunks.len();

    // Single chunk case: no merging needed
    if total_chunks == 1 {
        info!("Analyzing single chunk (no merge required)");
        let chunk = &chunks[0];
        let prompt = build_single_chunk_prompt(prompt_template, &chunk.content);
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
        }];

        let completion_options = CompletionOptions::from(options);
        let response = client.complete(&messages, &completion_options).await?;

        debug!(
            prompt_tokens = response.prompt_tokens,
            completion_tokens = response.completion_tokens,
            "Single chunk analysis complete"
        );
        return Ok(response.content);
    }

    // Multiple chunks: analyze each and merge
    info!(total_chunks = total_chunks, "Analyzing multiple chunks");

    let chunk_results =
        analyze_chunks_sequentially(&chunks, prompt_template, client, options).await?;

    // Merge all chunk results
    merge_chunk_results(chunk_results, client, options).await
}

/// Analyze chunks sequentially, building context-aware prompts.
///
/// # Arguments
///
/// * `chunks` - The chunks to analyze
/// * `prompt_template` - The base prompt template
/// * `client` - The LLM client
/// * `options` - Analysis options
///
/// # Returns
///
/// A vector of chunk results, one for each input chunk.
async fn analyze_chunks_sequentially(
    chunks: &[Chunk],
    prompt_template: &str,
    client: &LLMClient,
    options: &AnalysisOptions,
) -> Result<Vec<ChunkResult>, RuleyError> {
    let total_chunks = chunks.len();
    let mut results = Vec::with_capacity(total_chunks);
    let completion_options = CompletionOptions::from(options);

    for chunk in chunks {
        let chunk_number = chunk.id + 1; // 1-indexed for human readability
        debug!(
            chunk = chunk_number,
            total = total_chunks,
            tokens = chunk.token_count,
            "Analyzing chunk"
        );

        let prompt =
            build_chunk_prompt(prompt_template, &chunk.content, chunk_number, total_chunks);
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt,
        }];

        let response = client.complete(&messages, &completion_options).await?;

        debug!(
            chunk = chunk_number,
            prompt_tokens = response.prompt_tokens,
            completion_tokens = response.completion_tokens,
            "Chunk analysis complete"
        );

        results.push(ChunkResult::new(
            chunk.id,
            response.content,
            response.prompt_tokens,
            response.completion_tokens,
        ));
    }

    Ok(results)
}

/// Merge multiple chunk analysis results into a coherent final output.
///
/// This function takes the individual analysis results from each chunk and
/// synthesizes them into a single, deduplicated, coherent set of rules and
/// insights.
///
/// # Arguments
///
/// * `chunk_results` - The analysis results from each chunk
/// * `client` - The LLM client for making the merge request
///
/// # Returns
///
/// A merged, coherent analysis combining insights from all chunks.
///
/// # Errors
///
/// Returns an error if the merge LLM call fails.
///
/// # Example
///
/// ```ignore
/// use ruley::llm::analysis::{merge_chunk_results, ChunkResult};
///
/// let results = vec![
///     ChunkResult::new(0, "Analysis of chunk 1...".to_string(), 500),
///     ChunkResult::new(1, "Analysis of chunk 2...".to_string(), 450),
/// ];
///
/// let merged = merge_chunk_results(results, &client).await?;
/// ```
pub async fn merge_chunk_results(
    chunk_results: Vec<ChunkResult>,
    client: &LLMClient,
    options: &AnalysisOptions,
) -> Result<String, RuleyError> {
    if chunk_results.is_empty() {
        return Err(RuleyError::ValidationError {
            message: "No chunk results to merge".to_string(),
            suggestion: "Ensure chunks were analyzed before merging".to_string(),
        });
    }

    // Single result: no merge needed
    if let [single] = &chunk_results[..] {
        return Ok(single.analysis.clone());
    }

    info!(
        num_results = chunk_results.len(),
        "Merging chunk analysis results"
    );

    let prompt = build_merge_prompt(&chunk_results);
    let messages = vec![Message {
        role: "user".to_string(),
        content: prompt,
    }];

    // Use higher max_tokens for merge since we're combining multiple analyses
    let merge_options = CompletionOptions {
        max_tokens: options.max_tokens.map(|t| t.saturating_mul(2)),
        temperature: options.temperature,
    };

    let response = client.complete(&messages, &merge_options).await?;

    debug!(
        prompt_tokens = response.prompt_tokens,
        completion_tokens = response.completion_tokens,
        "Chunk results merged successfully"
    );

    Ok(response.content)
}

/// Build a prompt for analyzing a single chunk (when no chunking is needed).
///
/// # Arguments
///
/// * `prompt_template` - The base prompt template
/// * `content` - The codebase content
fn build_single_chunk_prompt(prompt_template: &str, content: &str) -> String {
    format!(
        "{prompt_template}\n\n\
        <codebase>\n\
        {content}\n\
        </codebase>"
    )
}

/// Build a prompt for analyzing a specific chunk.
///
/// Includes context about the chunk's position (N of M) to help the LLM
/// understand this is part of a larger analysis.
///
/// # Arguments
///
/// * `prompt_template` - The base prompt template
/// * `content` - The chunk content
/// * `chunk_number` - The chunk number (1-indexed)
/// * `total_chunks` - Total number of chunks
fn build_chunk_prompt(
    prompt_template: &str,
    content: &str,
    chunk_number: usize,
    total_chunks: usize,
) -> String {
    format!(
        "{prompt_template}\n\n\
        NOTE: This is chunk {chunk_number} of {total_chunks} from a large codebase.\n\
        Focus on extracting insights from this portion. The results will be merged later.\n\
        If you see partial code or references to code not in this chunk, note it but focus on what's present.\n\n\
        <codebase_chunk id=\"{chunk_number}\" total=\"{total_chunks}\">\n\
        {content}\n\
        </codebase_chunk>"
    )
}

/// Build the merge prompt for combining chunk analyses.
///
/// Instructs the LLM to synthesize multiple partial analyses into a coherent
/// whole, deduplicating insights and combining observations.
///
/// # Arguments
///
/// * `chunk_results` - The analysis results from all chunks
fn build_merge_prompt(chunk_results: &[ChunkResult]) -> String {
    let mut analyses = String::new();

    for result in chunk_results {
        analyses.push_str(&format!(
            "<chunk_analysis id=\"{}\">\n{}\n</chunk_analysis>\n\n",
            result.chunk_id + 1,
            result.analysis
        ));
    }

    format!(
        "You are merging the analysis results from {count} chunks of a large codebase.\n\
        Each chunk was analyzed separately. Your task is to:\n\n\
        1. **Synthesize** all insights into a coherent, unified analysis\n\
        2. **Deduplicate** any repeated observations or rules\n\
        3. **Combine** similar conventions or patterns into single, comprehensive rules\n\
        4. **Resolve conflicts** by choosing the most specific or accurate insight\n\
        5. **Preserve** important details that appear in only one chunk\n\n\
        Output a single, well-organized analysis that reads as if the entire codebase was analyzed at once.\n\
        Do not mention chunks or the merge process in your output.\n\n\
        <chunk_analyses>\n\
        {analyses}\
        </chunk_analyses>",
        count = chunk_results.len(),
        analyses = analyses
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::provider::{CompletionResponse, LLMProvider, Pricing};
    use async_trait::async_trait;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Mock provider for testing analysis logic.
    struct MockAnalysisProvider {
        call_count: Arc<AtomicUsize>,
        responses: Vec<String>,
    }

    impl MockAnalysisProvider {
        fn new(responses: Vec<String>) -> Self {
            Self {
                call_count: Arc::new(AtomicUsize::new(0)),
                responses,
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockAnalysisProvider {
        async fn complete(
            &self,
            _messages: &[Message],
            _options: &CompletionOptions,
        ) -> Result<CompletionResponse, RuleyError> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst);
            let response = self
                .responses
                .get(idx)
                .cloned()
                .unwrap_or_else(|| format!("Response {}", idx));

            Ok(CompletionResponse::new(response, 50, 50))
        }

        fn model(&self) -> &str {
            "mock-model"
        }

        fn pricing(&self) -> Pricing {
            Pricing {
                input_per_1k: 0.0,
                output_per_1k: 0.0,
            }
        }
    }

    fn create_test_chunk(id: usize, content: &str) -> Chunk {
        Chunk {
            id,
            content: content.to_string(),
            token_count: content.split_whitespace().count(),
            overlap_token_count: 0,
        }
    }

    #[test]
    fn test_chunk_result_new() {
        let result = ChunkResult::new(0, "Analysis output".to_string(), 100, 50);
        assert_eq!(result.chunk_id, 0);
        assert_eq!(result.analysis, "Analysis output");
        assert_eq!(result.prompt_tokens, 100);
        assert_eq!(result.completion_tokens, 50);
        assert_eq!(result.total_tokens(), 150);
    }

    #[test]
    fn test_analysis_options_default() {
        let opts = AnalysisOptions::default();
        assert_eq!(opts.max_tokens, Some(4096));
        assert_eq!(opts.temperature, Some(0.3));
    }

    #[test]
    fn test_build_single_chunk_prompt() {
        let template = "Analyze this codebase";
        let content = "fn main() {}";

        let prompt = build_single_chunk_prompt(template, content);

        assert!(prompt.contains("Analyze this codebase"));
        assert!(prompt.contains("<codebase>"));
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("</codebase>"));
    }

    #[test]
    fn test_build_chunk_prompt() {
        let template = "Analyze this codebase";
        let content = "fn main() {}";

        let prompt = build_chunk_prompt(template, content, 2, 5);

        assert!(prompt.contains("Analyze this codebase"));
        assert!(prompt.contains("chunk 2 of 5"));
        assert!(prompt.contains("<codebase_chunk id=\"2\" total=\"5\">"));
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("</codebase_chunk>"));
    }

    #[test]
    fn test_build_merge_prompt() {
        let results = vec![
            ChunkResult::new(0, "Analysis 1".to_string(), 50, 50),
            ChunkResult::new(1, "Analysis 2".to_string(), 50, 50),
        ];

        let prompt = build_merge_prompt(&results);

        assert!(prompt.contains("merging the analysis results from 2 chunks"));
        assert!(prompt.contains("<chunk_analysis id=\"1\">"));
        assert!(prompt.contains("Analysis 1"));
        assert!(prompt.contains("<chunk_analysis id=\"2\">"));
        assert!(prompt.contains("Analysis 2"));
        assert!(prompt.contains("Deduplicate"));
        assert!(prompt.contains("Synthesize"));
    }

    #[tokio::test]
    async fn test_analyze_single_chunk() {
        let provider = MockAnalysisProvider::new(vec!["Single chunk analysis".to_string()]);
        let client = LLMClient::new(Box::new(provider));

        let chunks = vec![create_test_chunk(0, "fn main() {}")];

        let result = analyze_chunked(chunks, "Analyze this", &client).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Single chunk analysis");
    }

    #[tokio::test]
    async fn test_analyze_multiple_chunks() {
        let provider = MockAnalysisProvider::new(vec![
            "Chunk 1 analysis".to_string(),
            "Chunk 2 analysis".to_string(),
            "Merged result".to_string(),
        ]);
        let call_count_check = provider.call_count.clone();
        let client = LLMClient::new(Box::new(provider));

        let chunks = vec![
            create_test_chunk(0, "fn main() {}"),
            create_test_chunk(1, "fn helper() {}"),
        ];

        let result = analyze_chunked(chunks, "Analyze this", &client).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Merged result");
        // Should have 3 calls: 2 chunks + 1 merge
        assert_eq!(call_count_check.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_analyze_empty_chunks() {
        let provider = MockAnalysisProvider::new(vec![]);
        let client = LLMClient::new(Box::new(provider));

        let chunks: Vec<Chunk> = vec![];

        let result = analyze_chunked(chunks, "Analyze this", &client).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            RuleyError::ValidationError { message, .. } => {
                assert!(message.contains("No chunks"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_merge_single_result() {
        let provider = MockAnalysisProvider::new(vec![]);
        let client = LLMClient::new(Box::new(provider));

        let results = vec![ChunkResult::new(0, "Only analysis".to_string(), 50, 50)];

        let merged = merge_chunk_results(results, &client, &AnalysisOptions::default()).await;

        assert!(merged.is_ok());
        assert_eq!(merged.unwrap(), "Only analysis");
    }

    #[tokio::test]
    async fn test_merge_empty_results() {
        let provider = MockAnalysisProvider::new(vec![]);
        let client = LLMClient::new(Box::new(provider));

        let results: Vec<ChunkResult> = vec![];

        let merged = merge_chunk_results(results, &client, &AnalysisOptions::default()).await;

        assert!(merged.is_err());
        match merged.unwrap_err() {
            RuleyError::ValidationError { message, .. } => {
                assert!(message.contains("No chunk results"));
            }
            _ => panic!("Expected ValidationError"),
        }
    }

    #[tokio::test]
    async fn test_analyze_with_custom_options() {
        let provider = MockAnalysisProvider::new(vec!["Custom analysis".to_string()]);
        let client = LLMClient::new(Box::new(provider));

        let chunks = vec![create_test_chunk(0, "fn main() {}")];
        let options = AnalysisOptions {
            max_tokens: Some(8192),
            temperature: Some(0.5),
        };

        let result = analyze_chunked_with_options(chunks, "Analyze this", &client, &options).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Custom analysis");
    }
}
