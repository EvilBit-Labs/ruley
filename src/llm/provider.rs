// Copyright (c) 2025-2026 the ruley contributors
// SPDX-License-Identifier: Apache-2.0

use crate::utils::error::RuleyError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionOptions {
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct CompletionResponse {
    pub content: String,
    /// Number of prompt/input tokens used.
    pub prompt_tokens: usize,
    /// Number of completion/output tokens used.
    pub completion_tokens: usize,
    /// Total tokens used (prompt + completion). Prefer using prompt_tokens + completion_tokens directly.
    pub tokens_used: usize,
}

impl CompletionResponse {
    /// Create a new CompletionResponse with separate token counts.
    pub fn new(content: String, prompt_tokens: usize, completion_tokens: usize) -> Self {
        Self {
            content,
            prompt_tokens,
            completion_tokens,
            tokens_used: prompt_tokens + completion_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pricing {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn complete(
        &self,
        messages: &[Message],
        options: &CompletionOptions,
    ) -> Result<CompletionResponse, RuleyError>;

    fn model(&self) -> &str;

    fn pricing(&self) -> Pricing;
}
