use crate::utils::error::RuleyError;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;

pub struct GitIgnorer {
    gitignore: ignore::gitignore::Gitignore,
}

impl GitIgnorer {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, RuleyError> {
        let mut builder = GitignoreBuilder::new(root);
        builder
            .add_line(None, ".git")
            .map_err(|e: ignore::Error| RuleyError::Config(e.to_string()))?;
        let gitignore = builder
            .build()
            .map_err(|e: ignore::Error| RuleyError::Config(e.to_string()))?;
        Ok(Self { gitignore })
    }

    pub fn is_ignored<P: AsRef<Path>>(&self, path: P) -> bool {
        self.gitignore
            .matched_path_or_any_parents(path, false)
            .is_ignore()
    }
}
