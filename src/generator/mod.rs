//! Rule generation module for AI IDE rules.
//!
//! This module provides:
//! - Prompt generation for LLM analysis and refinement
//! - Rule structures for storing generated rules
//! - Response parsing for LLM outputs

pub mod prompts;
pub mod refinement;
pub mod rules;

pub use prompts::{build_analysis_prompt, build_refinement_prompt, build_smart_merge_prompt};
pub use refinement::{RefinementResult, refine_invalid_output};
pub use rules::{
    FormattedRules, GeneratedRules, GenerationMetadata, RuleType, get_default_rule_type,
    parse_analysis_response,
};
