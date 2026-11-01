//! Change-detail collection for one repository (research R4, FR-002, FR-009).
//!
//! Reads exclusively through `GitRunner` and the system `git` binary
//! (Principle III) and is strictly read-only: nothing here stages, writes or
//! commits (Principle V). The payload mirrors exactly what the existing commit
//! step would include — tracked changes plus untracked file content.

use crate::git_runner::GitRunner;
use std::path::Path;
use thiserror::Error;

/// Lines taken from each untracked file before moving on (research R4).
pub const MAX_UNTRACKED_LINES_PER_FILE: usize = 200;
/// Recent commit subjects fetched for style alignment (FR-003, research R4).
pub const STYLE_SUBJECT_COUNT: usize = 10;

/// Why a repository cannot be summarized (rendered as a per-row error, FR-008).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChangeDetailError {
    #[error("nothing to summarize")]
    NothingToSummarize,
    #[error("merge conflicts — resolve first")]
    Conflicts,
    #[error("repository is not readable: {0}")]
    Unavailable(String),
}

/// The bounded prompt input plus the style samples the agent receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeDetail {
    /// Ready-to-send change detail (already truncated if it exceeded the budget).
    pub prompt: String,
    /// Up to [`STYLE_SUBJECT_COUNT`] recent commit subjects.
    pub style_subjects: Vec<String>,
    /// Whether the payload hit the character budget.
    pub truncated: bool,
    /// Files named in the payload, for diagnostics only (never logged in full).
    pub untracked_count: usize,
}

/// Collects `git diff HEAD` + bounded untracked content + recent subjects, then
/// truncates the whole payload to `diff_budget_chars` (FR-009). Larger change
/// sets degrade in quality but must never fail or hang (spec edge case).
pub async fn collect_change_detail(
    repo_path: &Path,
    diff_budget_chars: usize,
) -> Result<ChangeDetail, ChangeDetailError> {
    if !GitRunner::resolve_git_dir(repo_path).exists() {
        return Err(ChangeDetailError::Unavailable(format!(
            "{} is not a readable git repository",
            repo_path.display()
        )));
    }

    if GitRunner::has_conflicts(repo_path).await {
        return Err(ChangeDetailError::Conflicts);
    }

    let diff = GitRunner::diff_head(repo_path).await.unwrap_or_default();
    let untracked = GitRunner::untracked_files(repo_path).await;

    if diff.trim().is_empty() && untracked.is_empty() {
        return Err(ChangeDetailError::NothingToSummarize);
    }

    let mut body = String::new();
    if !diff.trim().is_empty() {
        body.push_str("## Tracked changes (git diff HEAD)\n\n");
        body.push_str(diff.trim_end());
        body.push('\n');
    }

    if !untracked.is_empty() {
        body.push_str("\n## Untracked files\n\n");
        for path in &untracked {
            let content = read_head_lines(&repo_path.join(path), MAX_UNTRACKED_LINES_PER_FILE);
            body.push_str(&format!("--- new file: {path} ---\n"));
            match content {
                Some(lines) => {
                    body.push_str(&lines);
                    if !lines.ends_with('\n') {
                        body.push('\n');
                    }
                }
                None => body.push_str("(binary or unreadable)\n"),
            }
        }
    }

    let (prompt, truncated) = truncate(&body, diff_budget_chars);
    let style_subjects = GitRunner::recent_subjects(repo_path, STYLE_SUBJECT_COUNT).await;

    Ok(ChangeDetail {
        prompt,
        style_subjects,
        truncated,
        untracked_count: untracked.len(),
    })
}

/// Truncates to `budget` characters, keeping the marker inside the budget
/// (research R4). Returns `(payload, truncated)`.
pub fn truncate(body: &str, budget: usize) -> (String, bool) {
    if body.chars().count() <= budget {
        return (body.to_string(), false);
    }
    let marker_chars = super::TRUNCATION_MARKER.chars().count();
    if budget <= marker_chars {
        // Budget too small to carry the marker: hard-trim, still bounded.
        return (body.chars().take(budget).collect(), true);
    }
    let keep = budget - marker_chars - 1; // one newline separates content from marker
    let head: String = body.chars().take(keep).collect();
    (
        format!("{}\n{}", head.trim_end(), super::TRUNCATION_MARKER),
        true,
    )
}

fn read_head_lines(path: &Path, max_lines: usize) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text = std::str::from_utf8(&bytes).ok()?;
    let mut kept: Vec<&str> = Vec::new();
    for line in text.lines().take(max_lines) {
        kept.push(line);
    }
    let mut out = kept.join("\n");
    if text.lines().count() > max_lines && !out.is_empty() {
        out.push_str("\n[… file longer than shown …]");
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Stdio;

    fn git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .current_dir(dir)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git available");
        assert!(status.success(), "git {args:?} failed");
    }

    fn repo_with_commit() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        git(&path, &["init", "-b", "main"]);
        git(&path, &["config", "user.email", "test@example.com"]);
        git(&path, &["config", "user.name", "RepoMan Tests"]);
        std::fs::write(path.join("tracked.txt"), "one\n").unwrap();
        git(&path, &["add", "tracked.txt"]);
        git(&path, &["commit", "-m", "feat: add the tracked file"]);
        (dir, path)
    }

    #[test]
    fn truncation_keeps_the_marker_inside_the_budget() {
        let body = "x".repeat(500);
        let (payload, truncated) = truncate(&body, 40_000);
        assert!(!truncated);
        assert_eq!(payload.chars().count(), 500);

        let (payload, truncated) = truncate(&body, 100);
        assert!(truncated);
        assert!(payload.ends_with(super::super::TRUNCATION_MARKER));
        assert!(
            payload.chars().count() <= 100,
            "{}",
            payload.chars().count()
        );

        let (tiny, _) = truncate(&body, 5);
        assert!(tiny.chars().count() <= 5, "{tiny}");
    }

    #[tokio::test]
    async fn collects_tracked_and_untracked_changes_with_style_subjects() {
        let (_guard, path) = repo_with_commit();
        std::fs::write(path.join("tracked.txt"), "one\ntwo\n").unwrap();
        std::fs::write(path.join("notes.txt"), "brand new\n").unwrap();

        let detail = collect_change_detail(&path, 40_000).await.unwrap();
        assert!(detail.prompt.contains("Tracked changes"));
        assert!(detail.prompt.contains("+two"));
        assert!(detail.prompt.contains("brand new"));
        assert!(detail.prompt.contains("notes.txt"));
        assert!(!detail.truncated);
        assert_eq!(detail.untracked_count, 1);
        assert_eq!(
            detail.style_subjects,
            vec!["feat: add the tracked file".to_string()]
        );
    }

    #[tokio::test]
    async fn caps_untracked_files_at_two_hundred_lines() {
        let (_guard, path) = repo_with_commit();
        let lines: Vec<String> = (1..=300).map(|i| format!("line {i}")).collect();
        std::fs::write(path.join("long.txt"), lines.join("\n")).unwrap();

        let detail = collect_change_detail(&path, 40_000).await.unwrap();
        assert!(detail.prompt.contains("line 200"));
        assert!(!detail.prompt.contains("line 201"));
        assert!(detail.prompt.contains("[… file longer than shown …]"));
    }

    #[tokio::test]
    async fn huge_change_sets_truncate_instead_of_failing() {
        let (_guard, path) = repo_with_commit();
        let fat = "y".repeat(100_000);
        std::fs::write(path.join("tracked.txt"), format!("one\n{fat}\n")).unwrap();

        let detail = collect_change_detail(&path, 1_000).await.unwrap();
        assert!(detail.truncated);
        assert!(detail.prompt.chars().count() <= 1_000);
        assert!(detail.prompt.ends_with(super::super::TRUNCATION_MARKER));
    }

    #[tokio::test]
    async fn clean_repository_has_nothing_to_summarize() {
        let (_guard, path) = repo_with_commit();
        assert_eq!(
            collect_change_detail(&path, 40_000).await.unwrap_err(),
            ChangeDetailError::NothingToSummarize
        );
    }

    #[tokio::test]
    async fn merge_conflict_state_is_reported_not_summarized() {
        let (_guard, path) = repo_with_commit();
        // Both branches edit the same line from the shared base commit.
        std::fs::write(path.join("tracked.txt"), "main-side change\n").unwrap();
        git(&path, &["commit", "-am", "fix: main side"]);
        git(&path, &["checkout", "-b", "side", "HEAD~1"]);
        std::fs::write(path.join("tracked.txt"), "branch-side change\n").unwrap();
        git(&path, &["commit", "-am", "fix: branch side"]);
        git(&path, &["checkout", "main"]);
        // Conflicting merge — leaves unmerged paths in the index.
        let merged = std::process::Command::new("git")
            .current_dir(&path)
            .args(["merge", "side"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git merge runs");
        assert!(!merged.success(), "expected a conflicted merge");

        assert_eq!(
            collect_change_detail(&path, 40_000).await.unwrap_err(),
            ChangeDetailError::Conflicts
        );
    }

    #[tokio::test]
    async fn style_subjects_are_capped_at_ten() {
        let (_guard, path) = repo_with_commit();
        for i in 0..15 {
            git(
                &path,
                &["commit", "--allow-empty", "-m", &format!("chore: seed {i}")],
            );
        }
        let subjects = GitRunner::recent_subjects(&path, STYLE_SUBJECT_COUNT).await;
        assert_eq!(subjects.len(), 10);
        assert_eq!(subjects[0], "chore: seed 14");
    }
}
