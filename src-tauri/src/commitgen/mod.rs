//! The single AI seam (specs/002-ai-commit-messages).
//!
//! This is the ONLY module allowed to depend on the LLM framework (`serdes_ai`)
//! — plan §Structure Decision — so a young, git-pinned dependency can be
//! swapped without touching services, transports or UI. Everything here is
//! read-only by construction (Principle V): no git write API is reachable.

pub mod agent;
pub mod diff;
pub mod draft;
pub mod settings;
pub mod tools;

pub use agent::{CommitGenError, RepoCtx};
pub use diff::collect_change_detail;
pub use draft::{CommitDraft, CommitDraftWire};
pub use settings::AiSettings;

use crate::service::CommitGenObserver;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Upper bound on simultaneous provider calls (research R6).
pub const CONCURRENCY: usize = 8;

/// Starts one generation batch and returns the freshly minted `runId`
/// immediately; the work continues on the tokio runtime so events stream
/// while the transport answers `202` (contract C3).
pub fn run_all(
    repo_paths: &[String],
    settings: &AiSettings,
    observer: Arc<dyn CommitGenObserver>,
) -> String {
    let run_id = uuid::Uuid::new_v4().to_string();
    let paths = repo_paths.to_vec();
    let snapshot = settings.clone();
    let task_run_id = run_id.clone();

    tokio::spawn(async move {
        run_batch(&task_run_id, &paths, &snapshot, observer).await;
    });

    run_id
}

/// Re-runs exactly one repository under an existing `runId` (FR-015).
///
/// Emits a fresh `started`/terminal pair for that path only; siblings are not
/// touched (contracts/commitgen-events.md §C3 rule 2).
pub fn retry_repo(
    path: &str,
    run_id: &str,
    settings: &AiSettings,
    observer: Arc<dyn CommitGenObserver>,
) {
    let paths = vec![path.to_string()];
    let snapshot = settings.clone();
    let task_run_id = run_id.to_string();

    tokio::spawn(async move {
        run_batch(&task_run_id, &paths, &snapshot, observer).await;
    });
}

/// Bounded concurrent batch (research R6). Every repository is handled
/// independently: one failure only ever produces an `error` event for its own
/// path (FR-008, SC-004). Nothing here writes to a repository (Principle V).
async fn run_batch(
    run_id: &str,
    repo_paths: &[String],
    settings: &AiSettings,
    observer: Arc<dyn CommitGenObserver>,
) {
    let credential = Arc::new(agent::read_credential(settings));
    let snapshot = Arc::new(settings.clone());

    run_batch_with(run_id, repo_paths, observer, move |path| {
        let snapshot = Arc::clone(&snapshot);
        let credential = Arc::clone(&credential);
        async move {
            let repo_path = Path::new(&path);
            let detail = collect_change_detail(repo_path, snapshot.diff_budget_chars).await?;
            let ctx = Arc::new(RepoCtx::new(
                repo_path.to_path_buf(),
                repo_name_of(repo_path),
            ));
            agent::generate_for_repo(agent::DraftRequest {
                settings: &snapshot,
                credential: credential.as_ref().clone(),
                ctx,
                change_detail: &detail,
            })
            .await
        }
    })
    .await;
}

/// Concurrency core, generic over the per-repo generator so tests can drive it
/// with an offline fake instead of a live provider.
async fn run_batch_with<F, Fut>(
    run_id: &str,
    repo_paths: &[String],
    observer: Arc<dyn CommitGenObserver>,
    generate: F,
) where
    F: Fn(String) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<CommitDraft, CommitGenError>> + Send + 'static,
{
    // Every row must read "generating" at once (US2-AC1).
    for path in repo_paths {
        observer.on_commit_gen(&CommitGenEvent::started(run_id, path));
    }

    let semaphore = Arc::new(Semaphore::new(CONCURRENCY));
    let generate = Arc::new(generate);
    let mut handles = Vec::with_capacity(repo_paths.len());

    for path in repo_paths {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("semaphore stays open for the whole batch");
        let observer = Arc::clone(&observer);
        let generate = Arc::clone(&generate);
        let task_run_id = run_id.to_string();
        let task_path = path.clone();
        let future = generate(path.clone());

        handles.push(tokio::spawn(async move {
            // Held for the whole attempt; no lock is held across an `.await`.
            let _permit = permit;
            match future.await {
                Ok(draft) => observer.on_commit_gen(&CommitGenEvent::done(
                    &task_run_id,
                    &task_path,
                    draft.to_wire(),
                )),
                Err(error) => observer.on_commit_gen(&CommitGenEvent::error(
                    &task_run_id,
                    &task_path,
                    error.user_reason(),
                )),
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}

fn repo_name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

impl From<diff::ChangeDetailError> for CommitGenError {
    fn from(error: diff::ChangeDetailError) -> Self {
        match error {
            diff::ChangeDetailError::NothingToSummarize => Self::NothingToSummarize,
            diff::ChangeDetailError::Conflicts => Self::Conflicts,
            diff::ChangeDetailError::Unavailable(detail) => Self::Io(detail),
        }
    }
}

/// Draft types the provider may choose from (data-model E2, FR-003).
pub const CONVENTIONAL_COMMIT_TYPES: [&str; 11] = [
    "feat", "fix", "refactor", "perf", "docs", "style", "test", "build", "ci", "chore", "revert",
];

/// Marker appended when the change detail exceeds `diff_budget_chars` (FR-009).
pub const TRUNCATION_MARKER: &str = "[... truncated ...]";

/// One generation lifecycle event on the typed bus (data-model E3).
///
/// Payload is identical on both transports; `draft` carries the backend-rendered
/// string so DE/EN and trimming edge cases never diverge (contracts/commitgen-events.md).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(
    rename_all = "camelCase",
    tag = "phase",
    rename_all_fields = "camelCase"
)]
pub enum CommitGenEvent {
    Started {
        run_id: String,
        path: String,
    },
    Done {
        run_id: String,
        path: String,
        draft: CommitDraftWire,
    },
    Error {
        run_id: String,
        path: String,
        error: String,
    },
}

impl CommitGenEvent {
    pub fn started(run_id: &str, path: &str) -> Self {
        Self::Started {
            run_id: run_id.to_string(),
            path: path.to_string(),
        }
    }

    pub fn done(run_id: &str, path: &str, draft: CommitDraftWire) -> Self {
        Self::Done {
            run_id: run_id.to_string(),
            path: path.to_string(),
            draft,
        }
    }

    pub fn error(run_id: &str, path: &str, reason: impl Into<String>) -> Self {
        Self::Error {
            run_id: run_id.to_string(),
            path: path.to_string(),
            error: reason.into(),
        }
    }

    /// Repo this event belongs to — the store keys solely on `path` (contracts §C3 rule 4).
    pub fn path(&self) -> &str {
        match self {
            Self::Started { path, .. } | Self::Done { path, .. } | Self::Error { path, .. } => path,
        }
    }

    /// Stable phase tag matching the serialized `phase` field.
    pub fn phase_tag(&self) -> &'static str {
        match self {
            Self::Started { .. } => "started",
            Self::Done { .. } => "done",
            Self::Error { .. } => "error",
        }
    }

    pub fn run_id(&self) -> &str {
        match self {
            Self::Started { run_id, .. }
            | Self::Done { run_id, .. }
            | Self::Error { run_id, .. } => run_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft() -> CommitDraftWire {
        CommitDraft {
            commit_type: "feat".to_string(),
            scope: Some("modal".to_string()),
            subject: "add AI commit message generation".to_string(),
            body: None,
        }
        .to_wire()
    }

    #[test]
    fn phases_serialize_camel_case_with_a_flat_phase_tag() {
        let started = serde_json::to_value(CommitGenEvent::started("r1", "/repo/a")).unwrap();
        assert_eq!(
            started,
            serde_json::json!({ "phase": "started", "runId": "r1", "path": "/repo/a" })
        );

        let done = serde_json::to_value(CommitGenEvent::done("r1", "/repo/a", draft())).unwrap();
        assert_eq!(done["phase"], "done");
        assert_eq!(done["runId"], "r1");
        assert_eq!(
            done["draft"]["rendered"],
            "feat(modal): add AI commit message generation"
        );
        assert!(done.get("error").is_none(), "{done}");

        let error = serde_json::to_value(CommitGenEvent::error("r1", "/repo/a", "boom")).unwrap();
        assert_eq!(
            error,
            serde_json::json!({ "phase": "error", "runId": "r1", "path": "/repo/a", "error": "boom" })
        );
    }

    #[derive(Default)]
    struct CollectingObserver {
        events: std::sync::Mutex<Vec<CommitGenEvent>>,
    }

    impl CommitGenObserver for CollectingObserver {
        fn on_commit_gen(&self, event: &CommitGenEvent) {
            self.events.lock().unwrap().push(event.clone());
        }
    }

    impl CollectingObserver {
        fn phases(&self) -> Vec<(String, String)> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .map(|event| (event.phase_tag().to_string(), event.path().to_string()))
                .collect()
        }
    }

    fn sample_draft() -> CommitDraft {
        CommitDraft {
            commit_type: "feat".to_string(),
            scope: None,
            subject: "add concurrency".to_string(),
            body: None,
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn every_started_event_precedes_the_first_terminal_event() {
        let observer = Arc::new(CollectingObserver::default());
        let paths: Vec<String> = (0..4).map(|i| format!("/repo/{i}")).collect();

        run_batch_with("run-1", &paths, observer.clone(), |_path| async {
            Ok::<_, CommitGenError>(sample_draft())
        })
        .await;

        let phases = observer.phases();
        assert_eq!(phases.len(), 8, "started + terminal per repo");
        assert!(
            phases[..4].iter().all(|(phase, _)| phase == "started"),
            "all rows must show generating first (US2-AC1): {phases:?}"
        );
        assert!(phases[4..].iter().all(|(phase, _)| phase == "done"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn repositories_generate_in_parallel_not_back_to_back() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::{Duration, Instant};

        let observer = Arc::new(CollectingObserver::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let paths: Vec<String> = (0..3).map(|i| format!("/repo/{i}")).collect();

        let started = Instant::now();
        run_batch_with("run-2", &paths, observer.clone(), {
            let in_flight = Arc::clone(&in_flight);
            let peak = Arc::clone(&peak);
            move |_path| {
                let in_flight = Arc::clone(&in_flight);
                let peak = Arc::clone(&peak);
                async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(60)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, CommitGenError>(sample_draft())
                }
            }
        })
        .await;
        let elapsed = started.elapsed();

        assert_eq!(peak.load(Ordering::SeqCst), 3, "all three ran at once");
        assert!(
            elapsed < Duration::from_millis(150),
            "wall time should track the slowest repo, not the sum (SC-001): {elapsed:?}"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn more_than_eight_repositories_queue_on_the_semaphore() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::time::Duration;

        let observer = Arc::new(CollectingObserver::default());
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let paths: Vec<String> = (0..12).map(|i| format!("/repo/{i}")).collect();

        run_batch_with("run-3", &paths, observer.clone(), {
            let in_flight = Arc::clone(&in_flight);
            let peak = Arc::clone(&peak);
            move |_path| {
                let in_flight = Arc::clone(&in_flight);
                let peak = Arc::clone(&peak);
                async move {
                    let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, CommitGenError>(sample_draft())
                }
            }
        })
        .await;

        let peak = peak.load(Ordering::SeqCst);
        assert!(peak <= CONCURRENCY, "never exceed the cap: {peak}");
        assert!(peak > 1, "must actually run concurrently: {peak}");
        assert_eq!(observer.phases().len(), 24);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn one_failing_repository_does_not_cancel_its_siblings() {
        let observer = Arc::new(CollectingObserver::default());
        let paths = vec!["/repo/ok".to_string(), "/repo/bad".to_string()];

        run_batch_with("run-4", &paths, observer.clone(), |path| async move {
            if path.ends_with("bad") {
                Err(CommitGenError::Provider("offline".to_string()))
            } else {
                Ok(sample_draft())
            }
        })
        .await;

        let events = observer.events.lock().unwrap().clone();
        let done: Vec<_> = events.iter().filter(|e| e.phase_tag() == "done").collect();
        let errors: Vec<_> = events.iter().filter(|e| e.phase_tag() == "error").collect();
        assert_eq!(done.len(), 1);
        assert_eq!(done[0].path(), "/repo/ok");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].path(), "/repo/bad");
        let reason = match errors[0] {
            CommitGenEvent::Error { error, .. } => error.clone(),
            other => panic!("expected an error event, got {other:?}"),
        };
        assert!(reason.contains("offline"), "{reason}");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn retry_emits_one_pair_for_the_failed_path_and_nothing_else() {
        let observer = Arc::new(CollectingObserver::default());
        let paths = vec!["/repo/a".to_string(), "/repo/b".to_string()];

        run_batch_with("run-r", &paths, observer.clone(), |_path| async {
            Ok::<_, CommitGenError>(sample_draft())
        })
        .await;
        observer.events.lock().unwrap().clear();

        run_batch_with(
            "run-r",
            &["/repo/b".to_string()],
            observer.clone(),
            |_path| async { Ok::<_, CommitGenError>(sample_draft()) },
        )
        .await;

        let phases = observer.phases();
        assert_eq!(
            phases,
            vec![
                ("started".to_string(), "/repo/b".to_string()),
                ("done".to_string(), "/repo/b".to_string()),
            ],
            "retry must touch only its own path (FR-015)"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn retry_repo_reuses_the_caller_run_id() {
        let observer = Arc::new(CollectingObserver::default());
        let settings = AiSettings {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "test-model".to_string(),
            api_key_env: "REPOMAN_TEST_MISSING_KEY_XYZ".to_string(),
            max_tool_calls: 12,
            diff_budget_chars: 40_000,
        };

        retry_repo(
            "/tmp/repoman-not-a-repository-xyz",
            "run-shared",
            &settings,
            observer.clone(),
        );

        let mut waited = std::time::Duration::ZERO;
        while observer.events.lock().unwrap().len() < 2
            && waited < std::time::Duration::from_secs(5)
        {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            waited += std::time::Duration::from_millis(20);
        }

        let events = observer.events.lock().unwrap().clone();
        assert_eq!(events.len(), 2, "{events:?}");
        assert_eq!(events[0].phase_tag(), "started");
        assert_eq!(events[1].phase_tag(), "error");
        assert!(events.iter().all(|event| event.run_id() == "run-shared"));
    }

    fn git(repo: &std::path::Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(args)
            .output()
            .expect("git available");
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    fn dirty_repo() -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("repo");
        std::fs::create_dir_all(&path).expect("repo dir");
        git(&path, &["init", "-q"]);
        git(&path, &["config", "user.email", "test@example.com"]);
        git(&path, &["config", "user.name", "Test"]);
        std::fs::write(path.join("README.md"), "hello\n").expect("write");
        git(&path, &["add", "."]);
        git(&path, &["commit", "-q", "-m", "feat: initial"]);
        std::fs::write(path.join("README.md"), "hello world\n").expect("write change");
        (dir, path.to_string_lossy().to_string())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_missing_credential_errors_every_row_and_never_writes() {
        let (_dir_a, repo_a) = dirty_repo();
        let (_dir_b, repo_b) = dirty_repo();
        let status_before = git(std::path::Path::new(&repo_a), &["status", "--porcelain"]);

        let observer = Arc::new(CollectingObserver::default());
        let settings = AiSettings {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "test-model".to_string(),
            api_key_env: "REPOMAN_TEST_UNSET_KEY_ABC".to_string(),
            max_tool_calls: 12,
            diff_budget_chars: 40_000,
        };
        let paths = vec![repo_a.clone(), repo_b.clone()];

        run_batch("run-missing", &paths, &settings, observer.clone()).await;

        let events = observer.events.lock().unwrap().clone();
        assert_eq!(events.len(), 4, "started + error for every row");
        assert!(events[..2]
            .iter()
            .all(|event| event.phase_tag() == "started"));
        for event in &events[2..] {
            let reason = match event {
                CommitGenEvent::Error { error, .. } => error.clone(),
                other => panic!("expected error, got {other:?}"),
            };
            assert!(
                reason.contains("missing-credential:REPOMAN_TEST_UNSET_KEY_ABC"),
                "{reason}"
            );
            assert!(!reason.contains("sk-"), "no secret material in reasons");
        }
        assert!(events.iter().all(|event| event.run_id() == "run-missing"));

        // Principle V / SC-002: generation only must not touch the work tree.
        let status_after = git(std::path::Path::new(&repo_a), &["status", "--porcelain"]);
        assert_eq!(status_before, status_after);
    }

    #[test]
    fn events_round_trip_and_accessors_match_the_payload() {
        let original = CommitGenEvent::done("r2", "/repo/b", draft());
        let json = serde_json::to_string(&original).unwrap();
        let parsed: CommitGenEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.run_id(), "r2");
        assert_eq!(parsed.path(), "/repo/b");
        match parsed {
            CommitGenEvent::Done { draft, .. } => assert_eq!(draft.commit_type, "feat"),
            other => panic!("unexpected phase: {other:?}"),
        }
    }
}
