use crate::utils::error::RuleyError;
use ignore::WalkBuilder;
use std::path::Path;

pub struct FileWalker {
    root: std::path::PathBuf,
}

impl FileWalker {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn walk(&self) -> Result<Vec<std::path::PathBuf>, RuleyError> {
        let mut files = Vec::new();

        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for result in walker {
            let entry = result.map_err(|e| {
                RuleyError::FileSystem(std::io::Error::other(format!(
                    "Failed to walk directory: {}",
                    e
                )))
            })?;
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                files.push(entry.path().to_path_buf());
            }
        }

        Ok(files)
    }
}
