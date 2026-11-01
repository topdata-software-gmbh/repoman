//! Bounded, read-only repository inspection tools (US4, research R3).
//!
//! Every lookup goes through the read-only `GitRunner` helpers (Principle III)
//! or a plain `std::fs` read, is sandboxed to the repository root (FR-011), and
//! claims one unit from the per-attempt budget carried by [`RepoCtx`] (FR-010).
//! Nothing here can write to a repository (Principle V).

use crate::commitgen::agent::RepoCtx;
use crate::git_runner::GitRunner;
use serde_json::{json, Value};
use serdes_ai::agent::{RunContext, ToolExecutor};
use serdes_ai::tools::{ToolDefinition, ToolError, ToolReturn};
use std::path::{Component, Path, PathBuf};

/// Cap for any single tool answer, so a huge file cannot flood the context.
pub const MAX_TOOL_OUTPUT_CHARS: usize = 20_000;
/// Matches returned by one `grep` call.
pub const MAX_GREP_MATCHES: usize = 40;
/// Files visited by one `grep` call.
pub const MAX_GREP_FILES: usize = 500;
/// Upper bound for `recent_history`.
pub const MAX_HISTORY: usize = 20;

/// Directories never worth searching for a commit-message hint.
const SKIPPED_DIRS: [&str; 8] = [
    ".git",
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    ".cache",
    "var",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    ReadFile,
    FileAtHead,
    RecentHistory,
    Grep,
}

impl ToolKind {
    fn name(self) -> &'static str {
        match self {
            Self::ReadFile => "read_file",
            Self::FileAtHead => "file_at_head",
            Self::RecentHistory => "recent_history",
            Self::Grep => "grep",
        }
    }

    fn definition(self) -> ToolDefinition {
        let path_only = json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"],
            "additionalProperties": false
        });

        match self {
            Self::ReadFile => ToolDefinition::new(
                self.name(),
                "Read the current working-tree content of one repository file (repo-relative path).",
            )
            .with_parameters(path_only),
            Self::FileAtHead => ToolDefinition::new(
                self.name(),
                "Read one repository file as it exists at HEAD (repo-relative path).",
            )
            .with_parameters(path_only),
            Self::RecentHistory => ToolDefinition::new(
                self.name(),
                "Read the most recent commit subjects for style alignment (1-20, default 10).",
            )
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "count": { "type": "integer", "minimum": 1, "maximum": MAX_HISTORY }
                },
                "additionalProperties": false
            })),
            Self::Grep => ToolDefinition::new(
                self.name(),
                "Case-insensitive text search inside the repository; optionally limited to one repo-relative subpath.",
            )
            .with_parameters(json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "path": { "type": "string" }
                },
                "required": ["query"],
                "additionalProperties": false
            })),
        }
    }
}

/// One executable inspection tool.
#[derive(Debug, Clone, Copy)]
pub struct RepoTool {
    kind: ToolKind,
}

impl RepoTool {
    pub fn new(kind: ToolKind) -> Self {
        Self { kind }
    }
}

/// Definitions + executors for all four tools, ready for `tool_with_executor`.
pub fn tool_executors() -> Vec<(ToolDefinition, RepoTool)> {
    [
        ToolKind::ReadFile,
        ToolKind::FileAtHead,
        ToolKind::RecentHistory,
        ToolKind::Grep,
    ]
    .into_iter()
    .map(|kind| (kind.definition(), RepoTool::new(kind)))
    .collect()
}

fn refused(path: &str) -> ToolError {
    ToolError::ExecutionFailed {
        message: format!(
            "path {path:?} is outside this repository; only repo-relative paths are allowed"
        ),
        retryable: false,
    }
}

fn failed(message: impl Into<String>) -> ToolError {
    ToolError::ExecutionFailed {
        message: message.into(),
        retryable: false,
    }
}

fn cap(text: String) -> String {
    if text.chars().count() <= MAX_TOOL_OUTPUT_CHARS {
        return text;
    }
    let mut capped: String = text.chars().take(MAX_TOOL_OUTPUT_CHARS).collect();
    capped.push_str("\n[... truncated ...]");
    capped
}

/// Rejects absolute paths and `..` components, then resolves symlinks and
/// verifies the result still lives under the repository root (FR-011).
fn resolve_existing(root: &Path, candidate: &str) -> Result<PathBuf, ToolError> {
    let relative = Path::new(candidate);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| part == Component::ParentDir)
    {
        return Err(refused(candidate));
    }

    let joined = root.join(relative);
    let canonical = std::fs::canonicalize(&joined)
        .map_err(|_| failed(format!("cannot read {candidate:?} in this repository")))?;
    let canonical_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    if !canonical.starts_with(&canonical_root) {
        return Err(refused(candidate));
    }
    Ok(canonical)
}

/// Lexical sandbox check for paths that need not exist on disk (HEAD lookups).
fn relative_inside(candidate: &str) -> Result<String, ToolError> {
    let relative = Path::new(candidate);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| part == Component::ParentDir)
        || candidate.trim().is_empty()
    {
        return Err(refused(candidate));
    }
    Ok(candidate.to_string())
}

fn string_arg(args: &Value, key: &str) -> Result<String, ToolError> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| failed(format!("missing required argument {key:?}")))
}

async fn read_file(root: &Path, candidate: &str) -> Result<String, ToolError> {
    let path = resolve_existing(root, candidate)?;
    let bytes = std::fs::read(&path).map_err(|_| failed(format!("cannot read {candidate:?}")))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok(cap(format!("--- {candidate} ---\n{text}")))
}

async fn file_at_head(root: &Path, candidate: &str) -> Result<String, ToolError> {
    let relative = relative_inside(candidate)?;
    match GitRunner::show_head_file(root, &relative).await {
        Some(text) => Ok(cap(format!("--- HEAD:{relative} ---\n{text}"))),
        None => Err(failed(format!("{candidate:?} does not exist at HEAD"))),
    }
}

async fn recent_history(root: &Path, args: &Value) -> Result<String, ToolError> {
    let count = args
        .get("count")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .unwrap_or(10)
        .clamp(1, MAX_HISTORY);
    let subjects = GitRunner::recent_subjects(root, count).await;
    if subjects.is_empty() {
        return Ok("(no commits yet)".to_string());
    }
    Ok(cap(subjects.join("\n")))
}

async fn grep(root: &Path, args: &Value) -> Result<String, ToolError> {
    let query = string_arg(args, "query")?;
    if query.trim().is_empty() {
        return Err(failed("query must not be empty"));
    }
    let needle = query.to_lowercase();

    let start = match args.get("path").and_then(Value::as_str) {
        Some(subpath) => resolve_existing(root, subpath)?,
        None => root.to_path_buf(),
    };

    let mut matches = Vec::new();
    let mut visited = 0_usize;
    walk(&start, root, &needle, &mut matches, &mut visited);

    if matches.is_empty() {
        return Ok(format!("no matches for {query:?}"));
    }
    Ok(cap(matches.join("\n")))
}

fn walk(dir: &Path, root: &Path, needle: &str, matches: &mut Vec<String>, visited: &mut usize) {
    if matches.len() >= MAX_GREP_MATCHES || *visited >= MAX_GREP_FILES {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|entry| entry.path()).collect();
    paths.sort();

    for path in paths {
        if matches.len() >= MAX_GREP_MATCHES || *visited >= MAX_GREP_FILES {
            return;
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if path.is_dir() {
            if SKIPPED_DIRS.contains(&name) {
                continue;
            }
            walk(&path, root, needle, matches, visited);
            continue;
        }
        if path
            .symlink_metadata()
            .map(|meta| meta.is_symlink())
            .unwrap_or(true)
        {
            continue;
        }
        if path
            .metadata()
            .map(|meta| meta.len() > 512 * 1024)
            .unwrap_or(true)
        {
            continue;
        }

        *visited += 1;
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
        for (index, line) in text.lines().enumerate() {
            if matches.len() >= MAX_GREP_MATCHES {
                return;
            }
            if line.to_lowercase().contains(needle) {
                let trimmed: String = line.chars().take(200).collect();
                matches.push(format!("{relative}:{}: {trimmed}", index + 1));
            }
        }
    }
}

#[async_trait::async_trait]
impl ToolExecutor<RepoCtx> for RepoTool {
    async fn execute(
        &self,
        args: Value,
        ctx: &RunContext<RepoCtx>,
    ) -> Result<ToolReturn, ToolError> {
        let deps = ctx.deps.as_ref();
        if !deps.claim_lookup(deps.lookup_limit) {
            return Err(failed(
                "lookup budget exhausted; answer from the change detail you already have",
            ));
        }

        let root = deps.path.as_path();
        let answer = match self.kind {
            ToolKind::ReadFile => read_file(root, &string_arg(&args, "path")?).await,
            ToolKind::FileAtHead => file_at_head(root, &string_arg(&args, "path")?).await,
            ToolKind::RecentHistory => recent_history(root, &args).await,
            ToolKind::Grep => grep(root, &args).await,
        }?;

        Ok(ToolReturn::text(answer))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serdes_ai::agent::RunContext;
    use std::process::Command;

    fn git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .status()
            .expect("git available");
        assert!(status.success(), "git {args:?} failed");
    }

    fn repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("repo");
        std::fs::create_dir_all(&path).expect("repo dir");
        git(&path, &["init", "-q"]);
        git(&path, &["config", "user.email", "test@example.com"]);
        git(&path, &["config", "user.name", "Test"]);
        std::fs::write(
            path.join("composer.json"),
            "{\"require\": {\"foo/bar\": \"^1.0\"}}\n",
        )
        .expect("write");
        git(&path, &["add", "."]);
        git(&path, &["commit", "-q", "-m", "feat: initial import"]);
        std::fs::write(
            path.join("composer.json"),
            "{\"require\": {\"foo/bar\": \"^1.2\"}}\n",
        )
        .expect("write change");
        (dir, path)
    }

    fn context(path: &Path, limit: u32) -> RunContext<RepoCtx> {
        RunContext::new(
            RepoCtx::new(path.to_path_buf(), "repo").with_lookup_limit(limit),
            "test-model",
        )
    }

    #[tokio::test]
    async fn read_file_returns_the_working_tree_content() {
        let (_dir, path) = repo();
        let ctx = context(&path, 12);
        let tool = RepoTool::new(ToolKind::ReadFile);

        let answer = tool
            .execute(json!({ "path": "composer.json" }), &ctx)
            .await
            .expect("read ok");

        assert!(format!("{answer:?}").contains("^1.2"));
    }

    #[tokio::test]
    async fn file_at_head_returns_the_committed_content() {
        let (_dir, path) = repo();
        let ctx = context(&path, 12);
        let tool = RepoTool::new(ToolKind::FileAtHead);

        let answer = tool
            .execute(json!({ "path": "composer.json" }), &ctx)
            .await
            .expect("head read ok");

        assert!(format!("{answer:?}").contains("^1.0"));
        assert!(!format!("{answer:?}").contains("^1.2"));
    }

    #[tokio::test]
    async fn recent_history_returns_commit_subjects() {
        let (_dir, path) = repo();
        let ctx = context(&path, 12);
        let tool = RepoTool::new(ToolKind::RecentHistory);

        let answer = tool
            .execute(json!({ "count": 5 }), &ctx)
            .await
            .expect("history ok");

        assert!(format!("{answer:?}").contains("feat: initial import"));
    }

    #[tokio::test]
    async fn grep_finds_text_and_skips_vcs_metadata() {
        let (_dir, path) = repo();
        let ctx = context(&path, 12);
        let tool = RepoTool::new(ToolKind::Grep);

        let answer = tool
            .execute(json!({ "query": "FOO/BAR" }), &ctx)
            .await
            .expect("grep ok");

        let text = format!("{answer:?}");
        assert!(text.contains("composer.json"));
        assert!(!text.contains(".git/"));
    }

    #[tokio::test]
    async fn paths_outside_the_repository_are_refused_without_leaking() {
        let (dir, path) = repo();
        std::fs::write(dir.path().join("secret.txt"), "TOP-SECRET").expect("secret");
        std::os::unix::fs::symlink(dir.path().join("secret.txt"), path.join("link.txt"))
            .expect("symlink");

        let ctx = context(&path, 12);
        let tool = RepoTool::new(ToolKind::ReadFile);

        for candidate in ["../secret.txt", "/etc/hosts", "link.txt"] {
            let error = tool
                .execute(json!({ "path": candidate }), &ctx)
                .await
                .expect_err("must be refused");
            let text = format!("{error:?}");
            assert!(!text.contains("TOP-SECRET"), "{candidate} leaked: {text}");
        }

        let head = RepoTool::new(ToolKind::FileAtHead);
        assert!(head
            .execute(json!({ "path": "../secret.txt" }), &ctx)
            .await
            .is_err());
    }

    #[tokio::test]
    async fn the_lookup_budget_is_enforced_per_attempt() {
        let (_dir, path) = repo();
        let ctx = context(&path, 2);
        let tool = RepoTool::new(ToolKind::ReadFile);

        for _ in 0..2 {
            tool.execute(json!({ "path": "composer.json" }), &ctx)
                .await
                .expect("within budget");
        }

        let error = tool
            .execute(json!({ "path": "composer.json" }), &ctx)
            .await
            .expect_err("budget exhausted");
        assert!(format!("{error:?}").to_lowercase().contains("budget"));
        assert_eq!(ctx.deps.lookups_used(), 2);
    }

    #[test]
    fn every_tool_has_a_name_and_a_parameter_schema() {
        let tools = tool_executors();
        assert_eq!(tools.len(), 4);
        let names: Vec<_> = tools
            .iter()
            .map(|(definition, _)| definition.name.clone())
            .collect();
        assert_eq!(
            names,
            vec!["read_file", "file_at_head", "recent_history", "grep"]
        );
        assert!(tools
            .iter()
            .all(|(definition, _)| definition.parameters_json_schema.is_object()));
    }
}
