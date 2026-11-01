use crate::git_log::{GitObserver, GitStepEvent, GitStepStatus, RunContext};
use crate::macros::{self, Guard, GuardState, MacroContext, Require};
use crate::models::{FileChange, GitRepoState, OperationResult, RemoteInfo};
use std::path::{Path, PathBuf};
use std::process::Output;
use tokio::process::Command;

pub struct GitRunner;

impl GitRunner {
    async fn run_git_cmd(repo_path: &Path, args: &[&str]) -> Result<Output, String> {
        Command::new("git")
            .current_dir(repo_path)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to execute git {:?}: {}", args, e))
    }

    async fn run_git_stdout(repo_path: &Path, args: &[&str]) -> Option<String> {
        match Self::run_git_cmd(repo_path, args).await {
            Ok(out) if out.status.success() => {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            }
            _ => None,
        }
    }

    /// Resolves the real git directory. Submodules and linked worktrees use a
    /// `.git` *file* pointing at a directory elsewhere.
    pub fn resolve_git_dir(repo_path: &Path) -> PathBuf {
        let dot_git = repo_path.join(".git");
        if dot_git.is_file() {
            if let Ok(contents) = std::fs::read_to_string(&dot_git) {
                if let Some(target) = contents.trim().strip_prefix("gitdir:") {
                    let target = PathBuf::from(target.trim());
                    return if target.is_absolute() {
                        target
                    } else {
                        repo_path.join(target)
                    };
                }
            }
        }
        dot_git
    }

    pub async fn get_repo_state(repo_path: &Path) -> GitRepoState {
        let (raw_branch, (modified, untracked, staged, conflicts), (hash, message), remotes) = tokio::join!(
            Self::get_current_branch(repo_path),
            Self::parse_status(repo_path),
            Self::get_head_commit(repo_path),
            Self::get_remotes(repo_path),
        );

        let is_detached_head = raw_branch == "HEAD" || raw_branch.is_empty();
        let current_branch = if is_detached_head {
            "HEAD (detached)".to_string()
        } else {
            raw_branch
        };
        let (ahead, behind) = if is_detached_head {
            (0, 0)
        } else {
            Self::get_ahead_behind(repo_path, &current_branch).await
        };

        GitRepoState {
            current_branch,
            is_detached_head,
            head_commit_hash: hash,
            head_commit_message: message,
            is_clean: modified == 0 && untracked == 0 && staged == 0 && !conflicts,
            ahead_count: ahead,
            behind_count: behind,
            modified_files: modified,
            untracked_files: untracked,
            staged_files: staged,
            has_conflicts: conflicts,
            operation_state: Self::detect_operation_state(repo_path),
            remotes,
        }
    }

    async fn get_current_branch(path: &Path) -> String {
        if let Some(name) = Self::run_git_stdout(path, &["rev-parse", "--abbrev-ref", "HEAD"]).await
        {
            if !name.is_empty() {
                return name;
            }
        }
        "HEAD".to_string()
    }

    async fn get_ahead_behind(path: &Path, branch: &str) -> (usize, usize) {
        let upstream = Self::run_git_stdout(
            path,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        )
        .await
        .filter(|upstream| !upstream.is_empty())
        .unwrap_or_else(|| format!("origin/{}", branch));

        let range = format!("{}...HEAD", upstream);
        if let Some(output) =
            Self::run_git_stdout(path, &["rev-list", "--left-right", "--count", &range]).await
        {
            let parts: Vec<&str> = output.split_whitespace().collect();
            if parts.len() == 2 {
                let behind = parts[0].parse::<usize>().unwrap_or(0);
                let ahead = parts[1].parse::<usize>().unwrap_or(0);
                return (ahead, behind);
            }
        }
        (0, 0)
    }

    async fn parse_status(path: &Path) -> (usize, usize, usize, bool) {
        let mut modified = 0;
        let mut untracked = 0;
        let mut staged = 0;
        let mut has_conflicts = false;

        if let Ok(out) = Self::run_git_cmd(path, &["status", "--porcelain=v1"]).await {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if line.len() < 2 {
                    continue;
                }
                let bytes = line.as_bytes();
                let x = bytes[0] as char;
                let y = bytes[1] as char;

                if x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D') {
                    has_conflicts = true;
                }
                if x == '?' && y == '?' {
                    untracked += 1;
                } else {
                    if x != ' ' && x != '?' {
                        staged += 1;
                    }
                    if y != ' ' && y != '?' {
                        modified += 1;
                    }
                }
            }
        }
        (modified, untracked, staged, has_conflicts)
    }

    async fn get_head_commit(path: &Path) -> (String, String) {
        if let Some(output) = Self::run_git_stdout(path, &["log", "-1", "--format=%h%x09%s"]).await
        {
            let mut parts = output.splitn(2, '\t');
            let hash = parts.next().unwrap_or("").to_string();
            let message = parts.next().unwrap_or("").to_string();
            return (hash, message);
        }
        (String::new(), String::new())
    }

    async fn get_remotes(path: &Path) -> Vec<RemoteInfo> {
        let mut list = Vec::new();
        if let Some(output) = Self::run_git_stdout(path, &["remote", "-v"]).await {
            for line in output.lines() {
                if line.ends_with("(fetch)") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        list.push(RemoteInfo {
                            name: parts[0].to_string(),
                            url: parts[1].to_string(),
                        });
                    }
                }
            }
        }
        list
    }

    fn detect_operation_state(path: &Path) -> String {
        let git_dir = Self::resolve_git_dir(path);
        if git_dir.join("MERGE_HEAD").exists() {
            "merging".to_string()
        } else if git_dir.join("rebase-apply").exists() || git_dir.join("rebase-merge").exists() {
            "rebasing".to_string()
        } else if git_dir.join("CHERRY_PICK_HEAD").exists() {
            "cherry-picking".to_string()
        } else {
            "idle".to_string()
        }
    }

    pub async fn list_local_branches(repo_path: &Path) -> Vec<String> {
        Self::run_git_stdout(
            repo_path,
            &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        )
        .await
        .map(|output| {
            output
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default()
    }

    pub async fn list_changes(repo_path: &Path) -> Vec<FileChange> {
        match Self::run_git_cmd(
            repo_path,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )
        .await
        {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(parse_file_change)
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether the working tree has anything not yet recorded in Git.
    pub(crate) async fn is_dirty(repo_path: &Path) -> bool {
        Self::run_git_stdout(repo_path, &["status", "--porcelain=v1"])
            .await
            .is_some_and(|output| !output.trim().is_empty())
    }

    /// Read-only: the exact change set a commit would take (`git diff HEAD`).
    /// `None` means the command failed — typically an unborn HEAD.
    pub async fn diff_head(repo_path: &Path) -> Option<String> {
        Self::run_git_stdout(repo_path, &["diff", "HEAD", "--no-color"]).await
    }

    /// Read-only: every untracked file, descending into directories
    /// (`status --porcelain=v1 --untracked-files=all`).
    pub async fn untracked_files(repo_path: &Path) -> Vec<String> {
        match Self::run_git_cmd(
            repo_path,
            &["status", "--porcelain=v1", "--untracked-files=all"],
        )
        .await
        {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| line.starts_with("?? "))
                .map(|line| line[3..].trim().to_string())
                .filter(|path| !path.is_empty())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Read-only: a file as recorded in the current HEAD (`git show HEAD:<file>`).
    pub async fn show_head_file(repo_path: &Path, relative_path: &str) -> Option<String> {
        Self::run_git_stdout(repo_path, &["show", &format!("HEAD:{relative_path}")]).await
    }

    /// Read-only: the last `count` commit subjects for message-style alignment.
    pub async fn recent_subjects(repo_path: &Path, count: usize) -> Vec<String> {
        let limit = count.max(1).to_string();
        Self::run_git_stdout(repo_path, &["log", "-n", &limit, "--format=%s"])
            .await
            .map(|output| {
                output
                    .lines()
                    .map(|line| line.trim().to_string())
                    .filter(|line| !line.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Read-only: unmerged paths in the porcelain status (same codes as `parse_status`).
    pub async fn has_conflicts(repo_path: &Path) -> bool {
        match Self::run_git_cmd(repo_path, &["status", "--porcelain=v1"]).await {
            Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| line.len() >= 2)
                .any(|line| {
                    let x = line.as_bytes()[0] as char;
                    let y = line.as_bytes()[1] as char;
                    x == 'U' || y == 'U' || (x == 'A' && y == 'A') || (x == 'D' && y == 'D')
                }),
            _ => false,
        }
    }

    pub(crate) async fn has_upstream(repo_path: &Path) -> bool {
        Self::run_git_stdout(
            repo_path,
            &[
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ],
        )
        .await
        .is_some()
    }

    /// Executes a declared macro and reports every step transition.
    pub async fn execute_macro(
        repo_path: &Path,
        macro_id: &str,
        ctx: &MacroContext<'_>,
        observer: &dyn GitObserver,
    ) -> OperationResult {
        let (macros, aliases) = match macros::template() {
            Ok(loaded) => loaded,
            Err(error) => {
                log::error!("git action template unusable: {error}");
                return failure_result(
                    repo_path,
                    "Git actions are not available in this build. \
                     Restart the application; if it persists, reinstall it.",
                    String::new(),
                );
            }
        };

        let Some(mac) = macros.macros.get(macro_id) else {
            return failure_result(
                repo_path,
                format!("This action is not available: {macro_id}"),
                String::new(),
            );
        };

        if mac.require == Require::OnBranch {
            let branch = Self::get_current_branch(repo_path).await;
            if branch == "HEAD" || branch.is_empty() {
                return failure_result(
                    repo_path,
                    "You are not on a branch. Switch to a branch before using this action.",
                    String::new(),
                );
            }
        }

        let mut guards = GuardState {
            auto_stash: ctx.auto_stash,
            ..GuardState::default()
        };
        if mac.steps.iter().any(|step| {
            step.when
                .iter()
                .any(|guard| matches!(guard, Guard::Dirty | Guard::Clean))
        }) {
            guards.dirty = Self::is_dirty(repo_path).await;
        }
        if mac.steps.iter().any(|step| {
            step.when
                .iter()
                .any(|guard| matches!(guard, Guard::HasUpstream | Guard::NoUpstream))
        }) {
            guards.has_upstream = Self::has_upstream(repo_path).await;
        }

        let run_id = uuid::Uuid::new_v4().to_string();
        let run = RunContext {
            run_id: &run_id,
            repo_path,
            macro_id,
            title: &mac.title,
        };
        let mut output = String::new();
        let mut failure: Option<String> = None;

        for (index, step) in mac.steps.iter().enumerate() {
            let args = match macros::render_args(&step.args, ctx) {
                Ok(args) => args,
                Err(error) => {
                    return failure_result(repo_path, error, String::new());
                }
            };
            let command = macros::render_command(repo_path, &args);
            let alias = step.alias.clone().or_else(|| aliases.hint_for(&step.args));
            let base = GitStepEvent::new(
                &run,
                index,
                &step.comment,
                &command,
                GitStepStatus::Running,
                alias,
            );

            if !guards.allows(&step.when) {
                observer.on_step(&GitStepEvent {
                    status: GitStepStatus::Skipped,
                    ..base
                });
                continue;
            }

            observer.on_step(&base);
            let started = std::time::Instant::now();
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

            match Self::run_git_cmd(repo_path, &arg_refs).await {
                Ok(result) if result.status.success() => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    let text = output_text(&result);
                    if !text.is_empty() {
                        if !output.is_empty() {
                            output.push('\n');
                        }
                        output.push_str(&text);
                    }
                    if step.sets == macros::Effect::Stashed {
                        guards.stashed = true;
                    }
                    observer.on_step(&GitStepEvent {
                        status: GitStepStatus::Ok,
                        duration_ms,
                        ..base
                    });
                }
                Ok(result) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    let error = friendly_error(&error_text(&result));
                    observer.on_step(&GitStepEvent {
                        status: GitStepStatus::Failed,
                        duration_ms,
                        message: Some(error.clone()),
                        ..base
                    });
                    failure = Some(error);
                    Self::report_remaining_skipped(
                        &run,
                        &mac.steps[index + 1..],
                        index + 1,
                        observer,
                    );
                    break;
                }
                Err(error) => {
                    let duration_ms = started.elapsed().as_millis() as u64;
                    observer.on_step(&GitStepEvent {
                        status: GitStepStatus::Failed,
                        duration_ms,
                        message: Some(error.clone()),
                        ..base
                    });
                    failure = Some(error);
                    Self::report_remaining_skipped(
                        &run,
                        &mac.steps[index + 1..],
                        index + 1,
                        observer,
                    );
                    break;
                }
            }
        }

        match failure {
            Some(error) => failure_result(repo_path, error, output),
            None => OperationResult {
                repo_path: repo_path.to_string_lossy().to_string(),
                success: true,
                output,
                error: None,
            },
        }
    }

    fn report_remaining_skipped(
        run: &RunContext<'_>,
        remaining: &[macros::GitStep],
        offset: usize,
        observer: &dyn GitObserver,
    ) {
        for (index, step) in remaining.iter().enumerate() {
            observer.on_step(&GitStepEvent::new(
                run,
                offset + index,
                &step.comment,
                &macros::render_command(run.repo_path, &step.args),
                GitStepStatus::Skipped,
                step.alias.clone(),
            ));
        }
    }

    pub async fn commit_changes(
        repo_path: &Path,
        message: &str,
        observer: &dyn GitObserver,
    ) -> OperationResult {
        let message = message.trim();
        if message.is_empty() {
            return failure_result(
                repo_path,
                "Write a short message that describes what you changed.",
                String::new(),
            );
        }

        let changes = Self::list_changes(repo_path).await;
        if changes.is_empty() {
            return failure_result(repo_path, "There are no changes to save.", String::new());
        }
        if changes.iter().any(|change| change.status == "conflict") {
            return failure_result(
                repo_path,
                "Some files have conflicts. Open the project in your editor, fix the marked \
                 files, then save to Git again.",
                String::new(),
            );
        }

        let ctx = MacroContext {
            message: Some(message),
            branch: None,
            repo_path,
            auto_stash: true,
        };
        Self::execute_macro(repo_path, "commit", &ctx, observer).await
    }

    pub async fn checkout_branch(
        repo_path: &Path,
        branch: &str,
        auto_stash: bool,
        observer: &dyn GitObserver,
    ) -> OperationResult {
        let current_branch = Self::get_current_branch(repo_path).await;
        if current_branch == branch {
            return OperationResult {
                repo_path: repo_path.to_string_lossy().to_string(),
                success: true,
                output: format!("Already on branch '{}'.", branch),
                error: None,
            };
        }

        let ctx = MacroContext {
            message: None,
            branch: Some(branch),
            repo_path,
            auto_stash,
        };
        Self::execute_macro(repo_path, "switch_branch", &ctx, observer).await
    }

    pub async fn execute_action(
        repo_path: &Path,
        action: &str,
        observer: &dyn GitObserver,
    ) -> OperationResult {
        let macro_id = match action {
            "fetch" | "pull" | "push" | "stash" | "stash_pop" => action,
            other => {
                return failure_result(repo_path, format!("Unknown action: {other}"), String::new())
            }
        };

        let branch = Self::get_current_branch(repo_path).await;
        let ctx = MacroContext {
            message: None,
            branch: Some(&branch),
            repo_path,
            auto_stash: true,
        };
        Self::execute_macro(repo_path, macro_id, &ctx, observer).await
    }
}

fn parse_file_change(line: &str) -> Option<FileChange> {
    if line.len() < 4 {
        return None;
    }

    let code = &line[..2];
    let raw_path = line[3..].trim();
    let path = raw_path
        .rsplit(" -> ")
        .next()
        .unwrap_or(raw_path)
        .trim_matches('"')
        .to_string();

    if path.is_empty() {
        return None;
    }

    let status = if code.contains('U') || code == "AA" || code == "DD" {
        "conflict"
    } else if code == "??" || code.contains('A') {
        "new"
    } else if code.contains('R') {
        "renamed"
    } else if code.contains('D') {
        "deleted"
    } else {
        "modified"
    };

    Some(FileChange {
        path,
        status: status.to_string(),
    })
}

fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    }
}

fn error_text(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        output_text(output)
    } else {
        stderr
    }
}

fn failure_result(repo_path: &Path, error: impl Into<String>, output: String) -> OperationResult {
    OperationResult {
        repo_path: repo_path.to_string_lossy().to_string(),
        success: false,
        output,
        error: Some(error.into()),
    }
}

fn friendly_error(raw: &str) -> String {
    let lower = raw.to_lowercase();

    let hint = if lower.contains("permission denied (publickey)")
        || lower.contains("authentication failed")
        || lower.contains("could not read username")
        || lower.contains("terminal prompts disabled")
    {
        Some("Git could not sign in to the server. Check your SSH key or Git credentials.")
    } else if lower.contains("non-fast-forward")
        || lower.contains("fetch first")
        || lower.contains("updates were rejected")
    {
        Some("There are new changes on the server. Click 'Get latest' first, then send your changes again.")
    } else if lower.contains("would be overwritten")
        || lower.contains("please commit your changes or stash")
    {
        Some("You have changes that are not saved to Git. Save them first, then try again.")
    } else if lower.contains("automatic merge failed")
        || lower.contains("fix conflicts")
        || lower.contains("unmerged")
    {
        Some("Some files have conflicts. Open the project in your editor, fix the marked files, then save to Git.")
    } else if lower.contains("unable to auto-detect email")
        || lower.contains("please tell me who you are")
    {
        Some("Git does not know your name and email yet. Set them in your Git settings, then try again.")
    } else if lower.contains("nothing to commit") || lower.contains("no changes added to commit") {
        Some("There are no changes to save.")
    } else if lower.contains("not a git repository") {
        Some("This folder is no longer a Git project.")
    } else if lower.contains("does not appear to be a git repository")
        || lower.contains("no such remote")
        || lower.contains("no configured push destination")
    {
        Some("This project is not connected to a remote server yet.")
    } else {
        None
    };

    match hint {
        Some(hint) => format!("{hint}\n\nTechnical details: {}", raw.trim()),
        None => raw.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_log::NullObserver;
    use std::fs;
    use std::process::Command as StdCommand;

    pub(super) fn run_git(dir: &Path, args: &[&str]) {
        let status = StdCommand::new("git")
            .current_dir(dir)
            .args(args)
            .status()
            .expect("git must be installed for tests");
        assert!(status.success(), "git {:?} failed", args);
    }

    pub(super) fn init_repository(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        run_git(dir, &["init", "-b", "main"]);
        run_git(dir, &["config", "user.name", "Test User"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        fs::write(dir.join("tracked.txt"), "initial\n").unwrap();
        run_git(dir, &["add", "."]);
        run_git(
            dir,
            &["-c", "commit.gpgsign=false", "commit", "-m", "initial"],
        );
    }

    #[tokio::test]
    async fn inspects_working_tree_branches_and_operations() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init_repository(&repo);

        fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        fs::write(repo.join("new.txt"), "new\n").unwrap();

        let state = GitRunner::get_repo_state(&repo).await;
        assert_eq!(state.current_branch, "main");
        assert!(!state.is_detached_head);
        assert!(!state.is_clean);
        assert_eq!(state.modified_files, 1);
        assert_eq!(state.untracked_files, 1);
        assert_eq!(state.staged_files, 0);
        assert!(!state.head_commit_hash.is_empty());
        assert_eq!(state.head_commit_message, "initial");
        assert_eq!(state.operation_state, "idle");

        let branches = GitRunner::list_local_branches(&repo).await;
        assert_eq!(branches, vec!["main".to_string()]);

        assert_eq!(GitRunner::resolve_git_dir(&repo), repo.join(".git"));

        run_git(&repo, &["branch", "feature"]);
        let result = GitRunner::checkout_branch(&repo, "feature", true, &NullObserver).await;
        assert!(result.success, "{:?}", result.error);

        let state = GitRunner::get_repo_state(&repo).await;
        assert_eq!(state.current_branch, "feature");
        assert_eq!(state.modified_files, 1);
        assert_eq!(state.untracked_files, 1);

        let stash_list = StdCommand::new("git")
            .current_dir(&repo)
            .args(["stash", "list"])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&stash_list.stdout)
            .trim()
            .is_empty());
    }

    #[tokio::test]
    async fn lists_and_commits_changes() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init_repository(&repo);

        fs::write(repo.join("tracked.txt"), "changed\n").unwrap();
        fs::write(repo.join("new.txt"), "new\n").unwrap();

        let changes = GitRunner::list_changes(&repo).await;
        assert!(changes
            .iter()
            .any(|change| change.path == "tracked.txt" && change.status == "modified"));
        assert!(changes
            .iter()
            .any(|change| change.path == "new.txt" && change.status == "new"));

        let result = GitRunner::commit_changes(&repo, "update theme header", &NullObserver).await;
        assert!(result.success, "{:?}", result.error);

        let state = GitRunner::get_repo_state(&repo).await;
        assert!(state.is_clean);
        assert_eq!(state.head_commit_message, "update theme header");

        let empty = GitRunner::commit_changes(&repo, "nothing", &NullObserver).await;
        assert!(!empty.success);
        assert!(empty.error.unwrap().contains("no changes"));
    }

    #[tokio::test]
    async fn refuses_to_commit_unresolved_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        init_repository(&repo);

        run_git(&repo, &["checkout", "-b", "feature"]);
        fs::write(repo.join("tracked.txt"), "feature\n").unwrap();
        run_git(&repo, &["add", "."]);
        run_git(
            &repo,
            &["-c", "commit.gpgsign=false", "commit", "-m", "feature"],
        );

        run_git(&repo, &["checkout", "main"]);
        fs::write(repo.join("tracked.txt"), "main\n").unwrap();
        run_git(&repo, &["add", "."]);
        run_git(
            &repo,
            &["-c", "commit.gpgsign=false", "commit", "-m", "main"],
        );

        let merge = StdCommand::new("git")
            .current_dir(&repo)
            .args(["merge", "feature"])
            .status()
            .unwrap();
        assert!(!merge.success());

        let changes = GitRunner::list_changes(&repo).await;
        assert!(changes.iter().any(|change| change.status == "conflict"));

        let result = GitRunner::commit_changes(&repo, "merge feature", &NullObserver).await;
        assert!(!result.success);
        assert!(result.error.unwrap().contains("conflicts"));
    }

    #[test]
    fn explains_common_git_errors() {
        let rejected = friendly_error("! [rejected] main -> main (non-fast-forward)");
        assert!(rejected.contains("Get latest"));

        let auth = friendly_error("fatal: Authentication failed for 'https://example.com'");
        assert!(auth.contains("sign in"));
    }

    #[test]
    fn resolves_gitfile_for_worktrees() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("worktree");
        fs::create_dir_all(&repo).unwrap();
        fs::write(repo.join(".git"), "gitdir: /tmp/example-git-dir\n").unwrap();
        assert_eq!(
            GitRunner::resolve_git_dir(&repo),
            PathBuf::from("/tmp/example-git-dir")
        );
    }
}

#[cfg(test)]
mod macro_tests {
    use super::*;
    use crate::git_log::{GitObserver, GitStepEvent, GitStepStatus};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recording(Mutex<Vec<GitStepEvent>>);

    impl GitObserver for Recording {
        fn on_step(&self, event: &GitStepEvent) {
            self.0.lock().unwrap().push(event.clone());
        }
    }

    #[tokio::test]
    async fn emits_comment_and_command_for_each_step() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        super::tests::init_repository(&repo);
        std::fs::write(repo.join("tracked.txt"), "changed\n").unwrap();

        let recorder = Recording::default();
        let result = GitRunner::commit_changes(&repo, "update", &recorder).await;
        assert!(result.success, "{:?}", result.error);

        let events = recorder.0.lock().unwrap().clone();
        let ok: Vec<_> = events
            .iter()
            .filter(|event| event.status == GitStepStatus::Ok)
            .collect();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[0].comment, "Mark every changed file for saving");
        assert!(ok[0].command.starts_with("git -C "));
        assert_eq!(ok[1].alias.as_deref(), Some("gcmsg"));
    }

    #[tokio::test]
    async fn pull_marks_stash_steps_as_skipped_when_clean() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path().join("repo");
        super::tests::init_repository(&repo);

        let recorder = Recording::default();
        let result = GitRunner::execute_action(&repo, "pull", &recorder).await;
        // No remote configured: the pull step fails, the stash steps never run.
        assert!(!result.success);

        let events = recorder.0.lock().unwrap().clone();
        let skipped: Vec<_> = events
            .iter()
            .filter(|event| event.status == GitStepStatus::Skipped)
            .collect();
        assert_eq!(skipped.len(), 2);
        assert!(skipped.iter().all(|event| event.command.contains("stash")));
    }
}
