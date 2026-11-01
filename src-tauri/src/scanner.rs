//! Repository discovery plus the live scan-progress stream.
//!
//! The `ScanObserver`/`ScanProgressEvent` pair mirrors `GitObserver`/`GitStepEvent`
//! in `git_log.rs`: the scanner stays free of transport types and tests inject a
//! collecting fake.

use crate::git_runner::GitRunner;
use crate::models::{GitRepoState, RepoKind, Repository};
use ignore::WalkBuilder;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    ".cache",
    ".venv",
    "venv",
    ".tox",
    "__pycache__",
    ".next",
    ".nuxt",
    ".idea",
    ".vscode",
];

/// At most one `tick` per run every 400 ms (same constant as the watcher debounce).
const TICK_THROTTLE: Duration = Duration::from_millis(400);
/// Heartbeat floor: re-emit the last tick at least every 500 ms while walking
/// so the UI sees ≥2 updates/second even on sparse trees.
const HEARTBEAT_FLOOR: Duration = Duration::from_millis(500);
const HEARTBEAT_POLL: Duration = Duration::from_millis(100);
/// Cap on per-root IO errors carried into `done { error }` so one broken tree
/// cannot produce megabytes of message text.
const MAX_WALK_ERRORS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanProgressPhase {
    Started,
    Found,
    Tick,
    Inspected,
    Done,
}

/// One payload type for every phase; fields per contracts/scan-progress-event.md §C2.
/// Only the per-phase constructors below exist, so an invalid field combination
/// is unconstructable and the frontend may assume well-formed payloads.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgressEvent {
    pub phase: ScanProgressPhase,
    pub run_id: String,
    pub root: Option<String>,
    pub path: Option<String>,
    pub name: Option<String>,
    pub kind: Option<RepoKind>,
    pub current_dir: Option<String>,
    pub roots: Vec<String>,
    pub started_at: Option<u64>,
    pub found_count: Option<usize>,
    pub elapsed_ms: Option<u64>,
    pub error: Option<String>,
}

impl ScanProgressEvent {
    fn blank(phase: ScanProgressPhase, run_id: &str) -> Self {
        Self {
            phase,
            run_id: run_id.to_string(),
            root: None,
            path: None,
            name: None,
            kind: None,
            current_dir: None,
            roots: Vec::new(),
            started_at: None,
            found_count: None,
            elapsed_ms: None,
            error: None,
        }
    }

    pub fn started(run_id: &str, roots: Vec<String>, started_at: u64) -> Self {
        let mut event = Self::blank(ScanProgressPhase::Started, run_id);
        event.roots = roots;
        event.started_at = Some(started_at);
        event
    }

    pub fn found(run_id: &str, root: &str, path: &str, name: &str) -> Self {
        let mut event = Self::blank(ScanProgressPhase::Found, run_id);
        event.root = Some(root.to_string());
        event.path = Some(path.to_string());
        event.name = Some(name.to_string());
        event
    }

    pub fn tick(run_id: &str, root: &str, current_dir: &str) -> Self {
        let mut event = Self::blank(ScanProgressPhase::Tick, run_id);
        event.root = Some(root.to_string());
        event.current_dir = Some(current_dir.to_string());
        event
    }

    pub fn inspected(run_id: &str, path: &str, kind: RepoKind) -> Self {
        let mut event = Self::blank(ScanProgressPhase::Inspected, run_id);
        event.path = Some(path.to_string());
        event.kind = Some(kind);
        event
    }

    pub fn done(run_id: &str, found_count: usize, elapsed_ms: u64, error: Option<String>) -> Self {
        let mut event = Self::blank(ScanProgressPhase::Done, run_id);
        event.found_count = Some(found_count);
        event.elapsed_ms = Some(elapsed_ms);
        event.error = error;
        event
    }
}

/// Receives every scan-progress event. Implemented by the event bus in production
/// and by a recording observer in tests.
pub trait ScanObserver: Send + Sync {
    fn on_progress(&self, event: &ScanProgressEvent);
}

/// Observer used by call sites that do not care about the progress stream.
#[cfg(test)]
pub struct NullScanObserver;

#[cfg(test)]
impl ScanObserver for NullScanObserver {
    fn on_progress(&self, _event: &ScanProgressEvent) {}
}

/// Observer that records every event for assertions.
#[cfg(test)]
#[derive(Default)]
pub struct CollectingObserver {
    events: Mutex<Vec<ScanProgressEvent>>,
}

#[cfg(test)]
impl CollectingObserver {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn events(&self) -> Vec<ScanProgressEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[cfg(test)]
impl ScanObserver for CollectingObserver {
    fn on_progress(&self, event: &ScanProgressEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

#[derive(Default)]
struct TickState {
    last_emit: Option<Instant>,
    last_dir: Option<String>,
    walking: bool,
}

pub struct RepoScanner;

impl RepoScanner {
    /// Walks one root and returns `(repository_paths, walk_errors)`.
    /// `walk_errors` are per-entry IO failures (capped); a fully unreadable root
    /// still reports an error instead of silently yielding zero results.
    ///
    /// The observer is an `Arc` because `ignore::WalkBuilder::filter_entry`
    /// requires a `'static` callback; callers that only have `&dyn ScanObserver`
    /// can pass `Arc::clone(&observer)`.
    pub fn scan_path(
        root: &Path,
        max_depth: usize,
        run_id: &str,
        observer: Arc<dyn ScanObserver>,
    ) -> (Vec<PathBuf>, Vec<String>) {
        if !root.is_dir() {
            return (
                Vec::new(),
                vec![format!("{} is not a readable folder", root.display())],
            );
        }

        let found = Arc::new(Mutex::new(Vec::<PathBuf>::new()));
        let sink = Arc::clone(&found);
        let walk_errors = Arc::new(Mutex::new(Vec::<String>::new()));
        let ticks = Arc::new(Mutex::new(TickState {
            walking: true,
            ..TickState::default()
        }));
        let root_str = root.to_string_lossy().to_string();
        let run_id = run_id.to_string();

        let mut builder = WalkBuilder::new(root);
        builder
            .max_depth(Some(max_depth.max(1)))
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true);

        let heartbeat_ticks = Arc::clone(&ticks);
        let heartbeat_root = root_str.clone();
        let heartbeat_run = run_id.clone();
        let heartbeat_observer = Arc::clone(&observer);
        let walk_ticks = Arc::clone(&ticks);
        let walk_root = root_str.clone();
        let walk_run = run_id.clone();
        let walk_observer = Arc::clone(&observer);
        let error_sink = Arc::clone(&walk_errors);

        std::thread::scope(|scope| {
            scope.spawn(move || loop {
                std::thread::sleep(HEARTBEAT_POLL);
                let mut state = heartbeat_ticks.lock().unwrap();
                if !state.walking {
                    break;
                }
                let stale = state
                    .last_emit
                    .map_or(true, |last| last.elapsed() >= HEARTBEAT_FLOOR);
                if stale {
                    if let Some(dir) = state.last_dir.clone() {
                        state.last_emit = Some(Instant::now());
                        let event = ScanProgressEvent::tick(&heartbeat_run, &heartbeat_root, &dir);
                        drop(state);
                        heartbeat_observer.on_progress(&event);
                    }
                }
            });

            builder.filter_entry(move |entry| {
                let name = entry.file_name().to_string_lossy();

                // A `.git` directory (or gitfile used by worktrees/submodules) marks a
                // repository root. Record the parent and do not descend into metadata.
                if name == ".git" {
                    if let Some(parent) = entry.path().parent() {
                        sink.lock().unwrap().push(parent.to_path_buf());
                        let path_str = parent.to_string_lossy().to_string();
                        let repo_name = parent
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path_str.clone());
                        walk_observer.on_progress(&ScanProgressEvent::found(
                            &walk_run, &walk_root, &path_str, &repo_name,
                        ));
                    }
                    return false;
                }

                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    let dir_str = entry.path().to_string_lossy().to_string();
                    let mut state = walk_ticks.lock().unwrap();
                    let now = Instant::now();
                    let due = state
                        .last_emit
                        .map_or(true, |last| now.duration_since(last) >= TICK_THROTTLE);
                    if due {
                        state.last_emit = Some(now);
                        state.last_dir = Some(dir_str.clone());
                        drop(state);
                        walk_observer
                            .on_progress(&ScanProgressEvent::tick(&walk_run, &walk_root, &dir_str));
                    } else {
                        state.last_dir = Some(dir_str);
                    }

                    if EXCLUDED_DIRS
                        .iter()
                        .any(|excluded| *excluded == name.as_ref())
                    {
                        return false;
                    }
                }

                true
            });

            for result in builder.build() {
                if let Err(error) = result {
                    let mut errors = error_sink.lock().unwrap();
                    if errors.len() < MAX_WALK_ERRORS {
                        errors.push(error.to_string());
                    }
                }
            }

            ticks.lock().unwrap().walking = false;
        });

        let mut repos = found.lock().map(|repos| repos.clone()).unwrap_or_default();
        repos.sort();
        repos.dedup();
        let errors = walk_errors
            .lock()
            .map(|errors| errors.clone())
            .unwrap_or_default();
        (repos, errors)
    }

    pub async fn scan_path_async(
        root: PathBuf,
        max_depth: usize,
        run_id: &str,
        observer: Arc<dyn ScanObserver>,
    ) -> (Vec<PathBuf>, Vec<String>) {
        let run_id = run_id.to_string();
        let root_for_error = root.clone();
        match tokio::task::spawn_blocking(move || {
            Self::scan_path(&root, max_depth, &run_id, observer)
        })
        .await
        {
            Ok(result) => result,
            Err(error) => (
                Vec::new(),
                vec![format!(
                    "scan of {} stopped unexpectedly: {error}",
                    root_for_error.display()
                )],
            ),
        }
    }

    pub async fn inspect_repository(repo_path: &Path, group_ids: Vec<String>) -> Repository {
        let path_str = repo_path.to_string_lossy().to_string();
        let name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path_str.clone());
        let id = repository_id(&path_str);

        if !repo_path.is_dir() {
            return Repository {
                id,
                name,
                path: path_str,
                is_accessible: false,
                state: GitRepoState::default(),
                group_ids,
                kind: RepoKind::Other,
            };
        }

        let state = GitRunner::get_repo_state(repo_path).await;
        let kind = detect_repository_kind(repo_path);

        Repository {
            id,
            name,
            path: path_str,
            is_accessible: true,
            state,
            group_ids,
            kind,
        }
    }
}

fn detect_repository_kind(repo_path: &Path) -> RepoKind {
    if repo_path.join("src/Resources/theme.json").is_file() {
        return RepoKind::Theme;
    }

    let composer_path = repo_path.join("composer.json");
    if let Ok(raw) = std::fs::read_to_string(composer_path) {
        if let Ok(composer) = serde_json::from_str::<serde_json::Value>(&raw) {
            if composer.get("type").and_then(|value| value.as_str())
                == Some("shopware-platform-plugin")
            {
                return RepoKind::Plugin;
            }
        }
    }

    RepoKind::Other
}

fn repository_id(path: &str) -> String {
    // Deterministic IDs keep Vue rows (and selection state) stable across refreshes.
    uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, path.as_bytes()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scans_repositories_and_skips_dependency_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let service_a = root.join("service-a");
        fs::create_dir_all(service_a.join(".git")).unwrap();

        let service_b = root.join("nested/deeper/service-b");
        fs::create_dir_all(service_b.join(".git")).unwrap();

        let worktree = root.join("worktree-repo");
        fs::create_dir_all(&worktree).unwrap();
        fs::write(worktree.join(".git"), "gitdir: /tmp/fake-git-dir\n").unwrap();

        fs::create_dir_all(root.join("node_modules/dependency/.git")).unwrap();
        fs::create_dir_all(root.join("vendor/library/.git")).unwrap();
        fs::create_dir_all(root.join("a/b/c/d/e/.git")).unwrap();

        let observer = Arc::new(CollectingObserver::new());
        let sink: Arc<dyn ScanObserver> = observer.clone();
        let (found, walk_errors) = RepoScanner::scan_path(root, 4, "run-1", sink);
        assert!(walk_errors.is_empty());

        assert!(found.contains(&service_a));
        assert!(found.contains(&service_b));
        assert!(found.contains(&worktree));
        assert!(!found
            .iter()
            .any(|p| p.starts_with(root.join("node_modules"))));
        assert!(!found.iter().any(|p| p.starts_with(root.join("vendor"))));
        assert!(!found.contains(&root.join("a/b/c/d/e")));

        // FR-001: every returned repository produced exactly one `found` event
        // that names the root being walked and the repository folder.
        let root_str = root.to_string_lossy().to_string();
        let events = observer.events();
        let found_events: Vec<_> = events
            .iter()
            .filter(|event| event.phase == ScanProgressPhase::Found)
            .collect();
        assert_eq!(found_events.len(), found.len());
        for path in &found {
            let path_str = path.to_string_lossy().to_string();
            let hit = found_events
                .iter()
                .any(|event| event.path.as_deref() == Some(path_str.as_str()));
            assert!(hit, "missing found event for {path_str}");
            assert!(found_events
                .iter()
                .all(|event| event.root.as_deref() == Some(root_str.as_str())));
            assert!(found_events.iter().all(|event| event.run_id == "run-1"));
        }
    }

    #[test]
    fn tick_events_respect_the_throttle_window() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        for index in 0..200 {
            fs::create_dir_all(root.join(format!("dir-{index}"))).unwrap();
        }

        let observer = Arc::new(CollectingObserver::new());
        let sink: Arc<dyn ScanObserver> = observer.clone();
        let (found, walk_errors) = RepoScanner::scan_path(root, 3, "run-1", sink);
        assert!(found.is_empty());
        assert!(walk_errors.is_empty());

        let events = observer.events();
        let ticks: Vec<_> = events
            .iter()
            .filter(|event| event.phase == ScanProgressPhase::Tick)
            .collect();

        // SC-001: the throttling never allows two ticks closer than 400 ms.
        // A short synthetic walk stays well below one tick per 400 ms, so a
        // handful of ticks for 200 directories proves the cap holds.
        assert!(
            ticks.len() <= 2,
            "expected throttled ticks, got {}",
            ticks.len()
        );
        assert!(ticks.iter().all(|event| event
            .current_dir
            .as_deref()
            .is_some_and(|dir| dir.starts_with(root.to_str().unwrap()))));
    }

    #[tokio::test]
    async fn scan_path_async_walks_with_the_null_observer() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("app/.git")).unwrap();

        let observer: Arc<dyn ScanObserver> = Arc::new(NullScanObserver);
        let (found, errors) =
            RepoScanner::scan_path_async(root.to_path_buf(), 4, "run-1", observer).await;

        assert_eq!(found, vec![root.join("app")]);
        assert!(errors.is_empty());
    }

    #[test]
    fn detects_theme_plugin_and_other_repositories() {
        let temp = tempfile::tempdir().unwrap();

        let theme = temp.path().join("theme");
        fs::create_dir_all(theme.join("src/Resources")).unwrap();
        fs::write(theme.join("src/Resources/theme.json"), "{}").unwrap();

        let plugin = temp.path().join("plugin");
        fs::create_dir_all(&plugin).unwrap();
        fs::write(
            plugin.join("composer.json"),
            r#"{"type":"shopware-platform-plugin"}"#,
        )
        .unwrap();

        let other = temp.path().join("other");
        fs::create_dir_all(&other).unwrap();

        assert_eq!(detect_repository_kind(&theme), RepoKind::Theme);
        assert_eq!(detect_repository_kind(&plugin), RepoKind::Plugin);
        assert_eq!(detect_repository_kind(&other), RepoKind::Other);
    }
}
