use crate::utils::error::RuleyError;
use git2::Repository;
use std::path::Path;

pub async fn clone_repo(url: &str, dest: &Path) -> Result<Repository, RuleyError> {
    let repo = Repository::clone(url, dest)?;
    Ok(repo)
}

pub fn is_git_repo<P: AsRef<Path>>(path: P) -> bool {
    Repository::open(path).is_ok()
}
