use crate::config::ConfigManager;
use crate::git_log::{GitObserver, GitStepEvent};
use crate::git_runner::GitRunner;
use crate::models::{
    AppConfig, BranchInfo, FileChange, OnboardingState, OperationResult, RepoGroup, Repository,
    ScanPath,
};
use crate::scanner::{RepoScanner, ScanObserver, ScanProgressEvent};
use crate::watcher::WatcherState;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Semaphore;

const INSPECTION_CONCURRENCY: usize = 8;

/// Streams per-repository commit-message generation progress onto the typed bus.
///
/// Implemented by `EventBus` (desktop + web); tests use [`NullCommitGenObserver`]
/// or a collecting fake. Mirrors the `GitObserver` / `ScanObserver` seams.
pub trait CommitGenObserver: Send + Sync {
    fn on_commit_gen(&self, event: &crate::commitgen::CommitGenEvent);
}

/// No-op observer for unit tests and callers that ignore the stream.
pub struct NullCommitGenObserver;

impl CommitGenObserver for NullCommitGenObserver {
    fn on_commit_gen(&self, _event: &crate::commitgen::CommitGenEvent) {}
}

pub fn get_config(config: &ConfigManager) -> Result<AppConfig, String> {
    config.snapshot()
}

/// Starts an AI commit-message run and returns its `runId` immediately
/// (contracts/generate-commit-messages-command.md C1–C4).
///
/// Transport-agnostic: the Tauri command and the web route both delegate here;
/// all validation happens before any task is spawned.
pub fn generate_commit_messages(
    repo_paths: Vec<String>,
    run_id: Option<String>,
    config: &ConfigManager,
    observer: Arc<dyn CommitGenObserver>,
) -> Result<String, String> {
    if repo_paths.is_empty() {
        return Err("no repositories selected".to_string());
    }

    let mut seen = HashSet::new();
    for path in &repo_paths {
        if !seen.insert(path.clone()) {
            return Err(format!("duplicate repository path: {path}"));
        }
        if !GitRunner::resolve_git_dir(Path::new(path)).exists() {
            return Err(format!("not a git repository: {path}"));
        }
    }

    let snapshot = config.snapshot()?;
    let settings = snapshot.ai.ok_or_else(|| "ai-not-configured".to_string())?;
    settings
        .validate()
        .map_err(|_| "ai-not-configured".to_string())?;

    match run_id {
        Some(existing) => {
            if repo_paths.len() != 1 {
                return Err("retry takes exactly one repository".to_string());
            }
            crate::commitgen::retry_repo(&repo_paths[0], &existing, &settings, observer);
            Ok(existing)
        }
        None => Ok(crate::commitgen::run_all(&repo_paths, &settings, observer)),
    }
}

pub fn add_scan_path(config: &ConfigManager, mut scan_path: ScanPath) -> Result<AppConfig, String> {
    let path = expand_home(&scan_path.path);
    if !PathBuf::from(&path).is_dir() {
        return Err(format!(
            "That folder does not exist or cannot be opened: {path}. Check the path and try again."
        ));
    }
    scan_path.path = path;

    {
        let mut cfg = config.config.lock().map_err(|e| e.to_string())?;
        cfg.scan_paths
            .retain(|sp| sp.id != scan_path.id && sp.path != scan_path.path);
        cfg.scan_paths.push(scan_path);
    }
    config.save()?;
    config.snapshot()
}

pub fn save_onboarding_state(
    config: &ConfigManager,
    mut onboarding: OnboardingState,
) -> Result<AppConfig, String> {
    if onboarding.current_step > 3 {
        return Err("Onboarding step must be between 0 and 3.".to_string());
    }

    onboarding.owner_names = onboarding
        .owner_names
        .into_iter()
        .map(|owner| owner.trim().trim_start_matches('@').to_lowercase())
        .filter(|owner| !owner.is_empty())
        .collect();
    onboarding.owner_names.sort();
    onboarding.owner_names.dedup();

    if let Some(path) = onboarding.scan_path.as_mut() {
        *path = expand_home(path);
    }

    {
        let mut cfg = config.config.lock().map_err(|error| error.to_string())?;
        cfg.onboarding = onboarding;
    }

    config.save()?;
    config.snapshot()
}

fn expand_home(raw: &str) -> String {
    let trimmed = raw.trim();
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));

    if trimmed == "~" {
        return home
            .map(|home| PathBuf::from(home).to_string_lossy().to_string())
            .unwrap_or_else(|| trimmed.to_string());
    }

    if let (Some(rest), Some(home)) = (trimmed.strip_prefix("~/"), home) {
        return PathBuf::from(home).join(rest).to_string_lossy().to_string();
    }

    trimmed.to_string()
}

pub fn remove_scan_path(config: &ConfigManager, id: String) -> Result<AppConfig, String> {
    {
        let mut cfg = config.config.lock().map_err(|e| e.to_string())?;
        cfg.scan_paths.retain(|sp| sp.id != id);
    }
    config.save()?;
    config.snapshot()
}

pub fn save_repo_group(config: &ConfigManager, group: RepoGroup) -> Result<AppConfig, String> {
    {
        let mut cfg = config.config.lock().map_err(|e| e.to_string())?;
        cfg.groups.retain(|g| g.id != group.id);
        cfg.groups.push(group);
    }
    config.save()?;
    config.snapshot()
}

pub fn delete_repo_group(config: &ConfigManager, group_id: String) -> Result<AppConfig, String> {
    {
        let mut cfg = config.config.lock().map_err(|e| e.to_string())?;
        cfg.groups.retain(|g| g.id != group_id);
    }
    config.save()?;
    config.snapshot()
}

pub async fn discover_repositories(
    config: &ConfigManager,
    watcher: &WatcherState,
    paths: Vec<String>,
    max_depth: usize,
    observer: Arc<dyn ScanObserver>,
) -> Result<Vec<Repository>, String> {
    let roots: Vec<String> = paths
        .into_iter()
        .filter(|path| !path.trim().is_empty())
        .collect();
    let run_id = uuid::Uuid::new_v4().to_string();
    let started_at = epoch_ms();

    observer.on_progress(&ScanProgressEvent::started(
        &run_id,
        roots.clone(),
        started_at,
    ));

    let outcome = run_discovery(
        config,
        watcher,
        &roots,
        max_depth,
        &run_id,
        Arc::clone(&observer),
    )
    .await;

    let (discovered, error) = match outcome {
        Ok(ok) => ok,
        Err(message) => {
            observer.on_progress(&ScanProgressEvent::done(
                &run_id,
                0,
                elapsed_since(started_at),
                Some(message.clone()),
            ));
            return Err(message);
        }
    };

    observer.on_progress(&ScanProgressEvent::done(
        &run_id,
        discovered.len(),
        elapsed_since(started_at),
        error.clone(),
    ));

    match error {
        Some(message) if discovered.is_empty() => Err(message),
        _ => Ok(discovered),
    }
}

/// One discovery run between `started` and `done`. Partial failures (a root
/// that cannot be walked) are reported via the second tuple element while the
/// healthy roots still deliver their repositories (research R9).
async fn run_discovery(
    config: &ConfigManager,
    watcher: &WatcherState,
    roots: &[String],
    max_depth: usize,
    run_id: &str,
    observer: Arc<dyn ScanObserver>,
) -> Result<(Vec<Repository>, Option<String>), String> {
    let group_map = match config.group_ids_by_path() {
        Ok(map) => map,
        Err(error) => return Err(error),
    };
    let limit = max_depth.max(1);
    let mut repo_paths = Vec::new();
    let mut walk_errors: Vec<String> = Vec::new();

    for path in roots {
        let (found, errors) =
            RepoScanner::scan_path_async(PathBuf::from(path), limit, run_id, Arc::clone(&observer))
                .await;
        repo_paths.extend(found);
        walk_errors.extend(errors);
    }

    repo_paths.sort();
    repo_paths.dedup();

    let semaphore = Arc::new(Semaphore::new(INSPECTION_CONCURRENCY));
    let mut tasks = Vec::with_capacity(repo_paths.len());
    let watch_paths = repo_paths.clone();

    for repo_path in repo_paths {
        let semaphore = Arc::clone(&semaphore);
        let observer = Arc::clone(&observer);
        let run_id = run_id.to_string();
        let group_ids = group_map
            .get(&repo_path.to_string_lossy().to_string())
            .cloned()
            .unwrap_or_default();

        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let repo = RepoScanner::inspect_repository(&repo_path, group_ids).await;
            observer.on_progress(&ScanProgressEvent::inspected(
                &run_id, &repo.path, repo.kind,
            ));
            repo
        }));
    }

    let mut discovered = Vec::with_capacity(tasks.len());
    for task in tasks {
        match task.await {
            Ok(repo) => discovered.push(repo),
            Err(error) => walk_errors.push(format!("inspection failed: {error}")),
        }
    }
    discovered.sort_by_key(|a| a.path.to_lowercase());

    watcher.update(watch_paths);

    let error = if walk_errors.is_empty() {
        None
    } else {
        Some(walk_errors.join("; "))
    };
    Ok((discovered, error))
}

fn epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn elapsed_since(started_at: u64) -> u64 {
    epoch_ms().saturating_sub(started_at)
}

pub async fn refresh_repository(path: String, group_ids: Vec<String>) -> Repository {
    RepoScanner::inspect_repository(&PathBuf::from(path), group_ids).await
}

pub async fn list_repo_changes(path: String) -> Vec<FileChange> {
    GitRunner::list_changes(&PathBuf::from(path)).await
}

pub async fn commit_repo_changes(
    path: String,
    message: String,
    observer: &dyn GitObserver,
) -> OperationResult {
    GitRunner::commit_changes(&PathBuf::from(path), &message, observer).await
}

/// Rewrites every step of one repository into the run shared by a group action,
/// so identical steps across clones fold into one card in the log.
struct RunScopedObserver {
    inner: Arc<dyn GitObserver>,
    run_id: String,
}

impl GitObserver for RunScopedObserver {
    fn on_step(&self, event: &GitStepEvent) {
        let mut scoped = event.clone();
        scoped.run_id = self.run_id.clone();
        self.inner.on_step(&scoped);
    }
}

pub async fn execute_group_action(
    repo_paths: Vec<String>,
    action: String,
    concurrency_limit: usize,
    observer: Arc<dyn GitObserver>,
) -> Result<Vec<OperationResult>, String> {
    if !matches!(
        action.as_str(),
        "fetch" | "pull" | "push" | "stash" | "stash_pop"
    ) {
        return Err(format!("Unsupported group action: {}", action));
    }

    let limit = concurrency_limit.clamp(1, 32);
    let semaphore = Arc::new(Semaphore::new(limit));
    let run_id = uuid::Uuid::new_v4().to_string();
    let mut tasks = Vec::with_capacity(repo_paths.len());

    for path in repo_paths {
        let semaphore = Arc::clone(&semaphore);
        let action = action.clone();
        let observer = Arc::clone(&observer);
        let run_id = run_id.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let scoped = RunScopedObserver {
                inner: observer,
                run_id,
            };
            GitRunner::execute_action(&PathBuf::from(path), &action, &scoped).await
        }));
    }

    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }
    Ok(results)
}

pub async fn switch_group_branch(
    repo_paths: Vec<String>,
    branch: String,
    auto_stash: bool,
    observer: Arc<dyn GitObserver>,
) -> Result<Vec<OperationResult>, String> {
    let semaphore = Arc::new(Semaphore::new(6));
    let run_id = uuid::Uuid::new_v4().to_string();
    let mut tasks = Vec::with_capacity(repo_paths.len());

    for path in repo_paths {
        let semaphore = Arc::clone(&semaphore);
        let branch = branch.clone();
        let observer = Arc::clone(&observer);
        let run_id = run_id.clone();
        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            let scoped = RunScopedObserver {
                inner: observer,
                run_id,
            };
            GitRunner::checkout_branch(&PathBuf::from(path), &branch, auto_stash, &scoped).await
        }));
    }

    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }
    Ok(results)
}

pub async fn list_group_branches(repo_paths: Vec<String>) -> Result<Vec<BranchInfo>, String> {
    let semaphore = Arc::new(Semaphore::new(INSPECTION_CONCURRENCY));
    let mut tasks = Vec::with_capacity(repo_paths.len());

    for path in repo_paths {
        let semaphore = Arc::clone(&semaphore);
        tasks.push(tokio::spawn(async move {
            let _permit = semaphore.acquire_owned().await.ok();
            GitRunner::list_local_branches(&PathBuf::from(path)).await
        }));
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    for task in tasks {
        if let Ok(branches) = task.await {
            for branch in branches {
                *counts.entry(branch).or_insert(0) += 1;
            }
        }
    }

    let mut branches: Vec<BranchInfo> = counts
        .into_iter()
        .map(|(name, repo_count)| BranchInfo { name, repo_count })
        .collect();
    branches.sort_by(|a, b| {
        b.repo_count
            .cmp(&a.repo_count)
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(branches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::ScanProgressPhase;

    fn temp_manager() -> (tempfile::TempDir, ConfigManager) {
        let temp = tempfile::tempdir().unwrap();
        let manager = ConfigManager::from_path(temp.path().join("config.json"));
        (temp, manager)
    }

    #[test]
    fn normalizes_and_persists_onboarding_state() {
        let (_temp, manager) = temp_manager();

        let saved = save_onboarding_state(
            &manager,
            OnboardingState {
                started: true,
                current_step: 2,
                scan_path: Some("/tmp/projects".to_string()),
                owner_names: vec![" @Acme ".to_string(), "acme".to_string(), String::new()],
                selected_repo_paths: vec!["/tmp/projects/theme".to_string()],
                ..OnboardingState::default()
            },
        )
        .unwrap();

        assert_eq!(saved.onboarding.owner_names, vec!["acme".to_string()]);
        assert_eq!(saved.onboarding.current_step, 2);
        assert!(saved.onboarding.started);
        assert_eq!(manager.snapshot().unwrap().onboarding.current_step, 2);
    }

    #[test]
    fn rejects_out_of_range_onboarding_step() {
        let (_temp, manager) = temp_manager();
        let state = OnboardingState {
            current_step: 4,
            ..OnboardingState::default()
        };

        assert!(save_onboarding_state(&manager, state).is_err());
    }

    #[tokio::test]
    async fn discovery_emits_started_found_inspected_and_done() {
        let (_temp, manager) = temp_manager();
        let event_bus = crate::watcher::EventBus::new();
        let watcher = WatcherState::new(event_bus);

        let root = _temp.path().join("projects");
        let repo = root.join("app");
        std::fs::create_dir_all(repo.join(".git")).unwrap();

        let observer = std::sync::Arc::new(crate::scanner::CollectingObserver::new());
        let discovered = discover_repositories(
            &manager,
            &watcher,
            vec![root.to_string_lossy().to_string()],
            4,
            observer.clone(),
        )
        .await
        .unwrap();

        let events = observer.events();
        let phases: Vec<_> = events.iter().map(|event| event.phase).collect();
        assert_eq!(phases.first(), Some(&ScanProgressPhase::Started));
        assert_eq!(phases.last(), Some(&ScanProgressPhase::Done));
        assert!(phases.contains(&ScanProgressPhase::Found));
        assert!(phases.contains(&ScanProgressPhase::Inspected));

        let started = &events[0];
        assert_eq!(started.roots, vec![root.to_string_lossy().to_string()]);
        assert!(started.started_at.is_some());

        let done = events.last().unwrap();
        assert_eq!(done.found_count, Some(discovered.len()));
        assert!(done.error.is_none());
        assert_eq!(started.run_id, done.run_id);
    }

    #[tokio::test]
    async fn discovery_always_emits_done_including_on_failures() {
        let (_temp, manager) = temp_manager();
        let event_bus = crate::watcher::EventBus::new();
        let watcher = WatcherState::new(event_bus);
        let observer = std::sync::Arc::new(crate::scanner::CollectingObserver::new());

        // SC-006: a root that vanished from disk must terminate the run with an
        // error rather than leaving the frontend spinning.
        let missing = _temp.path().join("gone");
        let outcome = discover_repositories(
            &manager,
            &watcher,
            vec![missing.to_string_lossy().to_string()],
            4,
            observer.clone(),
        )
        .await;

        assert!(outcome.is_err());
        let events = observer.events();
        assert_eq!(events.first().unwrap().phase, ScanProgressPhase::Started);
        let done = events.last().unwrap();
        assert_eq!(done.phase, ScanProgressPhase::Done);
        assert!(done.error.is_some());
        assert_eq!(done.found_count, Some(0));
    }
}
