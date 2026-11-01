# Phase 0 Research: Live Scan Progress & Activity Log

All mechanism-level decisions were settled during design; ADR__261101-1 is the
authoritative record. This file resolves the remaining implementation-level
unknowns against the actual code.

## R1. Progress transport

- **Decision**: `AppEvent::ScanProgress(ScanProgressEvent)` on the existing `EventBus`; desktop maps it to Tauri event `scan:progress`, web maps it to SSE event name `scan-progress` on the existing single `/api/events` stream.
- **Rationale**: ADR__261101-1; watcher.rs:19 and web.rs:222 show the bus + both transports are a two-line extension.
- **Alternatives considered**: dedicated polling command, direct `AppHandle` emit in the scanner — both rejected in the ADR.

## R2. Sink API between service and scanner

- **Decision**: a new `ScanObserver` trait defined **in `scanner.rs`** (payload struct alongside it, mirroring `GitObserver`/`GitStepEvent` living together in `git_log.rs`), `Send + Sync`, with `on_progress(&ScanProgressEvent)`. `EventBus` implements it exactly like it implements `GitObserver` (watcher.rs:37). `scan_path` takes `&dyn ScanObserver`; `spawn_blocking` receives an `Arc<dyn ScanObserver>`.
- **Rationale**: keeps scanner Tauri-free, testable with a collecting fake, and stylistically identical to the established observer pattern.
- **Alternatives considered**: `Fn(&ScanProgressEvent)` closure (harder to name/document in signatures, no NullObserver analog); `mpsc::Sender` threaded through `scan_path` (works but re-invents what `GitObserver` already solved).

## R3. Event phases and their emitters

- **Decision**:
  - `started { run_id, roots, started_at }` — service (top of `discover_repositories`)
  - `found { run_id, root, path, name }` — scanner, inside the `WalkBuilder` filter when `.git` is hit
  - `tick { run_id, root, current_dir }` — scanner, throttled (R4)
  - `inspected { run_id, path, kind }` — service, as each `inspect_repository` task completes
  - `done { run_id, found_count, elapsed_ms, error: Option<String> }` — service, always (even on `JoinError`/`Err`)
- **Rationale**: matches ADR__261101-1; `found` fires instantly from the walk, `inspected` fills the type badge later so the row updates in place (FR-005). The existing inspection uses a bounded `Semaphore` (service.rs:133) — emit from inside each spawned task as it finishes; ordering is completion order, which is honest for a log.
- **Alternatives considered**: emitting only after inspection (the walk stays silent — rejected in ADR); single combined event kind discriminated by Option fields (loses exhaustive matching; rejected).

## R4. Throttling the tick stream

- **Decision**: `tick` is emitted from the blocking walk at most once per 400 ms using a `Mutex<Instant>`/last-emit comparison in the closure (same constant value as the watcher's `DEBOUNCE`). Heartbeat floor: while a root is being walked, a tick is re-emitted at least every ~500 ms even if the directory did not change, so SC-001's "≥2 updates/sec" is a guarantee, not a side effect of traffic. `found`/`inspected`/`started`/`done` are never throttled.
- **Rationale**: the walker visits ~100k dirs; 400 ms ≈ the max useful UI rate (spec: ≥2 updates/sec). Reusing 400 ms keeps one recognizable constant in the codebase.
- **Alternatives considered**: emit per directory + drop in the frontend (floods both transports); 1 s interval (violates SC-001 "at least twice per second").

## R5. Broadcast capacity and loss semantics

- **Decision**: raise `EventBus::new` capacity from 512 to 8192 events. Accept that in an extreme burst (thousands of repos discovered in seconds) the broadcast may still drop for a lagging receiver; therefore `done` carries the authoritative `found_count`, and the frontend uses `max(counted entries, done.found_count)` for the header/summary. The returned `Vec<Repository>` from the command remains the source of truth for the table.
- **Rationale**: `tokio::sync::broadcast` (watcher.rs:24) drops for slow receivers by design; the modal is a *progress view*, not an inventory, so eventual summary-correctness satisfies FR-002/FR-013 without building a replay buffer.
- **Alternatives considered**: per-subscriber unbounded queue (redesigns the bus for one feature — rejected); batching `found` events into arrays on the bus (works, but 8192 capacity + summary correction is simpler; can be revisited if testers see gaps).

## R6. Run scoping

- **Decision**: every event carries `run_id` (uuid v4, generated per `discover_repositories` call — same pattern as `execute_group_action`'s `run_id`, service.rs:209). The frontend ignores events whose `run_id` ≠ the current run and resets state on each `started`.
- **Rationale**: FR-009 (a modal shows exactly one run); stale late events from a cancelled/previous scan must not leak in.
- **Alternatives considered**: sequence numbers (more state to reconcile); timestamp ordering (clock jitter).

## R7. Frontend state shape & timer

- **Decision**: in `repoStore.ts`, a `scanActivity` slice: `runId`, `roots`, `running`, `currentDir`, `entries: Map<path, { name, root, kind?, inspected }>` (Map preserves insertion order; `inspected` mutates in place), `finished: { count, elapsedMs, error? } | null`, `modalOpen`. Elapsed time is a frontend 1 Hz interval that runs only while `running` (stopped on `done`); Rust does not re-render a timer.
- **Rationale**: keeps all three views (onboarding line, AllReposPage empty state, modal) reading one store; the command promise still resolves the final repository list as today, so nothing else changes behaviorally.
- **Alternatives considered**: component-local state per view (triplicates the reducer logic); Rust-side elapsed ticks (event noise for a UI-only value).

## R8. Render cap and truncation

- **Decision**: modal log renders `computed(() => entriesSlice)` = last 500 entries with a synthetic "…und N mehr" row on top when truncated (FR-013); `currentDir` is rendered with a middle-ellipsis helper and full path in `title` (FR-014). Auto-scroll follows new entries unless the user has scrolled up (track via one scroll listener).
- **Rationale**: 500 rows ≈ one screenful × several; a plain `v-for` over ≤500 simple rows needs no virtualization dependency.
- **Alternatives considered**: windowed list library (new dependency, overkill at 500 rows).

## R9. Error/termination contract

- **Decision**: `discover_repositories` wraps everything so **every exit path emits `done`** — per-root walk failures collect into `error: Some(joined messages)` while successful roots still deliver repos; a `spawn_blocking` `JoinError` produces `done { error }` too. On the web side the SSE map is infallible (serde).
- **Rationale**: SC-006 demands zero hanging running-states; the command already returns `Result`, but the frontend spinner is driven by the promise *and* must terminate if the transport dies — the `done` event plus promise settlement cover both.
- **Alternatives considered**: frontend timeout heuristic (arbitrary, fires on legitimately slow scans).

## R10. i18n keys

- **Decision**: new keys under `scan.*` (modal title, header labels, summary, error, "and N more", chip tooltip) plus updated `onboarding.find.scanning` / `allRepos.checking` to interpolation forms (`{dir}`, `{count}`). Both `messages.de.ts` and `messages.en.ts` ship together (FR-012, ADR__261031-1).

## Open questions

None — no NEEDS CLARIFICATION items remain from the spec.
