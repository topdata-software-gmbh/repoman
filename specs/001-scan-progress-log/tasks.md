---
description: "Task list for Live Scan Progress & Activity Log"
---

# Tasks: Live Scan Progress & Activity Log

**Input**: Design documents from `specs/001-scan-progress-log/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/scan-progress-event.md, quickstart.md

**Tests**: Included — plan.md Technical Context/Testing and the design's validation section require cargo + vitest coverage; quickstart §1 makes them gates.

**Organization**: Tasks grouped by user story (US1 P1 onboarding progress, US2 P2 activity modal, US3 P3 completion/refresh). The whole backend event pipeline is foundational so each story is purely frontend on top of it.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no pending dependency)
- **[Story]**: US1 / US2 / US3

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Baseline safety before touching scanner/service code

- [X] T001 Verify baseline green before any change: run `cargo test` from `src-tauri/`, then `pnpm test` and `pnpm typecheck` from repo root; record results

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The full scan-progress stream end-to-end (Rust emitter → both transports → TS types → store) so every story reads live data.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Define `ScanProgressEvent` payload and `ScanObserver` trait in `src-tauri/src/scanner.rs` (mirrors `GitObserver`/`GitStepEvent` in `git_log.rs`): serde camelCase fields exactly per contracts §C2; `phase` ∈ {started, found, tick, inspected, done}; one per-phase constructor (e.g. `ScanProgressEvent::found(run_id, root, path, name)`) so invalid field combos are unconstructable (data-model E3); trait is `Send + Sync` with `on_progress(&ScanProgressEvent)` plus a `NullObserver`-style no-op default for tests
- [X] T003 Extend `AppEvent` with `ScanProgress(ScanProgressEvent)` in `src-tauri/src/events.rs`
- [X] T004 In `src-tauri/src/watcher.rs`: implement `ScanObserver for EventBus` (clone + `sender.send`, same as `impl GitObserver` at line 37) and raise broadcast capacity from 512 to 8192 in `EventBus::new` (research R5)
- [X] T005 Refactor `scan_path`/`scan_path_async` in `src-tauri/src/scanner.rs` to take `&dyn ScanObserver` (`Arc<dyn ScanObserver>` into `spawn_blocking`) and emit: `found` for every `.git` hit alongside the existing push, `tick { current_dir }` throttled to "at most 1 per 400 ms per run" using a `std::time::Instant` last-emit check in the filter closure, plus a heartbeat floor — re-emit the latest tick at least every ~500 ms while a root is being walked so SC-001's ≥2 updates/sec holds on sparse trees (research R4); returned `Vec<PathBuf>` unchanged; update the existing tests in the same file to use a collecting fake observer
- [X] T006 In `src-tauri/src/service.rs`: give `discover_repositories` an observer parameter (callers pass `Arc<dyn ScanObserver>`); generate `run_id` uuid v4 per call; emit `started { roots, started_at }` first; feed the per-root walk observer; emit `inspected { path, kind }` from each inspection task as it completes (completion order is fine, research R3); emit `done { found_count, elapsed_ms, error }` on EVERY exit path — per-root walk failures collect into `error`, `spawn_blocking` `JoinError` also produces `done { error }` (research R9) — and update the command wrapper in `src-tauri/src/commands/git_commands.rs:8` to pass `State<'_, EventBus>` through
- [X] T007 Add transport mappings: forward `AppEvent::ScanProgress` as `handle.emit("scan:progress", event)` in the setup loop of `src-tauri/src/main.rs:99` and as SSE event name `scan-progress` with JSON data in the `/api/events` map of `src-tauri/src/web.rs:222` (one stream, no new endpoint)
- [X] T008 [P] Add the TS mirror `ScanProgressEvent` interface to `src/types/domain.ts` exactly as contracts §C4 (reuse existing `RepoKind`)
- [X] T009 Extend `subscribeToEvents` in `src/lib/backend.ts`: `AppEventHandlers.onScanProgress`; desktop = `listen<ScanProgressEvent>('scan:progress', …)` alongside the existing two; web = `source.addEventListener('scan-progress', …)` with JSON.parse wrapped so "a malformed frame must never break the live view" (same guard as `git-step`)
- [X] T010 Implement the `scanActivity` slice in `src/stores/repoStore.ts` per data-model E1: state `runId, roots, running, currentDir, startedAt, entries: Map<path,{name,root,kind?,inspected}>, finished: {count, elapsedMs, error?} | null, modalOpen`; reducer rules: ignore events whose `runId` ≠ current (FR-009), full reset on each `started`, `found` inserts once (dedup by path — "no repository appears twice"), `tick` sets `currentDir`, `inspected` mutates the existing entry's `kind` in place and never appends, `done` sets `finished` with `running=false` and header count = `max(entries.size, foundCount)` (research R5); wire `onScanProgress` into the existing store event subscription
- [X] T011 Add store unit tests in `src/lib/__tests__/scanActivity.test.ts` covering every T010 reducer rule (started reset / found dedup / inspected-in-place / tick / done reconciliation / late-event-by-other-runId ignored)

**Checkpoint**: Stream emits on both transports and lands in the store — UI work can begin; `pnpm tauri dev` shows console-reachable progress state.

---

## Phase 3: User Story 1 - Watch a scan make progress during onboarding (Priority: P1) 🎯 MVP

**Goal**: Onboarding step 0 shows the directory being processed and a live found count instead of static spinner text (FR-001, FR-002, FR-012; SC-001, SC-002).

**Independent Test**: quickstart §2 — onboarding scan of `/tmp/scan-demo` must show a changing directory line within 2 s and a count rising within 1 s of each discovery.

### Implementation for User Story 1

- [ ] T012 [US1] Add DE + EN message keys to `src/lib/i18n/messages.de.ts` and `src/lib/i18n/messages.en.ts`: `onboarding.find.progress` with `{dir}` and `{count}` interpolation (e.g. "Prüfe {dir}… · {count} Projekte gefunden" / "Checking {dir}… · {count} projects found" — user-facing DE copy uses "Projekte", matching the existing vocabulary, never "Repos"), parity enforced by the existing i18n test
- [ ] T013 [US1] Replace the static `t('onboarding.find.scanning')` paragraph (line ~140) in `src/components/onboarding/OnboardingStepFind.vue` with the live line bound to `store.scanActivity` (`currentDir`, entries count); fall back to the old text when no `started` event has arrived yet; submit/step-advance flow unchanged
- [ ] T014 [P] [US1] Component test in `src/components/__tests__/onboarding-find-progress.test.ts`: with a seeded store slice, the rendered line contains the current dir and count and updates when tick/found events are pushed through the reducer

**Checkpoint**: MVP — onboarding scans visibly work live. Independently shippable.

---

## Phase 4: User Story 2 - Inspect the live activity log via a modal (Priority: P2)

**Goal**: Clickable "Suche läuft…" chip opens the live Scan activity modal with header (dir + count + elapsed) and append-only per-repo log with type badges (FR-003–FR-006, FR-013, FR-014; SC-003).

**Independent Test**: quickstart §2 step 3 — mid-scan the modal shows all rows found so far, none duplicated; rows gain badges without duplicating; closing/reopening doesn't disturb onboarding.

### Implementation for User Story 2

- [X] T015 [P] [US2] Add DE + EN `scan.*` keys to `src/lib/i18n/messages.de.ts` / `src/lib/i18n/messages.en.ts`: modal title, found label, elapsed label, kind-pending badge text, "…und {n} mehr" overflow row, open-log chip tooltip
- [X] T016 [US2] Create `src/components/ScanActivityModal.vue` using the same ui-kit modal primitives as the existing modals (e.g. `src/components/AddScanPathModal.vue`): header = `currentDir` rendered with middle-ellipsis helper and full path in `title` (FR-014), exact found count, elapsed timer via "a frontend 1 Hz interval that runs only while running" (research R7); body = `v-for` over "the last 500 entries" of `store.scanActivity.entries` with a synthetic "…und N mehr" first row when truncated (FR-013), newest at bottom, auto-scroll that "stops for that user action" once the user scrolls up; rows show name + root + kind badge (dim pending until inspected); error/summary rendering hooks read `finished` (used from US3)
- [X] T017 [US2] Make the "Suche läuft…" chip at `src/components/Sidebar.vue:231` a button that calls `store.openScanModal()` while scanning (FR-003) and mount `ScanActivityModal` at the app-level modal/drawer host in `src/App.vue` (next to `GitLogDrawer`/existing modals); reopening must not affect the onboarding flow (US2-AC4)
- [X] T018 [P] [US2] Component test in `src/components/__tests__/scan-activity-modal.test.ts`: insertion-order rows with no duplicates, `inspected` updates badge in place, >500 entries shows exactly 500 rows + "und N mehr" with header count exact, middle-ellipsis + `title` on long dirs

**Checkpoint**: Full live inspection available for any scan (onboarding or refresh) via one modal.

---

## Phase 5: User Story 3 - See the result and stay in control when the scan ends (Priority: P3)

**Goal**: Trustworthy finish: summary stays readable until closed, chip lingers ~10 s clickable, errors never hang, multi-root attribution, refresh flow wired to the same visuals (FR-007–FR-011, FR-015; SC-004, SC-006).

**Independent Test**: quickstart §3 (multi-root refresh) + §4 (kill the folder mid-scan) + §2 step 4 (summary + chip timeout).

### Implementation for User Story 3

- [X] T019 [US3] Implement the finished/summary state in `src/components/ScanActivityModal.vue` (header becomes "Scan fertig — {count} Projekte in {elapsed}"; log stays readable until user closes) and the chip lifecycle in `src/components/Sidebar.vue`: after `done` the chip shows the summary and "MUST remain clickable showing that run's summary for approximately 10 seconds before disappearing" (FR-015) — one timer driving `store.scanActivity.summaryVisible` (data-model E1), cleared on reopen/new run
- [X] T020 [US3] Harden termination paths in `src/stores/repoStore.ts`: `done { error }` or a rejected `discoverRepositories` promise must both end `running` and surface the message (FR-008, SC-006); ensure a second `started` while an old summary is displayed resets everything (FR-009/US3-AC4); extend `src/lib/__tests__/scanActivity.test.ts` with error-promise and reset-on-second-start cases
- [X] T021 [US3] Multi-root attribution in `src/components/ScanActivityModal.vue`: each row shows its originating `root` (shortened, FR-010); header names the root currently being processed; verify against the web-mode path with a second scan folder per quickstart §3
- [X] T022 [US3] Replace the static `t('allRepos.checking', …)` line at `src/components/AllReposPage.vue:200` with the same live dir+count binding as US1 (FR-011: one progress stream for onboarding AND refresh), keeping the existing "Prüfe {folders}" text as pre-`started` fallback
- [X] T023 [P] [US3] Test in `src/components/__tests__/scan-activity-modal.test.ts`: after `done` header shows summary incl. elapsed and log persists; error `done` renders error state and chip clears (covers US3-AC1..AC4 view side)

**Checkpoint**: All three stories independently functional; refresh and onboarding share identical live behavior.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T024 Run the full automated gate set: `cargo test` in `src-tauri/`, `pnpm test`, `pnpm typecheck`, `pnpm build` — all green
- [ ] T025 [P] Manual end-to-end pass through `specs/001-scan-progress-log/quickstart.md` §2–§6, including the SC-005 overhead comparison ("end-to-end scan duration grows by no more than 5% versus today's silent scan") on `/topdata`; note results
- [ ] T026 [P] i18n parity check: run existing `src/lib/__tests__/i18n.test.ts`; confirm zero untranslated new strings (FR-012)
- [ ] T027 Update `README.md` (scan-feedback mention) and `CHANGELOG.md` entry for the feature

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1**: immediate
- **Phase 2 (Foundational)**: after T001; BLOCKS all stories (backend + transports + store must exist before any view binds)
- **US1 (P1)**: after Phase 2 — MVP
- **US2 (P2)**: after Phase 2; shares only the store with US1 → can build in parallel with US1
- **US3 (P3)**: after Phase 2; T019/T023 extend files created in US2 (T016) — do US2 before US3's modal tasks
- **Polish**: after all delivered stories

### Within Each User Story

Tests bundled with their component; no story ships with red T011/T014/T018/T020/T023 suites.

### Parallel Opportunities

- T002→T006 sequential (Rust core); T008 ∥ anything in Rust (TS file); T009/T010 sequential (helper then store)
- After Phase 2: US1 (T012–T014, wizard files) ∥ US2 (T015–T018, modal/sidebar files) — disjoint paths except shared i18n files (edit once per story branch or order T012 before T015)
- T024/T025/T026/T027 mutually parallel except T024 gates T025

## Parallel Example: User Story 2

```bash
Task: "T015 [P] [US2] scan.* i18n keys in src/lib/i18n/messages.de.ts / messages.en.ts"
Task: "T016 [US2] Create src/components/ScanActivityModal.vue"
# after T016:
Task: "T017 [US2] Chip button in src/components/Sidebar.vue + mount in src/App.vue"
Task: "T018 [P] [US2] Test src/components/__tests__/scan-activity-modal.test.ts"
```

## Implementation Strategy

### MVP First (US1)

1. T001 → 2. Phase 2 (T002–T011) → 3. US1 (T012–T014) → 4. **STOP & VALIDATE** via quickstart §2 → demo.

### Incremental Delivery

US2 (modal + chip) → validate §2.3 → US3 (summary/error/multi-root/refresh) → validate §3–§6. Each checkpoint ships without breaking prior stories because the stream contract (contracts/scan-progress-event.md) is frozen in Phase 2.

---

## Notes

- Constraint literals repeated verbatim where they apply (throttle 400 ms, capacity 8192, cap 500 rows, ~10 s chip) so implementers never guess.
- Do not touch the returned `Vec<Repository>` flow — results table remains the inventory source of truth (data-model E5).
- Commit after each checkpoint.
