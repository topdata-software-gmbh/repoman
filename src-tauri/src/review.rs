//! Read-only change viewer core (specs/004-diff-viewer).
//!
//! Every Git read, status/hunk parse, cap, rename/conflict/whitespace decision,
//! and saved-state resolution lives here. Transports only delegate.

pub mod image;
pub mod limits;
pub mod saved;
pub mod status;
pub mod text;

use crate::git_runner::GitRunner;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use limits::TruncationReason;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewMode {
    Unsaved,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectState {
    Ready,
    Empty,
    Unreadable,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffStatus {
    New,
    Modified,
    Deleted,
    Renamed,
    Conflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiffKind {
    Text,
    Image,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PreviewNotice {
    TooLarge,
    Unsupported,
    Corrupt,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageState {
    Ready,
    TooLarge,
    Unsupported,
    Corrupt,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSummary {
    pub file_count: usize,
    pub added_lines: usize,
    pub removed_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewError {
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedCommit {
    pub hash: String,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffFileEntry {
    pub path: String,
    pub old_path: Option<String>,
    pub status: DiffStatus,
    pub kind: DiffKind,
    pub whitespace_only: bool,
    pub added_lines: usize,
    pub removed_lines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectComparison {
    pub path: String,
    pub name: String,
    pub mode: ReviewMode,
    pub state: ProjectState,
    pub files: Vec<DiffFileEntry>,
    pub summary: ReviewSummary,
    pub saved: Option<SavedCommit>,
    pub error: Option<ReviewError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DiffLine {
    Same {
        old_number: usize,
        new_number: usize,
        old_text: String,
        new_text: String,
    },
    Changed {
        old_number: usize,
        new_number: usize,
        old_text: String,
        new_text: String,
    },
    Added {
        new_number: usize,
        new_text: String,
    },
    Removed {
        old_number: usize,
        old_text: String,
    },
    Fold {
        old_start: usize,
        new_start: usize,
        count: usize,
        rows: Vec<DiffLine>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextPreview {
    pub rows: Vec<DiffLine>,
    pub truncated: bool,
    pub truncation_reason: Option<TruncationReason>,
    pub raw: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageSide {
    pub data_url: String,
    pub width: usize,
    pub height: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreview {
    pub state: ImageState,
    pub old: Option<ImageSide>,
    pub new: Option<ImageSide>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryPreview {
    pub old_bytes: usize,
    pub new_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub path: String,
    pub old_path: Option<String>,
    pub status: DiffStatus,
    pub kind: DiffKind,
    pub whitespace_only: bool,
    pub added_lines: usize,
    pub removed_lines: usize,
    pub notice: Option<PreviewNotice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextPreview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImagePreview>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<BinaryPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewProjectRequest {
    pub path: String,
    pub mode: ReviewMode,
    #[serde(default)]
    pub commit_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFileRequest {
    pub path: String,
    pub mode: ReviewMode,
    #[serde(default)]
    pub commit_hash: Option<String>,
    pub file_path: String,
    #[serde(default)]
    pub old_path: Option<String>,
}

fn project_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string())
}

fn summarize(files: &[DiffFileEntry]) -> ReviewSummary {
    let mut summary = ReviewSummary {
        file_count: files.len(),
        ..ReviewSummary::default()
    };
    for entry in files {
        summary.added_lines += entry.added_lines;
        summary.removed_lines += entry.removed_lines;
    }
    summary
}

fn ready_comparison(
    request: &ReviewProjectRequest,
    files: Vec<DiffFileEntry>,
    saved: Option<SavedCommit>,
) -> ProjectComparison {
    let state = if files.is_empty() {
        ProjectState::Empty
    } else {
        ProjectState::Ready
    };
    let summary = summarize(&files);
    ProjectComparison {
        path: request.path.clone(),
        name: project_name(&request.path),
        mode: request.mode,
        state,
        files,
        summary,
        saved,
        error: None,
    }
}

fn unreadable_comparison(request: &ReviewProjectRequest) -> ProjectComparison {
    ProjectComparison {
        path: request.path.clone(),
        name: project_name(&request.path),
        mode: request.mode,
        state: ProjectState::Unreadable,
        files: Vec::new(),
        summary: ReviewSummary::default(),
        saved: None,
        error: Some(ReviewError {
            code: "unreadable".to_string(),
            detail: None,
        }),
    }
}

fn unavailable_comparison(request: &ReviewProjectRequest, code: &str) -> ProjectComparison {
    ProjectComparison {
        path: request.path.clone(),
        name: project_name(&request.path),
        mode: request.mode,
        state: ProjectState::Unavailable,
        files: Vec::new(),
        summary: ReviewSummary::default(),
        saved: None,
        error: Some(ReviewError {
            code: code.to_string(),
            detail: None,
        }),
    }
}

fn ensure_repository(path: &str) -> Result<PathBuf, String> {
    let repo_path = PathBuf::from(path);
    if !GitRunner::resolve_git_dir(&repo_path).exists() {
        return Err("not-a-repository".to_string());
    }
    Ok(repo_path)
}

/// Project summary: availability, file inventory, and totals (contract C2/C3).
pub async fn review_project(request: ReviewProjectRequest) -> Result<ProjectComparison, String> {
    let repo_path = ensure_repository(&request.path)?;

    match request.mode {
        ReviewMode::Unsaved => match status::unsaved_inventory_checked(&repo_path).await {
            Some(files) => Ok(ready_comparison(&request, files, None)),
            None => Ok(unreadable_comparison(&request)),
        },
        ReviewMode::Save => {
            match saved::resolve_saved(&repo_path, request.commit_hash.as_deref()).await {
                Ok(resolved) => {
                    let files =
                        status::saved_inventory(&repo_path, &resolved.base, &resolved.target).await;
                    Ok(ready_comparison(&request, files, Some(resolved.saved)))
                }
                Err(error) => Ok(unavailable_comparison(&request, error.code())),
            }
        }
    }
}

async fn committed_bytes(repo_path: &Path, rev: &str, path: &str) -> Option<Vec<u8>> {
    GitRunner::show_bytes(repo_path, &format!("{rev}:{path}")).await
}

async fn worktree_bytes(repo_path: &Path, path: &str) -> Option<Vec<u8>> {
    tokio::fs::read(repo_path.join(path)).await.ok()
}

async fn load_sides(
    repo_path: &Path,
    entry: &DiffFileEntry,
    base: Option<&str>,
    target: Option<&str>,
) -> (Option<Vec<u8>>, Option<Vec<u8>>) {
    let old_path = entry.old_path.as_deref().unwrap_or(&entry.path);
    let old = match base {
        Some(rev) if entry.status != DiffStatus::New => {
            committed_bytes(repo_path, rev, old_path).await
        }
        _ => None,
    };
    let new = if entry.status == DiffStatus::Deleted {
        None
    } else {
        match target {
            Some(rev) => committed_bytes(repo_path, rev, &entry.path).await,
            None => worktree_bytes(repo_path, &entry.path).await,
        }
    };
    (old, new)
}

async fn binary_side_size(
    repo_path: &Path,
    entry: &DiffFileEntry,
    base: Option<&str>,
    target: Option<&str>,
    old: bool,
) -> usize {
    if old {
        if entry.status == DiffStatus::New {
            return 0;
        }
        let path = entry.old_path.as_deref().unwrap_or(&entry.path);
        match base {
            Some(rev) => GitRunner::cat_file_size(repo_path, &format!("{rev}:{path}"))
                .await
                .unwrap_or(0) as usize,
            None => 0,
        }
    } else {
        if entry.status == DiffStatus::Deleted {
            return 0;
        }
        match target {
            Some(rev) => GitRunner::cat_file_size(repo_path, &format!("{rev}:{}", entry.path))
                .await
                .unwrap_or(0) as usize,
            None => tokio::fs::metadata(repo_path.join(&entry.path))
                .await
                .map(|metadata| metadata.len() as usize)
                .unwrap_or(0),
        }
    }
}

fn missing_preview(path: &str, old_path: Option<String>) -> FilePreview {
    FilePreview {
        path: path.to_string(),
        old_path,
        status: DiffStatus::Modified,
        kind: DiffKind::Text,
        whitespace_only: false,
        added_lines: 0,
        removed_lines: 0,
        notice: Some(PreviewNotice::Missing),
        text: Some(TextPreview {
            rows: Vec::new(),
            truncated: false,
            truncation_reason: None,
            raw: None,
            language: None,
        }),
        image: None,
        binary: None,
    }
}

/// One bounded preview for the selected file (contract C4/C5).
pub async fn review_file(request: ReviewFileRequest) -> Result<FilePreview, String> {
    let repo_path = ensure_repository(&request.path)?;

    let (files, base, target) = match request.mode {
        ReviewMode::Unsaved => {
            let files = status::unsaved_inventory_checked(&repo_path)
                .await
                .ok_or_else(|| "unreadable".to_string())?;
            let base = GitRunner::rev_parse_head(&repo_path).await;
            (files, base, None)
        }
        ReviewMode::Save => {
            let resolved = saved::resolve_saved(&repo_path, request.commit_hash.as_deref())
                .await
                .map_err(|error| error.code().to_string())?;
            let files = status::saved_inventory(&repo_path, &resolved.base, &resolved.target).await;
            (files, Some(resolved.base), Some(resolved.target))
        }
    };

    let entry = match files
        .into_iter()
        .find(|entry| entry.path == request.file_path)
    {
        Some(entry) => entry,
        // A HEAD-based saved review can move between summary and preview. A
        // summarized file that vanished explains itself instead of failing the
        // whole session (contract C5/C6).
        None if request.mode == ReviewMode::Save && request.commit_hash.is_none() => {
            return Ok(missing_preview(
                &request.file_path,
                request.old_path.clone(),
            ));
        }
        None => return Err("unknown-file".to_string()),
    };

    let mut preview = FilePreview {
        path: entry.path.clone(),
        old_path: entry.old_path.clone(),
        status: entry.status,
        kind: entry.kind,
        whitespace_only: entry.whitespace_only,
        added_lines: entry.added_lines,
        removed_lines: entry.removed_lines,
        notice: None,
        text: None,
        image: None,
        binary: None,
    };

    match entry.kind {
        DiffKind::Text => {
            let (old, new) =
                load_sides(&repo_path, &entry, base.as_deref(), target.as_deref()).await;
            let old_text = old
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .unwrap_or_default();
            let new_text = new
                .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
                .unwrap_or_default();
            let mut diff_paths = vec![entry.path.as_str()];
            if let Some(old_path) = entry.old_path.as_deref() {
                diff_paths.insert(0, old_path);
            }
            let diff_text = GitRunner::diff_unified0(
                &repo_path,
                base.as_deref(),
                target.as_deref(),
                &diff_paths,
            )
            .await
            .unwrap_or_default();
            preview.text = Some(text::build_preview(&old_text, &new_text, &diff_text));
        }
        DiffKind::Image => {
            let (old, new) =
                load_sides(&repo_path, &entry, base.as_deref(), target.as_deref()).await;
            let preview_image = image::build_preview(&entry.path, old.as_deref(), new.as_deref());
            preview.notice = match preview_image.state {
                ImageState::Ready => None,
                ImageState::TooLarge => Some(PreviewNotice::TooLarge),
                ImageState::Unsupported => Some(PreviewNotice::Unsupported),
                ImageState::Corrupt => Some(PreviewNotice::Corrupt),
                ImageState::Missing => Some(PreviewNotice::Missing),
            };
            preview.image = Some(preview_image);
        }
        DiffKind::Binary => {
            let old_bytes =
                binary_side_size(&repo_path, &entry, base.as_deref(), target.as_deref(), true)
                    .await;
            let new_bytes = binary_side_size(
                &repo_path,
                &entry,
                base.as_deref(),
                target.as_deref(),
                false,
            )
            .await;
            preview.notice = Some(PreviewNotice::Unsupported);
            preview.binary = Some(BinaryPreview {
                old_bytes,
                new_bytes,
            });
        }
    }

    Ok(preview)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn run_git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git must be installed");
        assert!(status.success(), "git {:?} failed", args);
    }

    fn init(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        run_git(dir, &["init", "-b", "main"]);
        run_git(dir, &["config", "user.name", "Test"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        fs::write(dir.join("app.js"), "const a = 1;\nconst b = 2;\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(
            dir,
            &["-c", "commit.gpgsign=false", "commit", "-m", "initial"],
        );
    }

    #[test]
    fn serializes_dtos_as_camel_case() {
        let comparison = ProjectComparison {
            path: "/tmp/theme".into(),
            name: "theme".into(),
            mode: ReviewMode::Unsaved,
            state: ProjectState::Ready,
            files: vec![DiffFileEntry {
                path: "src/app.js".into(),
                old_path: None,
                status: DiffStatus::Modified,
                kind: DiffKind::Text,
                whitespace_only: false,
                added_lines: 1,
                removed_lines: 1,
            }],
            summary: ReviewSummary {
                file_count: 1,
                added_lines: 1,
                removed_lines: 1,
            },
            saved: None,
            error: None,
        };
        let json = serde_json::to_value(&comparison).unwrap();
        assert_eq!(json["summary"]["fileCount"], 1);
        assert_eq!(json["summary"]["addedLines"], 1);
        assert_eq!(json["files"][0]["oldPath"], serde_json::Value::Null);
        assert_eq!(json["files"][0]["whitespaceOnly"], false);
        assert_eq!(json["mode"], "unsaved");
    }

    #[tokio::test]
    async fn reviews_without_touching_status_or_head() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);

        fs::write(repo.join("app.js"), "const a = 1;\nconst b = 3;\n").unwrap();
        fs::write(repo.join("new.js"), "new\n").unwrap();
        fs::write(repo.join("deleted.js"), "bye\n").unwrap();
        run_git(&repo, &["add", "deleted.js"]);
        run_git(
            &repo,
            &["-c", "commit.gpgsign=false", "commit", "-m", "add deleted"],
        );
        run_git(&repo, &["rm", "deleted.js"]);

        let before_status = GitRunner::run_git_bytes(&repo, &["status", "--porcelain=v1"])
            .await
            .unwrap();
        let before_head = GitRunner::rev_parse_head(&repo).await;

        let comparison = review_project(ReviewProjectRequest {
            path: repo.to_string_lossy().to_string(),
            mode: ReviewMode::Unsaved,
            commit_hash: None,
        })
        .await
        .unwrap();
        assert_eq!(comparison.state, ProjectState::Ready);
        assert_eq!(comparison.summary.file_count, 3);

        let preview = review_file(ReviewFileRequest {
            path: repo.to_string_lossy().to_string(),
            mode: ReviewMode::Unsaved,
            commit_hash: None,
            file_path: "app.js".into(),
            old_path: None,
        })
        .await
        .unwrap();
        assert!(preview.text.is_some());

        let after_status = GitRunner::run_git_bytes(&repo, &["status", "--porcelain=v1"])
            .await
            .unwrap();
        assert_eq!(before_status, after_status);
        assert_eq!(before_head, GitRunner::rev_parse_head(&repo).await);
    }

    #[tokio::test]
    async fn rename_preview_compares_old_and_new_content() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);

        run_git(&repo, &["mv", "app.js", "moved.js"]);
        fs::write(repo.join("moved.js"), "const a = 1;\nconst b = 3;\n").unwrap();

        let preview = review_file(ReviewFileRequest {
            path: repo.to_string_lossy().to_string(),
            mode: ReviewMode::Unsaved,
            commit_hash: None,
            file_path: "moved.js".into(),
            old_path: Some("app.js".into()),
        })
        .await
        .unwrap();

        assert_eq!(preview.status, DiffStatus::Renamed);
        assert_eq!(preview.old_path.as_deref(), Some("app.js"));
        let rows = &preview.text.unwrap().rows;
        let changed = rows.iter().find_map(|row| match row {
            DiffLine::Changed {
                old_text, new_text, ..
            } => Some((old_text.clone(), new_text.clone())),
            _ => None,
        });
        assert_eq!(
            changed,
            Some(("const b = 2;".to_string(), "const b = 3;".to_string()))
        );
    }

    #[tokio::test]
    async fn rejects_unknown_files() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);
        fs::write(repo.join("app.js"), "changed\n").unwrap();

        let result = review_file(ReviewFileRequest {
            path: repo.to_string_lossy().to_string(),
            mode: ReviewMode::Unsaved,
            commit_hash: None,
            file_path: "../outside.txt".into(),
            old_path: None,
        })
        .await;
        assert_eq!(result.unwrap_err(), "unknown-file");
    }
}
