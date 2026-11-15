//! Resolving a saved state to a `<base>..<target>` pair (contract C4/C5).

use super::SavedCommit;
use crate::git_runner::GitRunner;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedError {
    SaveNotFound,
    HeadNotFound,
}

impl SavedError {
    pub fn code(self) -> &'static str {
        match self {
            SavedError::SaveNotFound => "save-not-found",
            SavedError::HeadNotFound => "head-not-found",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedSaved {
    pub base: String,
    pub target: String,
    pub saved: SavedCommit,
}

/// Resolves `commitHash` (absent = current `HEAD`) and its parent. A root commit
/// diffs against the well-known empty tree.
pub async fn resolve_saved(
    repo_path: &Path,
    commit_hash: Option<&str>,
) -> Result<ResolvedSaved, SavedError> {
    let explicit = commit_hash.is_some();
    let requested = match commit_hash {
        Some(hash) => hash.trim().to_string(),
        None => GitRunner::rev_parse_head(repo_path)
            .await
            .ok_or(SavedError::HeadNotFound)?,
    };

    if requested.is_empty() {
        return Err(if explicit {
            SavedError::SaveNotFound
        } else {
            SavedError::HeadNotFound
        });
    }

    let target = GitRunner::rev_parse_verify_commit(repo_path, &requested)
        .await
        .ok_or(if explicit {
            SavedError::SaveNotFound
        } else {
            SavedError::HeadNotFound
        })?;

    let (message, timestamp) = GitRunner::show_commit_subject_and_time(repo_path, &target)
        .await
        .ok_or(SavedError::SaveNotFound)?;

    let parents = GitRunner::rev_list_parents(repo_path, &target)
        .await
        .unwrap_or_default();
    let base = parents
        .get(1)
        .cloned()
        .unwrap_or_else(|| GitRunner::empty_tree().to_string());

    Ok(ResolvedSaved {
        base,
        target: target.clone(),
        saved: SavedCommit {
            hash: target,
            message,
            timestamp,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::status::saved_inventory;
    use crate::review::DiffStatus;
    use std::fs;
    use std::process::Command;

    fn run_git(dir: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .current_dir(dir)
            .args(args)
            .output()
            .expect("git must be installed");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    fn init(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        run_git(dir, &["init", "-b", "main"]);
        run_git(dir, &["config", "user.name", "Test"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        fs::write(dir.join("app.js"), "one\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(
            dir,
            &["-c", "commit.gpgsign=false", "commit", "-m", "initial"],
        );
    }

    #[tokio::test]
    async fn head_save_resolves_to_its_parent() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);
        let first = run_git(&repo, &["rev-parse", "HEAD"]);

        fs::write(repo.join("app.js"), "two\n").unwrap();
        run_git(&repo, &["add", "."]);
        run_git(
            &repo,
            &["-c", "commit.gpgsign=false", "commit", "-m", "second"],
        );

        let resolved = resolve_saved(&repo, None).await.unwrap();
        assert_eq!(resolved.base, first);
        assert_eq!(resolved.saved.message, "second");
        assert_eq!(resolved.target, run_git(&repo, &["rev-parse", "HEAD"]));

        let files = saved_inventory(&repo, &resolved.base, &resolved.target).await;
        assert!(files
            .iter()
            .any(|file| file.path == "app.js" && file.status == DiffStatus::Modified));
    }

    #[tokio::test]
    async fn root_commit_uses_the_empty_tree() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);

        let resolved = resolve_saved(&repo, None).await.unwrap();
        assert_eq!(resolved.base, GitRunner::empty_tree());
        assert_eq!(resolved.saved.message, "initial");
    }

    #[tokio::test]
    async fn unknown_hash_is_save_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);

        let error = resolve_saved(&repo, Some("0000000000000000000000000000000000000000"))
            .await
            .unwrap_err();
        assert_eq!(error, SavedError::SaveNotFound);
        assert_eq!(error.code(), "save-not-found");
    }

    #[tokio::test]
    async fn unborn_head_is_head_not_found() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        fs::create_dir_all(&repo).unwrap();
        run_git(&repo, &["init", "-b", "main"]);

        let error = resolve_saved(&repo, None).await.unwrap_err();
        assert_eq!(error, SavedError::HeadNotFound);
        assert_eq!(error.code(), "head-not-found");
    }
}
