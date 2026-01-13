use crate::utils::error::RuleyError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    TypeScript,
    JavaScript,
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

pub struct TreeSitterCompressor;

impl Compressor for TreeSitterCompressor {
    fn compress(&self, _source: &str, _language: Language) -> Result<String, RuleyError> {
        // TODO: Implement tree-sitter compression
        todo!("Tree-sitter compression not yet implemented")
    }

    fn compression_ratio(&self) -> f32 {
        0.7 // Target ~70% token reduction
    }
}
