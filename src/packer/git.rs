use crate::utils::error::RuleyError;
use git2::Repository;
use std::path::{Path, PathBuf};

pub async fn clone_repo(
    url: &str,
    dest: &Path,
) -> Result<Repository, crate::utils::error::RuleyError> {
    let repo = Repository::clone(url, dest)?;
    Ok(repo)
}

pub fn is_git_repo<P: AsRef<Path>>(path: P) -> bool {
    Repository::open(path).is_ok()
}

pub async fn find_git_root(path: &Path) -> Result<PathBuf, RuleyError> {
    let mut current = path;
    loop {
        // Check current directory for .git (both directory and file)
        let git_path = current.join(".git");
        if git_path.exists() {
            return Ok(current.to_path_buf());
        }

        // Ascend to parent directory
        if let Some(parent) = current.parent() {
            current = parent;
        } else {
            // Reached filesystem root without finding .git
            return Err(git2::Error::from_str("No git root found").into());
        }
    }
}
