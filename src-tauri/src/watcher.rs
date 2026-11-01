use crate::events::AppEvent;
use crate::git_log::{GitObserver, GitStepEvent};
use crate::git_runner::GitRunner;
use crate::scanner::{ScanObserver, ScanProgressEvent};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::broadcast;

const DEBOUNCE: Duration = Duration::from_millis(400);
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Fan-out channel shared by desktop (Tauri events) and web (SSE) frontends.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<AppEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(8192);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AppEvent> {
        self.sender.subscribe()
    }

    pub fn publish_fs_changed(&self, repo_path: String) {
        let _ = self.sender.send(AppEvent::FsChanged(repo_path));
    }
}

impl GitObserver for EventBus {
    fn on_step(&self, event: &GitStepEvent) {
        let _ = self.sender.send(AppEvent::GitStep(event.clone()));
    }
}

impl ScanObserver for EventBus {
    fn on_progress(&self, event: &ScanProgressEvent) {
        let _ = self.sender.send(AppEvent::ScanProgress(event.clone()));
    }
}

impl crate::service::CommitGenObserver for EventBus {
    fn on_commit_gen(&self, event: &crate::commitgen::CommitGenEvent) {
        // Observability only: the payload itself never carries a credential.
        tracing::debug!(
            phase = event.phase_tag(),
            path = event.path(),
            run_id = event.run_id(),
            "commitgen event"
        );
        let _ = self.sender.send(AppEvent::CommitGen(event.clone()));
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GitWatcher {
    _watcher: RecommendedWatcher,
    stop: Arc<AtomicBool>,
}

impl Drop for GitWatcher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl GitWatcher {
    pub fn new(event_bus: EventBus, watch_paths: Vec<PathBuf>) -> notify::Result<Self> {
        let (tx, rx) = mpsc::channel::<notify::Result<Event>>();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            Config::default(),
        )?;
        let stop = Arc::new(AtomicBool::new(false));

        let mut roots: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut poll_roots: Vec<(PathBuf, PathBuf)> = Vec::new();
        let mut watched = 0usize;
        let mut watch_failed = false;

        for repo_path in &watch_paths {
            let git_dir = GitRunner::resolve_git_dir(repo_path);
            if !git_dir.exists() {
                continue;
            }

            let canonical_git_dir =
                std::fs::canonicalize(&git_dir).unwrap_or_else(|_| git_dir.clone());
            match watcher.watch(&canonical_git_dir, RecursiveMode::Recursive) {
                Ok(()) => {
                    watched += 1;
                    roots.push((canonical_git_dir, repo_path.clone()));
                }
                Err(error) => {
                    watch_failed = true;
                    log::warn!(
                        "failed to watch {}, falling back to polling: {error}",
                        canonical_git_dir.display()
                    );
                    poll_roots.push((canonical_git_dir, repo_path.clone()));
                }
            }
        }

        log::info!(
            "watching git metadata for {watched} repositories ({} via polling)",
            poll_roots.len()
        );

        if watch_failed {
            log::warn!(
                "inotify watch limit exhausted; polling {} repositories every {:?}",
                poll_roots.len(),
                POLL_INTERVAL
            );
        }

        let poll_event_bus = event_bus.clone();
        let poll_stop = stop.clone();
        std::thread::spawn(move || {
            let mut last_emit: HashMap<String, Instant> = HashMap::new();

            for event_result in rx {
                let Ok(event) = event_result else {
                    continue;
                };

                for path in event.paths {
                    if !is_relevant_git_path(&path) {
                        continue;
                    }

                    let Some(repo_root) = repo_root_for(&path, &roots) else {
                        continue;
                    };

                    let key = repo_root.to_string_lossy().to_string();
                    let now = Instant::now();
                    let should_emit = last_emit
                        .get(&key)
                        .map_or(true, |last| now.duration_since(*last) >= DEBOUNCE);

                    if should_emit {
                        last_emit.insert(key.clone(), now);
                        event_bus.publish_fs_changed(key);
                    }
                }
            }
        });

        if !poll_roots.is_empty() {
            spawn_poller(poll_roots, poll_event_bus, poll_stop);
        }

        Ok(Self {
            _watcher: watcher,
            stop,
        })
    }
}

const RELEVANT_FILES: &[&str] = &[
    "HEAD",
    "index",
    "packed-refs",
    "FETCH_HEAD",
    "ORIG_HEAD",
    "MERGE_HEAD",
    "CHERRY_PICK_HEAD",
    "REBASE_HEAD",
    "AUTO_MERGE",
];

type GitFingerprint = Vec<(PathBuf, u64, Option<SystemTime>)>;

fn spawn_poller(poll_roots: Vec<(PathBuf, PathBuf)>, event_bus: EventBus, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut fingerprints: HashMap<PathBuf, GitFingerprint> = poll_roots
            .iter()
            .map(|(git_dir, repo_path)| (repo_path.clone(), git_fingerprint(git_dir)))
            .collect();

        while !stop.load(Ordering::Relaxed) {
            std::thread::sleep(POLL_INTERVAL);

            if stop.load(Ordering::Relaxed) {
                break;
            }

            for (git_dir, repo_path) in &poll_roots {
                let current = git_fingerprint(git_dir);
                if fingerprints.get(repo_path) != Some(&current) {
                    fingerprints.insert(repo_path.clone(), current);
                    event_bus.publish_fs_changed(repo_path.to_string_lossy().to_string());
                }
            }
        }
    });
}

fn git_fingerprint(git_dir: &Path) -> GitFingerprint {
    let mut entries = Vec::new();

    for name in RELEVANT_FILES {
        push_fingerprint(&git_dir.join(name), &mut entries);
    }

    collect_ref_fingerprints(&git_dir.join("refs"), &mut entries);
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

fn collect_ref_fingerprints(dir: &Path, entries: &mut GitFingerprint) {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ref_fingerprints(&path, entries);
        } else {
            push_fingerprint(&path, entries);
        }
    }
}

fn push_fingerprint(path: &Path, entries: &mut GitFingerprint) {
    match std::fs::metadata(path) {
        Ok(metadata) => {
            entries.push((path.to_path_buf(), metadata.len(), metadata.modified().ok()))
        }
        Err(_) => entries.push((path.to_path_buf(), 0, None)),
    }
}

fn is_relevant_git_path(path: &Path) -> bool {
    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
        if RELEVANT_FILES.contains(&name) {
            return true;
        }
    }

    path.components()
        .any(|component| component.as_os_str() == std::ffi::OsStr::new("refs"))
}

fn repo_root_for(path: &Path, roots: &[(PathBuf, PathBuf)]) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(path).ok();

    if let Some(canonical) = canonical.as_ref() {
        if let Some((_, repo_root)) = roots
            .iter()
            .find(|(git_dir, _)| canonical.starts_with(git_dir))
        {
            return Some(repo_root.clone());
        }
    }

    roots
        .iter()
        .find(|(git_dir, _)| path.starts_with(git_dir))
        .map(|(_, repo_root)| repo_root.clone())
}

pub struct WatcherState {
    watcher: Mutex<Option<GitWatcher>>,
    event_bus: EventBus,
}

impl WatcherState {
    pub fn new(event_bus: EventBus) -> Self {
        Self {
            watcher: Mutex::new(None),
            event_bus,
        }
    }

    pub fn update(&self, repo_paths: Vec<PathBuf>) {
        if repo_paths.is_empty() {
            if let Ok(mut guard) = self.watcher.lock() {
                *guard = None;
            }
            return;
        }

        match GitWatcher::new(self.event_bus.clone(), repo_paths) {
            Ok(watcher) => {
                if let Ok(mut guard) = self.watcher.lock() {
                    *guard = Some(watcher);
                }
            }
            Err(error) => {
                log::warn!("failed to start git watcher: {error}");
            }
        }
    }
}
