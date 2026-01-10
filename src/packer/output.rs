use crate::utils::error::RuleyError;

pub struct PackedCodebase {
    pub files: Vec<PackedFile>,
    pub total_tokens: usize,
}

pub struct PackedFile {
    pub path: String,
    pub content: String,
    pub tokens: usize,
}

pub fn pack_codebase(_files: &[std::path::PathBuf]) -> Result<PackedCodebase, RuleyError> {
    // TODO: Implement codebase packing
    todo!("Codebase packing not yet implemented")
}
