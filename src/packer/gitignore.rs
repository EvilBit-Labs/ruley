use crate::utils::error::RuleyError;
use ignore::gitignore::GitignoreBuilder;
use std::path::Path;

pub struct GitIgnorer {
    gitignore: ignore::gitignore::Gitignore,
}

impl GitIgnorer {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, RuleyError> {
        let mut builder = GitignoreBuilder::new(&root);

        // Add built-in .git ignore
        builder
            .add_line(None, ".git")
            .map_err(|e: ignore::Error| RuleyError::Config(e.to_string()))?;

        // Load .gitignore file from the repository root
        let gitignore_path = root.as_ref().join(".gitignore");
        if gitignore_path.exists() {
            builder.add(gitignore_path);
        }

        // Optionally load .git/info/exclude if it exists
        let git_exclude_path = root.as_ref().join(".git/info/exclude");
        if git_exclude_path.exists() {
            builder.add(git_exclude_path);
        }

        let gitignore = builder
            .build()
            .map_err(|e: ignore::Error| RuleyError::Config(e.to_string()))?;
        Ok(Self { gitignore })
    }

    pub fn is_ignored<P: AsRef<Path>>(&self, path: P) -> bool {
        let path = path.as_ref();

        // Try matching as a file first
        if self
            .gitignore
            .matched_path_or_any_parents(path, false)
            .is_ignore()
        {
            return true;
        }

        // Try matching as a directory (for patterns like "node_modules/")
        self.gitignore
            .matched_path_or_any_parents(path, true)
            .is_ignore()
    }
}
