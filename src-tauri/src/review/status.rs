//! File inventory: porcelain/name-status parsing plus the rename, conflict and
//! whitespace-only rules (data-model E3, research R1/R6).

use super::image::is_image_extension;
use super::{DiffFileEntry, DiffKind, DiffStatus};
use crate::git_runner::GitRunner;
use std::collections::HashMap;
use std::path::Path;
use tokio::io::AsyncReadExt;

const SAMPLE_BYTES: usize = 8_192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawStatusEntry {
    pub code: String,
    pub path: String,
    pub old_path: Option<String>,
    pub unmerged: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NumStat {
    pub added: usize,
    pub removed: usize,
    pub binary: bool,
}

fn is_unmerged_code(code: &str) -> bool {
    if code.len() < 2 {
        return false;
    }
    let x = code.as_bytes()[0] as char;
    let y = code.as_bytes()[1] as char;
    x == 'U' || y == 'U' || code == "AA" || code == "DD"
}

fn has_rename(code: &str) -> bool {
    code.chars()
        .any(|character| character == 'R' || character == 'C')
}

/// Parses `git status --porcelain=v1 -z`. Rename entries are `<new>\0<old>\0`.
pub fn parse_porcelain_z(bytes: &[u8]) -> Vec<RawStatusEntry> {
    let text = String::from_utf8_lossy(bytes);
    let mut tokens = text.split('\0');
    let mut entries = Vec::new();

    while let Some(token) = tokens.next() {
        if token.len() < 3 {
            continue;
        }
        let code = token[..2].to_string();
        let path = token[3..].to_string();
        if path.is_empty() {
            continue;
        }
        let old_path = if has_rename(&code) {
            tokens.next().map(str::to_string)
        } else {
            None
        };
        entries.push(RawStatusEntry {
            unmerged: is_unmerged_code(&code),
            code,
            path,
            old_path,
        });
    }

    entries
}

pub fn status_for(code: &str, unmerged: bool, operation_active: bool) -> DiffStatus {
    if unmerged && operation_active {
        return DiffStatus::Conflict;
    }
    let x = code.as_bytes().first().copied().unwrap_or(b' ') as char;
    let y = code.as_bytes().get(1).copied().unwrap_or(b' ') as char;

    if x == '?' {
        DiffStatus::New
    } else if has_rename(code) {
        DiffStatus::Renamed
    } else if x == 'A' || y == 'A' {
        DiffStatus::New
    } else if x == 'D' || y == 'D' {
        DiffStatus::Deleted
    } else {
        DiffStatus::Modified
    }
}

/// Parses `git diff --numstat -z`. A rename record has an empty path followed
/// by `<old>\0<new>\0`.
pub fn parse_numstat_z(bytes: &[u8]) -> HashMap<String, NumStat> {
    let text = String::from_utf8_lossy(bytes);
    let mut tokens = text.split('\0');
    let mut map = HashMap::new();

    while let Some(token) = tokens.next() {
        if token.is_empty() {
            continue;
        }
        let mut parts = token.splitn(3, '\t');
        let added = parts.next().unwrap_or("");
        let removed = parts.next().unwrap_or("");
        let path = parts.next().unwrap_or("");
        let binary = added == "-" || removed == "-";
        let stat = NumStat {
            added: added.parse().unwrap_or(0),
            removed: removed.parse().unwrap_or(0),
            binary,
        };

        if path.is_empty() {
            let _old = tokens.next();
            if let Some(new_path) = tokens.next() {
                if !new_path.is_empty() {
                    map.insert(new_path.to_string(), stat);
                }
            }
        } else {
            map.insert(path.to_string(), stat);
        }
    }

    map
}

/// Parses `git diff --name-status -z -M`: `R100\0<old>\0<new>\0`, otherwise
/// `<status>\0<path>\0`.
pub fn parse_name_status_z(bytes: &[u8]) -> Vec<(DiffStatus, Option<String>, String)> {
    let text = String::from_utf8_lossy(bytes);
    let mut tokens = text.split('\0');
    let mut entries = Vec::new();

    while let Some(token) = tokens.next() {
        if token.is_empty() {
            continue;
        }
        let letter = token.chars().next().unwrap_or('M');
        if letter == 'R' || letter == 'C' {
            let old_path = tokens.next().unwrap_or("").to_string();
            let new_path = tokens.next().unwrap_or("").to_string();
            if new_path.is_empty() {
                continue;
            }
            let status = if letter == 'R' {
                DiffStatus::Renamed
            } else {
                DiffStatus::New
            };
            entries.push((
                status,
                if letter == 'R' { Some(old_path) } else { None },
                new_path,
            ));
        } else {
            let path = tokens.next().unwrap_or("").to_string();
            if path.is_empty() {
                continue;
            }
            let status = match letter {
                'A' => DiffStatus::New,
                'D' => DiffStatus::Deleted,
                _ => DiffStatus::Modified,
            };
            entries.push((status, None, path));
        }
    }

    entries
}

pub fn classify_kind(path: &str, stat: Option<&NumStat>, sample: &[u8]) -> DiffKind {
    if is_image_extension(path) {
        DiffKind::Image
    } else if stat.map(|value| value.binary).unwrap_or(false) || sample.contains(&0) {
        DiffKind::Binary
    } else {
        DiffKind::Text
    }
}

async fn read_sample(path: &Path) -> Vec<u8> {
    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return Vec::new();
    };
    let mut buffer = vec![0u8; SAMPLE_BYTES];
    match file.read(&mut buffer).await {
        Ok(read) => {
            buffer.truncate(read);
            buffer
        }
        Err(_) => Vec::new(),
    }
}

async fn blob_sample(repo_path: &Path, rev: &str, path: &str) -> Vec<u8> {
    let spec = format!("{rev}:{path}");
    match GitRunner::show_bytes(repo_path, &spec).await {
        Some(mut bytes) => {
            bytes.truncate(SAMPLE_BYTES);
            bytes
        }
        None => Vec::new(),
    }
}

fn zero_for_binary(stat: Option<&NumStat>) -> (usize, usize) {
    match stat {
        Some(stat) if !stat.binary => (stat.added, stat.removed),
        _ => (0, 0),
    }
}

#[allow(dead_code)]
pub async fn unsaved_inventory(repo_path: &Path) -> Vec<DiffFileEntry> {
    unsaved_inventory_checked(repo_path)
        .await
        .unwrap_or_default()
}

/// `None` means the repository could not be read at all (contract C6).
pub async fn unsaved_inventory_checked(repo_path: &Path) -> Option<Vec<DiffFileEntry>> {
    let porcelain = GitRunner::status_porcelain_z(repo_path).await?;
    Some(unsaved_inventory_from(repo_path, porcelain).await)
}

async fn unsaved_inventory_from(repo_path: &Path, porcelain: Vec<u8>) -> Vec<DiffFileEntry> {
    let raw = parse_porcelain_z(&porcelain);
    if raw.is_empty() {
        return Vec::new();
    }

    let operation = GitRunner::operation_state(repo_path);
    let operation_active = matches!(
        operation.as_str(),
        "merging" | "rebasing" | "cherry-picking"
    );
    let head = GitRunner::rev_parse_head(repo_path).await;
    let base = head.as_deref();
    let numstat = GitRunner::diff_numstat_z(repo_path, base, None)
        .await
        .map(|bytes| parse_numstat_z(&bytes))
        .unwrap_or_default();

    let mut entries = Vec::with_capacity(raw.len());
    for item in raw {
        let status = status_for(&item.code, item.unmerged, operation_active);
        let stat = numstat.get(&item.path);
        let sample = match (&status, &item.old_path) {
            (DiffStatus::Deleted, _) => match base {
                Some(rev) => blob_sample(repo_path, rev, &item.path).await,
                None => Vec::new(),
            },
            _ => read_sample(&repo_path.join(&item.path)).await,
        };
        let kind = classify_kind(&item.path, stat, &sample);
        let whitespace_only = status == DiffStatus::Modified
            && kind == DiffKind::Text
            && GitRunner::diff_quiet_ignore_space(repo_path, base, None, &item.path).await;
        let (added_lines, removed_lines) = zero_for_binary(stat);

        entries.push(DiffFileEntry {
            path: item.path,
            old_path: item.old_path,
            status,
            kind,
            whitespace_only,
            added_lines,
            removed_lines,
        });
    }

    entries.sort_by_key(|entry| entry.path.to_lowercase());
    entries
}

pub async fn saved_inventory(repo_path: &Path, base: &str, target: &str) -> Vec<DiffFileEntry> {
    let name_status = GitRunner::diff_name_status_z(repo_path, Some(base), Some(target))
        .await
        .unwrap_or_default();
    let raw = parse_name_status_z(&name_status);
    if raw.is_empty() {
        return Vec::new();
    }
    let numstat = GitRunner::diff_numstat_z(repo_path, Some(base), Some(target))
        .await
        .map(|bytes| parse_numstat_z(&bytes))
        .unwrap_or_default();

    let mut entries = Vec::with_capacity(raw.len());
    for (status, old_path, path) in raw {
        let stat = numstat.get(&path);
        let new_sample = match status {
            DiffStatus::Deleted => Vec::new(),
            _ => blob_sample(repo_path, target, &path).await,
        };
        let old_sample = match &old_path {
            Some(old) => blob_sample(repo_path, base, old).await,
            None => blob_sample(repo_path, base, &path).await,
        };
        let sample = if new_sample.is_empty() {
            old_sample
        } else {
            new_sample
        };
        let kind = classify_kind(&path, stat, &sample);
        let whitespace_only = status == DiffStatus::Modified
            && kind == DiffKind::Text
            && GitRunner::diff_quiet_ignore_space(repo_path, Some(base), Some(target), &path).await;
        let (added_lines, removed_lines) = zero_for_binary(stat);

        entries.push(DiffFileEntry {
            path,
            old_path,
            status,
            kind,
            whitespace_only,
            added_lines,
            removed_lines,
        });
    }

    entries.sort_by_key(|entry| entry.path.to_lowercase());
    entries
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
        fs::write(dir.join("tracked.txt"), "one\ntwo\n").unwrap();
        fs::write(dir.join("gone.txt"), "bye\n").unwrap();
        fs::write(dir.join("move-me.txt"), "one\ntwo\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(
            dir,
            &["-c", "commit.gpgsign=false", "commit", "-m", "initial"],
        );
    }

    #[test]
    fn parses_rename_pairs_from_porcelain() {
        let raw = b"R  new.txt\0old.txt\0?? fresh.txt\0";
        let entries = parse_porcelain_z(raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "new.txt");
        assert_eq!(entries[0].old_path.as_deref(), Some("old.txt"));
        assert_eq!(entries[1].path, "fresh.txt");
        assert!(entries[1].old_path.is_none());
    }

    #[test]
    fn conflict_label_requires_an_active_operation() {
        assert_eq!(status_for("UU", true, true), DiffStatus::Conflict);
        assert_eq!(status_for("UU", true, false), DiffStatus::Modified);
        assert_eq!(status_for("AA", true, true), DiffStatus::Conflict);
    }

    #[tokio::test]
    async fn inventories_modified_new_deleted_and_renamed_files() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);

        fs::write(repo.join("tracked.txt"), "one\nchanged\n").unwrap();
        fs::write(repo.join("fresh.txt"), "new\n").unwrap();
        run_git(&repo, &["rm", "gone.txt"]);
        run_git(&repo, &["mv", "move-me.txt", "moved.txt"]);
        fs::write(repo.join("ignored.log"), "ignored\n").unwrap();
        fs::write(repo.join(".gitignore"), "*.log\n").unwrap();

        let entries = unsaved_inventory(&repo).await;
        let find = |path: &str| entries.iter().find(|entry| entry.path == path).cloned();

        assert_eq!(find("tracked.txt").unwrap().status, DiffStatus::Modified);
        assert_eq!(find("fresh.txt").unwrap().status, DiffStatus::New);
        assert_eq!(find("gone.txt").unwrap().status, DiffStatus::Deleted);
        let moved = find("moved.txt").unwrap();
        assert_eq!(moved.status, DiffStatus::Renamed);
        assert_eq!(moved.old_path.as_deref(), Some("move-me.txt"));
        assert!(find("ignored.log").is_none());
    }

    #[tokio::test]
    async fn inventory_leaves_head_and_status_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init(&repo);
        fs::write(repo.join("tracked.txt"), "one\nchanged\n").unwrap();

        let before_status = GitRunner::run_git_bytes(&repo, &["status", "--porcelain=v1"])
            .await
            .unwrap();
        let before_head = GitRunner::rev_parse_head(&repo).await;

        let _ = unsaved_inventory(&repo).await;

        let after_status = GitRunner::run_git_bytes(&repo, &["status", "--porcelain=v1"])
            .await
            .unwrap();
        assert_eq!(before_status, after_status);
        assert_eq!(before_head, GitRunner::rev_parse_head(&repo).await);
    }
}
