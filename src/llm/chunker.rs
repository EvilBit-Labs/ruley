use crate::utils::error::RuleyError;

pub struct Chunker {
    #[allow(dead_code)]
    max_tokens: usize,
}

impl Chunker {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    pub fn chunk(&self, _text: &str, _token_count: usize) -> Result<Vec<String>, RuleyError> {
        // TODO: Implement token-aware chunking
        todo!("Chunking not yet implemented")
    }
}
