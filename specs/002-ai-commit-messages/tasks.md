---
description: "Task list for AI-Generated Commit Messages"
---

# Tasks: AI-Generated Commit Messages

**Input**: Design documents from `specs/002-ai-commit-messages/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/generate-commit-messages-command.md, contracts/commitgen-events.md, quickstart.md

**Tests**: Included — plan.md §Technical Context/Testing and constitution "Development Workflow & Quality Gates" require `cargo test` + `vitest` coverage for every new behavior including error paths; quickstart.md §Automated gates makes them blocking. One `#[ignore]`d live-provider test mirrors sb practice.

**Organization**: Tasks grouped by user story (US1 P1 generate/review/commit, US2 P2 concurrency+live status, US3 P2 isolated failure+retry, US4 P3 bounded repo inspection, US5 P3 graceful degradation). The shared contract layer (settings, draft schema, diff collection, event bus, transports, store state machine) is foundational so every story is a thin increment on top of a frozen wire format.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no pending dependency)
- **[Story]**: US1 / US2 / US3 / US4 / US5

## Path Conventions

Single workspace (plan.md §Project Structure): Rust core in `src-tauri/src/`, frontend in `src/`. `commitgen/` is the ONLY module allowed to `use serdes_ai` (structure decision — swap-risk isolation).

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Pin the new dependency and verify baseline before touching anything.

- [x] T001 Verify baseline green before any change: run `cargo test` and `cargo clippy --all-targets -- -D warnings` from `src-tauri/`, then `pnpm vitest run` and `pnpm typecheck` from repo root; record results (constitution quality gates)
- [x] T002 Add the LLM dependency to `src-tauri/Cargo.toml` exactly per research R1: `serdes-ai = { git = "https://github.com/janfeddersen-wq/serdesAI", tag = "v0.3.0" }` (facade crate), plus whatever in-workspace pieces it exposes (`serdes-ai-output`, retries) only if the facade does not re-export them; commit the resulting `src-tauri/Cargo.lock` (binary crate must pin; pre-1.0 dependency mitigated by the seam + pin)
- [x] T003 [P] Create the empty seam module `src-tauri/src/commitgen/mod.rs` (declare `settings`, `draft`, `diff`, `agent`, `tools` submodules with `pub use` stubs) and add `mod commitgen;` to `src-tauri/src/main.rs` so later phases never touch module wiring

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The frozen contract end-to-end — persisted settings, draft schema + renderer, read-only diff collection, typed event on both transports, TS mirror types, and the frontend state machine. No LLM call is made in this phase (a mock/empty provider is acceptable); it must compile and pass tests without network.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

### Settings & persistence

- [x] T004 Implement `AiSettings` in `src-tauri/src/commitgen/settings.rs` per data-model E1, serde camelCase: `base_url: String`, `model: String`, `api_key_env: String` (**default `"OPENAI_API_KEY"`, stores only the VARIABLE NAME — the credential value is never persisted, FR-013/SC-006**), `max_tool_calls: u32` (default **12**), `diff_budget_chars: usize` (default **40000**); add `fn validate(&self) -> Result<(), AiConfigError>` requiring `base_url` to parse as `https?://…` (invalid → distinct `badConfig` outcome, data-model E1 rule 2)
- [x] T005 Add the optional settings block: `pub ai: Option<AiSettings>` with `#[serde(default)]` on the `AppConfig` struct in `src-tauri/src/models.rs:107` (plan lists `config.rs` — the struct actually lives in `models.rs`; `ConfigManager::save`/`snapshot` in `src-tauri/src/config.rs:59/41` already serialize the whole struct via the atomic temp-file-rename writer — do NOT change the write path, Principle V)
- [x] T006 [P] Unit tests in `src-tauri/src/commitgen/settings.rs` `#[cfg(test)]`: defaults (12 / 40000 / OPENAI_API_KEY), absent `ai` deserializes to `None`, round-trips through `ConfigManager` keep `api_key_env` name and never a value, invalid `baseUrl` rejected by `validate()`

### Draft schema & rendering (backend-owned, single source of truth)

- [x] T007 Implement `CommitDraft` in `src-tauri/src/commitgen/draft.rs` per data-model E2: fields `commit_type` (serialized as `type`), `scope: Option<String>`, `subject: String`, `body: Option<String>`; implement the serdesAI `OutputSchema` with the type enum **exactly `feat|fix|refactor|perf|docs|style|test|build|ci|chore|revert`**, `scope` lowercase-hyphen normalized, `subject` **non-empty, ≤ 72 chars after trim, no trailing period**, `body` optional; wire form is camelCase `{ type, scope, subject, body, rendered }`
- [x] T008 Implement `CommitDraft::render(&self) -> String` in `src-tauri/src/commitgen/draft.rs` (research R2): `"{type}({scope}): {subject}"` when scope present else `"{type}: {subject}"`, then blank line + `body` when present; `rendered` is computed once here so DE/EN or trimming edge cases never diverge between transports (data-model E2)
- [x] T009 [P] Schema/render unit tests in `src-tauri/src/commitgen/draft.rs` `#[cfg(test)]` covering **without any live model** (research R2): all 11 enum values accepted, unknown type rejected, 73-char subject rejected / 72 accepted, trailing period rejected, lowercase-hyphen normalization, render with and without scope/body

### Read-only git access (Principle III — system CLI only)

- [x] T010 Add read-only helpers to `src-tauri/src/git_runner.rs` reusing the existing private `run_git_stdout` (line 20): `diff_head(repo_path)` → `git diff HEAD --no-color`; `untracked_files(repo_path)` → `git status --porcelain --untracked-files=all` parsed into paths; `show_head_file(repo_path, rel)` → `git show HEAD:<file>`; `recent_subjects(repo_path, n)` → `git log -n <n> --format=%s`; plus `has_conflicts(repo_path)` from porcelain unmerged codes (UU/AA/etc.). Every helper is read-only — no writes, no staging (Principle V)
- [x] T011 Implement change-detail collection in `src-tauri/src/commitgen/diff.rs` per research R4: diff via `GitRunner::diff_head` + first **≤ 200 lines per file** of each untracked file (read from disk, capped by lines not bytes) + the last **10** commit subjects for style alignment (FR-003), then hard-truncate the whole payload to `diff_budget_chars` (default **40000**) appending the literal marker **`[... truncated ...]`**; empty change set → `Err("nothing to summarize")`; conflict state → `Err("merge conflicts — resolve first")` (contract C4 rows 3–4). Large diffs MUST truncate, never fail or hang (edge case, FR-009)
- [x] T012 [P] Truncation/detection tests in `src-tauri/src/commitgen/diff.rs` `#[cfg(test)]` using a temp git repo (existing `tempfile` dev-dependency, no LLM): 200-line-per-file untracked cap, budget respected with marker present on a synthetic 50 000-char diff, no-pending-changes error, conflict-state error, subjects capped at 10

### Event contract + both transports (Principle II, ADR__261030-4)

- [x] T013 Define `CommitGenEvent` in `src-tauri/src/commitgen/mod.rs` per data-model E3 with serde camelCase (mirror `ScanProgressEvent` style): `run_id: String`, `path: String`, `phase: CommitGenPhase` (`started | done | error`), `draft: Option<CommitDraftWire>` (phase=done), `error: Option<String>` (phase=error); one per-phase constructor (`started(run_id, path)`, `done(run_id, path, draft)`, `error(run_id, path, reason)`) so invalid field combos are unconstructable
- [x] T014 Add `CommitGen(CommitGenEvent)` to the `AppEvent` enum in `src-tauri/src/events.rs` and declare `pub trait CommitGenObserver: Send + Sync { fn on_commit_gen(&self, event: &CommitGenEvent); }` in `src-tauri/src/service.rs` (plan §Project Structure) with a `NullCommitGenObserver`-style no-op default for tests (mirrors `GitObserver` at `service.rs:277` / `ScanObserver` usage)
- [x] T015 Implement `CommitGenObserver for EventBus` in `src-tauri/src/watcher.rs:19` (clone + `sender.send`, identical to the existing `impl GitObserver` at line 38 and `impl ScanObserver` at line 44; broadcast capacity is already 8192 at line 25 — do not lower it)
- [x] T016 Add the transport mappings (both are one-line delegators, Principle II): forward `AppEvent::CommitGen(e)` as `handle.emit("commitgen:repo", e)` inside the existing match in `src-tauri/src/main.rs:99-111`, and map it to SSE event name `commitgen-repo` with the JSON payload in the `/api/events` `filter_map` of `src-tauri/src/web.rs:222-232` (same convention as `git-step` / `scan-progress`, one stream, no new endpoint)
- [x] T017 [P] Add the TS mirror types to `src/types/domain.ts` exactly as contracts/commitgen-events.md §C2: `CommitDraft` (`{ type, scope, subject, body, rendered }`), `CommitGenPhase = 'started' | 'done' | 'error'`, `CommitGenEvent` (`{ runId, path, phase, draft?, error? }`), and the frontend row type `RepoDraftRow` (`{ path, name, status: 'idle'|'generating'|'ready'|'error', message, dirty, error }`) per data-model E4
- [x] T018 [P] Extend `src/lib/backend.ts`: add `generateCommitMessages(paths: string[]): Promise<string /* runId */>` following the existing `request(...)` pattern at line 171 (desktop `invoke('generate_commit_messages', { repoPaths })` / web `POST /api/repositories/generate-commit-messages` with `{ repoPaths }`), and add `onCommitGenRepo?: (event: CommitGenEvent) => void` to `AppEventHandlers` wired in `subscribeToEvents` (`backend.ts:226`): desktop `listen<CommitGenEvent>('commitgen:repo', …)` alongside lines 230-232, web `source.addEventListener('commitgen-repo', …)` with the same JSON.parse guard so "a malformed frame must never break the live view" (as done for `git-step`/`scan-progress`, lines 263-265)

### Frontend state machine (serves US1, US2, US3, US5)

- [x] T019 Implement the `commitgen` slice in `src/stores/repoStore.ts` (setup-style store at line 113) per data-model E4: state `activeRunId: string | null`, `rows: Map<path, RepoDraftRow>`; `startRun(paths)`: call `backend.generateCommitMessages(paths)`, store `runId`, seed one `idle` row per path (row exists ONLY for the modal's current selection); `handleCommitGenEvent(event)`: ignore events whose `runId ≠ activeRunId` (late events after close), `started` → `status='generating'` and clear `dirty` for that path, `done` → `status='ready'` + `message = draft.rendered` **but ignored when `dirty` is true** (user edits win), `error` → `status='error'` + `error` reason, manual text in an error row makes it commit-eligible (US3-AC3); `retryRepo(path)`: re-run the single path under the SAME `runId` (fresh `started`/terminal pair, siblings untouched); expose a `canCommit` computed = every selected repo has a non-empty `message` and no row is `generating` (FR-006/US2-AC3); discard the whole slice when the save modal closes (session-only, data-model E4 diagram)
- [x] T020 [P] Store tests in `src/lib/__tests__/commitgen.test.ts` (vitest + Pinia, mirroring `src/lib/__tests__/scanActivity.test.ts` conventions): started/done/error transitions, unknown-runId ignored, dirty flag blocks a late `done` from overwriting edits, `retry` resets only that path, `canCommit` false while any row generating or any message empty, error row + typed text becomes committable, close discards state (FR-007, FR-008, FR-015, US1-AC3)

**Checkpoint**: Contract + settings + state machine frozen; `cargo test`, `pnpm vitest run`, `pnpm typecheck` green with no network access. UI can now bind to a run whose events are produced by a fake observer.

---

## Phase 3: User Story 1 - Generate per-repo messages, review, then commit (Priority: P1) 🎯 MVP

**Goal**: One control generates one Conventional-Commit draft per selected repository from that repository's own pending changes, previewed as editable rows; the commit then uses each row's final text and generation never commits (FR-001–FR-006, FR-012; SC-002, SC-003).

**Independent Test**: quickstart §Prerequisites + rows 3–5 — configure a model, select two repos with different changes, generate, verify two distinct drafts each describing only its own repo, edit one, commit, check `git log -1` per repo; and closing the dialog commits nothing.

### Implementation for User Story 1

- [x] T021 [US1] Build the agent wiring in `src-tauri/src/commitgen/agent.rs` per research R2/R7/R8: construct one OpenAI-compatible serdesAI client per run from `AiSettings` (`base_url`, `model`), read the credential **at run start** from `std::env::var(api_key_env)` on the server process (absent → typed error carrying the variable NAME so rows can name it — the value itself must never enter an error string, log, or payload, FR-013/SC-006); expose `generate_for_repo(repo_path, &AiSettings, credential, change_detail) -> Result<CommitDraft, CommitGenError>` using `Agent<RepoCtx, CommitDraft>` with structured output and serdesAI retries/backoff; **no inspection tools in this phase** (US4 adds them)
- [x] T022 [US1] Add the single system prompt as one Rust `const SYSTEM_PROMPT` in `src-tauri/src/commitgen/agent.rs` per research R8 (repo name, recent style subjects, Conventional Commit rules, "answer from the change detail; inspect only when it is insufficient" guidance) and a golden-file test in `src-tauri/src/commitgen/agent.rs` `#[cfg(test)]` asserting the const equals `src-tauri/src/commitgen/testdata/system_prompt.golden` (create the golden file) so prompt wording is visible in review diffs
- [x] T023 [US1] Implement `run_all(repo_paths: &[String], settings: &AiSettings, observer: &dyn CommitGenObserver) -> String /* runId */` orchestration in `src-tauri/src/commitgen/mod.rs`: uuid v4 `run_id`; snapshot settings once (mid-run config changes have no effect, data-model E1 rule 3); per repo emit `started(path)` → collect diff (`commitgen::diff`) → run agent → emit `done(path, draft+rendered)` or `error(path, human-readable reason)`; each repo's failure is contained to its own path (never abort the batch)
- [x] T024 [US1] Add the thin core entry `generate_commit_messages(repo_paths: Vec<String>, config: &ConfigManager, observer: Arc<dyn CommitGenObserver>) -> Result<String, String>` in `src-tauri/src/service.rs` per contracts/generate-commit-messages-command.md: reject an empty list with `Err("no repositories selected")`; reject duplicate paths and paths whose git dir does not resolve (`GitRunner::resolve_git_dir`) BEFORE any task spawns (C2); return `Err("ai-not-configured")` when `config.ai` is `None` or `AiSettings::validate()` fails (C4 row 1); otherwise snapshot settings and hand off to `commitgen::run_all` — no formatting, no task logic here (Principle II)
- [x] T025 [US1] Add the Tauri command `#[tauri::command] pub async fn generate_commit_messages(repo_paths: Vec<String>, config: State<'_, ConfigManager>, event_bus: State<'_, EventBus>) -> Result<GenerateCommitMessagesResponse, String>` in `src-tauri/src/commands/git_commands.rs` (pattern: `commit_repo_changes` at line 39, response struct `{ run_id }` serialized camelCase → `{ "runId": … }`), and register it in the `invoke_handler!` list in `src-tauri/src/main.rs:113`
- [x] T026 [US1] Add the web route in `src-tauri/src/web.rs`: `POST /api/repositories/generate-commit-messages` (next to `/api/repositories/commit` at line 326, behind the same `require_token` layer at line 331) deserializing `{ repoPaths }`, delegating to the identical `service::generate_commit_messages`, returning **HTTP 202 Accepted** with `{ "runId": … }` (contract C3)
- [x] T027 [US1] Replace the single shared textarea in `src/components/SaveChangesModal.vue` (currently `message` ref at line 22, textarea block lines 198-210) with the per-repo row list per data-model E4 and FR-004/FR-005: one row per selected repo (repo name, status badge, editable textarea bound to `row.message`, retry icon slot reserved for US3), seeded from `store.commitgen.rows`; drafts are review-only — generation must not call `commitRepositories`
- [x] T028 [US1] Wire the modal flow in `src/components/SaveChangesModal.vue`: add the "✨ Generate with AI" button that calls `store.startRun(repoPaths)` for the current selection; change the commit path at line 119 so each repo commits **its own row's final text** (extend/reuse `store.commitRepositories` to accept per-path messages rather than one shared string, leaving the actual commit command and its gating rules otherwise unchanged, FR-005); gate the submit button (line 241, `canSave` at line 32) on `store.commitgen.canCommit` plus the existing repo-selection rule (FR-006); closing the modal discards the slice (US1-AC3)
- [x] T029 [US1] Add DE + EN keys to `src/lib/i18n/messages.de.ts` and `src/lib/i18n/messages.en.ts` (alongside the existing `save.*` block starting at line 259): `save.ai.generate` (button), `save.ai.generating`, `save.ai.ready`, `save.ai.draftLabel`, `save.ai.draftHint`, `save.ai.retry` — user-facing DE copy keeps the existing "Projekte" vocabulary (Principle IV)
- [x] T030 [P] [US1] Component test in `src/components/__tests__/save-changes-rows.test.ts` (`@vue/test-utils` + vitest, mock `src/lib/backend.ts`): clicking generate calls `generateCommitMessages` with the selected paths and renders one row per repo; pushing `started`/`done` events through the store makes that row editable with `draft.rendered`; editing a row and confirming commits exactly the edited text per repo and nothing during generation (covers US1-AC1..AC4 view side)

**Checkpoint**: MVP — US1 fully functional and independently testable (quickstart rows 3–5). Nothing else in the Save Changes flow changed.

---

## Phase 4: User Story 2 - Concurrent generation with live per-repo status (Priority: P2)

**Goal**: All selected repos generate simultaneously, each row shows its own state, and a row becomes editable the moment its draft lands without waiting for the slowest repo; commit stays blocked until every row has a message (FR-007, SC-001, US2-AC1..AC3).

**Independent Test**: quickstart row 3 with three repos where one is noticeably slower — two rows editable while the third still reads "generating"; commit button stays disabled until the last row has text.

### Implementation for User Story 2

- [ ] T031 [US2] Convert `commitgen::run_all` in `src-tauri/src/commitgen/mod.rs` to a concurrent fan-out per research R6: `const CONCURRENCY: usize = 8;` batch-scoped `Arc<Semaphore>` (constant, later env-tunable), one `tokio::spawn` task per repo that acquires an owned permit (`semaphore.acquire_owned()`) and **holds no lock across an `.await`** (Principle V); emit `started` for every path BEFORE awaiting the first task so all N rows show generating at once (US2-AC1); typical 1–20 repos finish in ≤3 waves
- [ ] T032 [US2] Enforce the per-row live-status contract in `src/stores/repoStore.ts`: apply each event to its own row the moment it arrives (no batch aggregation, no aggregate `done` — completion is derived from rows per contracts/commitgen-events.md §C3 rule 3), and confirm `canCommit` (T019) returns false while ANY row is `generating` or has an empty message (US2-AC3, FR-006)
- [ ] T033 [US2] Render per-row status badges in `src/components/SaveChangesModal.vue` rows (T027): `generating` (spinner + `t('save.ai.generating')`), `ready` (editable, `save.ai.ready`), `error` (reason text), with the textarea disabled only while `generating` so a finished row is immediately editable while siblings spin (US2-AC2)
- [ ] T034 [P] [US2] Concurrency test in `src-tauri/src/commitgen/mod.rs` `#[cfg(test)]` with a mock provider (sleep-per-repo, no live LLM): all N `started` events precede the first `done`, wall time ≈ slowest single repo and not the sum (SC-001), and >8 repos queue on the semaphore while ≤8 run
- [ ] T035 [P] [US2] Store test additions in `src/lib/__tests__/commitgen.test.ts`: interleaved events for different paths update independent rows; `canCommit` false while one row `generating`, true once every row has text

**Checkpoint**: US1 and US2 both work independently; batch use no longer feels broken.

---

## Phase 5: User Story 3 - Isolated failure and single-repo retry (Priority: P2)

**Goal**: A provider/model/empty-result failure touches only its own row, shows a human-readable reason with a retry affordance, retry regenerates that single repo, and manual typing into a failed row makes it committable (FR-008, FR-015, SC-004, US3-AC1..AC3).

**Independent Test**: quickstart row 6 — kill the model endpoint for one repo mid-run; siblings keep their drafts; retry regenerates only the failed row; fix config and retry succeeds.

### Implementation for User Story 3

- [ ] T036 [US3] Define the failure taxonomy in `src-tauri/src/commitgen/agent.rs` as `enum CommitGenError { MissingCredential { env_var }, Provider(String), Model(String), EmptyResult, MalformedDraft(String), NothingToSummarize, Conflicts, Io(String) }` with a `fn user_reason(&self) -> String` returning a stable machine-readable reason code plus the variable NAME for `MissingCredential` (never a value, FR-013) and **non-retryable-with-AI** flags for `NothingToSummarize`/`Conflicts` (contract C4); map serdesAI retry exhaustion (after exponential backoff, research R8) and empty/malformed drafts to `error` events — never commit silently (edge case "malformed provider output")
- [ ] T037 [US3] Implement single-repo retry in `src-tauri/src/commitgen/mod.rs`: `retry_one(path, &AiSettings, observer)` emitting a fresh `started`/terminal pair for that path under the SAME `run_id` (contracts §C3 rule 2), never emitting for a path whose attempt is still open, and exposing `pub async fn retry_repo(paths: &[String], path: &str, settings: &AiSettings, observer: &dyn CommitGenObserver)` used by the existing run entry; wire `service::generate_commit_messages` + the Tauri command and the web route to accept an optional single-path retry body `{ repoPaths: [one] }` reusing the SAME runId via a `runId` field on the request (contract C2/C3 otherwise unchanged)
- [ ] T038 [US3] Bind the retry affordance in `src/components/SaveChangesModal.vue` rows (T027/T033): retry icon on `error` rows calling `store.retryRepo(path)`; hide the icon for non-retryable reasons (`nothing to summarize`, `merge conflicts`) and show the next-action hint instead (Principle IV); confirm a manual `message` typed into an `error` row clears `canCommit`'s block (US3-AC3, already reducer-supported in T019)
- [ ] T039 [US3] Add DE + EN keys to `src/lib/i18n/messages.de.ts` / `messages.en.ts`: `save.ai.missingKey` (with `{envVar}` interpolation), `save.ai.badConfig`, `save.ai.notConfigured`, and `save.ai.errors.*` per reason (`nothingToSummarize`, `conflicts`, `provider`, `empty`, `malformed`) — every error string rendered as the next action the user can take (Principle IV, contracts/commitgen-events.md §C4)
- [ ] T040 [P] [US3] Store tests in `src/lib/__tests__/commitgen.test.ts`: an `error` event for one path leaves sibling rows' drafts and edits untouched (SC-004); `started` for a retry resets only that path and clears its `dirty` flag (FR-015); events for an unknown `runId` stay ignored
- [ ] T041 [P] [US3] Rust tests in `src-tauri/src/commitgen/mod.rs` `#[cfg(test)]` with a mock provider that fails only one path: batch completes, siblings `done`, failing path `error` with a reason string, retry emits exactly one new `started`+terminal pair for that path and no event for any other path

**Checkpoint**: One bad repo never costs the batch; retry is local and honest (user-triggered, research R8).

---

## Phase 6: User Story 4 - Repository inspection for ambiguous changes (Priority: P3)

**Goal**: The generator may consult the repository — current file content, the file at HEAD, recent subjects, bounded text search — under a per-run lookup budget (default 12), always sandboxed to that repository (FR-010, FR-011, US4-AC1..AC3).

**Independent Test**: quickstart-style check on a one-line dependency bump: the draft names the actual dependency (information available only via inspection) and the recorded lookup count never exceeds the configured limit.

### Implementation for User Story 4

- [ ] T042 [US4] Create `src-tauri/src/commitgen/tools.rs` per research R3: `RepoCtx { path: PathBuf, git: GitRunner-handles }` as the agent's `Deps`, and four serdesAI `FunctionTool` closures — `read_file`, `file_at_head`, `recent_history`, `grep` — each reading ONLY through the `GitRunner` read-only helpers from T010 (Principle III) with line/size caps on output; the run's `max_tool_calls` comes from `AiSettings.max_tool_calls` (default 12) passed as the agent run budget, and on exhaustion the agent must answer best-effort (US4-AC2)
- [ ] T043 [US4] Enforce the sandbox in `src-tauri/src/commitgen/tools.rs`: every path argument is `std::fs::canonicalize`d and prefix-checked against `RepoCtx.path`; anything outside (including `..` escapes and symlink tricks) is refused with a short error surfaced to the model and NO content leaked (FR-011, US4-AC3)
- [ ] T044 [US4] Register the tools in `src-tauri/src/commitgen/agent.rs` (`Agent<RepoCtx, CommitDraft>` with the four tools + `max_tool_calls`), extend `SYSTEM_PROMPT` (update the golden file from T022) with the inspection guidance "inspect only when the change detail is insufficient" (research R8), and thread the budget through `run_all`/`retry_one` (T023/T037)
- [ ] T045 [P] [US4] Tests in `src-tauri/src/commitgen/tools.rs` `#[cfg(test)]` with a temp git repo and NO live model: each of the four tools returns correct content, an out-of-repo path (`../escape`, absolute `/etc/hosts`, symlink) is refused, and a budget-exhaustion mock (counting wrapper) proves the agent answers best-effort at exactly ≤ `max_tool_calls` lookups (default 12)

**Checkpoint**: Drafts get CLI-grade depth for version bumps/renames while cost and blast radius stay bounded and configurable.

---

## Phase 7: User Story 5 - Graceful degradation without configuration (Priority: P3)

**Goal**: Users with no AI settings (or a missing env credential) see the control disabled with a short hint, everything else behaves exactly as before, and misconfiguration is diagnosable without crashes (FR-012, FR-014, SC-005, US5-AC1..AC3).

**Independent Test**: quickstart rows 1–2 — delete the `ai` block, open the dialog (control disabled + hint, manual flow commits normally); restore config but unset the key (run accepted, every row errors naming the variable, typing still works).

### Implementation for User Story 5

- [ ] T046 [US5] Gate the generate control in `src/components/SaveChangesModal.vue` (T028 button): disabled + `title`/inline hint when `store.config.ai` is `null`/absent (`t('save.ai.notConfigured')`) versus a distinct hint when `AiSettings` exists but fails `validate()` (`t('save.ai.badConfig')`, data-model E1 rule 2); manual message typing, `canCommit`, and commit behavior stay byte-identical to today's flow for unconfigured users (SC-005)
- [ ] T047 [US5] Make the missing-credential path a per-row failure instead of a hard error: in `src-tauri/src/commitgen/mod.rs`/`service.rs`, when `api_key_env` is unset in the process environment at run start, ACCEPT the run (contract C4 row 2) and emit `error` for EVERY row with the reason naming the variable, so the frontend renders `save.ai.missingKey` with `{envVar}` and manual entry still works (US5-AC2)
- [ ] T048 [US5] On web mode document and verify that the credential source is the **repoman server process** environment (research R7): `POST /api/repositories/generate-commit-messages` must read env server-side, and no response/event/log line may contain the key value (SC-006); add a `tracing` field hygiene check that log records for commitgen exclude both the credential and full change-detail payloads (plan §Constraints)
- [ ] T049 [P] [US5] Component tests in `src/components/__tests__/save-changes-rows.test.ts`: with `config.ai` absent the button is disabled and shows the not-configured hint while the manual textarea path still commits; with an invalid `baseUrl` the bad-config hint appears; with `ai` present the button enables
- [ ] T050 [P] [US5] i18n test run: `pnpm vitest run src/lib/__tests__/i18n.test.ts` proves every new `save.ai.*` key exists in BOTH `messages.de.ts` and `messages.en.ts` (FR-014, Principle IV / ADR 261031-1)

**Checkpoint**: The app is fully usable with AI off; every failure mode tells the beginner what to do next in their own language.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [ ] T051 Run the full automated gate set (quickstart §Automated gates): `cargo test --all-features`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` in `src-tauri/`; `pnpm vitest run`, `pnpm typecheck`, `pnpm build` at repo root — all green
- [ ] T052 Add the one `#[ignore]`d live-provider integration test in `src-tauri/src/commitgen/agent.rs` `#[cfg(test)]` (mirrors sb practice, plan §Testing): real endpoint + model from an `AiSettings` built off `OPENAI_API_KEY`/base-url env, asserting a schema-valid draft for a temp repo with a known change; run manually via `cargo test --all-features -- --ignored`
- [ ] T053 [P] Verify the read-only guarantee end to end (Principle V, SC-002, US1-AC3): `git log`/`git status` of every selected repo is unchanged after generation only; run quickstart row 5 (close dialog mid/post-run → no commit, no drafts persisted anywhere, session-only rule)
- [ ] T054 [P] Manual pass through quickstart §Manual scenarios rows 1–10 (including web mode row 9 over SSE and the German/English switcher row 10) and record results; SC-003's 7-of-10 acceptance rate measured once on real repos with drafts left unedited
- [ ] T055 [P] Security/edge audit in `src-tauri/src/commitgen/`: confirm no secret reaches `AppConfig`/logs/UI (SC-006), 5 000-file vendor drop truncates instead of hanging (row 7), merge-conflict repo errors non-retryably (row 8), and de-selecting a repo mid-run drops its row and ignores its in-flight results (data-model E4 rule 3)
- [ ] T056 Update `README.md` (Save Changes dialog mention of "✨ Generate with AI" + how to add the `ai` block and set the env var) and add the `CHANGELOG.md` entry in the same change as required by the constitution
- [ ] T057 [P] Confirm the seam discipline: `grep -rn "use serdes_ai\|serdes_ai::" src-tauri/src` matches ONLY files under `src-tauri/src/commitgen/` (plan §Structure Decision); fix any drift

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: immediate; T002 (dependency) must land before any `commitgen` code compiles
- **Phase 2 (Foundational)**: after Phase 1 — BLOCKS all user stories (settings + draft schema + diff + event contract + transports + store machine must exist before any view binds or any agent runs)
- **US1 (P1)**: after Phase 2 — MVP; the agent/runtime/orchestration/command/route/rows work lives here
- **US2 (P2)**: after Phase 2 + US1's run entry (T023) exists (T031 converts it to concurrent); row-status UI (T033) extends T027
- **US3 (P2)**: after Phase 2 + US1; T036/T037 extend T021/T023; T038 extends T027/T033
- **US4 (P3)**: after US1 (agent exists at T021/T022); independent of US2/US3 UI work
- **US5 (P3)**: after Phase 2 (needs `config.ai` shape from T005) + US1 (button exists from T028); T047 refines T021's credential error path
- **Polish**: after all delivered stories

### Within Each User Story

- Rust: schema/types → collection/runtime → orchestration → thin transports → UI binding → tests bundled in the same files where the repo puts in-module `#[cfg(test)]` tests
- Tests for reducer/store behavior are adjacent to (T019) and extended by each story; component tests are adjacent to the modal change they cover
- Every story ends with `cargo test` + `pnpm vitest run` + `pnpm typecheck` green for its surface (constitution gate)

### Parallel Opportunities

- T003 ∥ T001/T002; T006 ∥ T004/T005; T009 ∥ T007/T008; T012 ∥ T010/T011; T017/T018 ∥ all Rust tasks (TS files only)
- After Phase 2: US2 (concurrency + status) ∥ US3 (failure taxonomy + retry) ∥ US4 (tools) ∥ US5 (config gating) by different developers — but note the shared-file orderings below
- Shared-file constraints (do NOT parallelize these pairs): `commitgen/agent.rs` T021 → T022 → T036 → T044; `commitgen/mod.rs` T023 → T031 → T037 → T047; `SaveChangesModal.vue` T027 → T028 → T033 → T038 → T046; i18n `messages.{de,en}.ts` edited once per story phase (order T029 → T039 → T049's keys are all in T039/T029 sets)

## Parallel Example: User Story 1

```bash
# after T021/T022 (agent + prompt) — these three touch disjoint files:
Task: "T023 [US1] commitgen::run_all orchestration in src-tauri/src/commitgen/mod.rs"
Task: "T025 [US1] tauri command in src-tauri/src/commands/git_commands.rs + main.rs invoke_handler"
Task: "T029 [US1] save.ai.* keys in src/lib/i18n/messages.de.ts / messages.en.ts"
# then sequentially on the frontend:
Task: "T027 [US1] per-row list in src/components/SaveChangesModal.vue"
Task: "T028 [US1] generate button + per-row commit wiring in src/components/SaveChangesModal.vue"
Task: "T030 [P] [US1] src/components/__tests__/save-changes-rows.test.ts"
```

## Implementation Strategy

### MVP First (User Story 1 only)

1. Phase 1 (T001–T003) → 2. Phase 2 (T004–T020) → 3. Phase 3 (T021–T030) → 4. **STOP & VALIDATE** via quickstart rows 3–5 + `cargo test` + `pnpm vitest run` → demo. One-click per-repo drafts, reviewable and editable, committing only on confirm.

### Incremental Delivery

1. US2 (T031–T035): batch feels fast and trustworthy → validate quickstart row 3 (slow repo).
2. US3 (T036–T041): failures isolate, retry works → validate row 6.
3. US4 (T042–T045): draft quality on ambiguous diffs, budget enforced → validate dependency-bump case.
4. US5 (T046–T050): unconfigured users unaffected → validate rows 1–2, 10.
5. Polish (T051–T057): gates, live test, docs, security audit.
Each step ships without breaking the previous because the wire contract (contracts/*) and reducer rules (data-model E4) are frozen in Phase 2.

### Parallel Team Strategy

With multiple developers: team completes Phase 1–2 together (contract freeze), then Developer A: US1 (must finish first for shared files), then B: US2 ∥ C: US3 ∥ D: US4 ∥ E: US5, respecting the shared-file orderings above.

---

## Notes

- Constraint literals repeated verbatim where they apply so nobody guesses: lookup budget default **12**, change-detail cap **40 000** chars, **≤ 200** lines per untracked file, last **10** commit subjects, **Semaphore(8)** concurrency, subject **≤ 72** chars / no trailing period, type enum **feat|fix|refactor|perf|docs|style|test|build|ci|chore|revert**, truncation marker **`[... truncated ...]`**, `api_key_env` default **OPENAI_API_KEY**, serdesAI pin **tag v0.3.0**, SSE/Tauri names **`commitgen:repo`** / **`commitgen-repo`**, HTTP **202**.
- `AppConfig` lives in `src-tauri/src/models.rs:107` even though plan.md's structure table says `config.rs`; persistence (atomic temp-file rename) stays untouched.
- Generation must never call a git write path, never stage, never commit — the commit step keeps reading per-row text only (Principle V, SC-002).
- Only `src-tauri/src/commitgen/` may depend on serdesAI; commands and web handlers remain one-line delegators (Principle II, ADR__261030-4).
- Drafts and row state are session-only: cleared when the Save Changes modal closes; nothing new persisted except the `ai` settings block (data-model).
- Commit after each task or logical group; stop at each checkpoint to validate the story independently.
