# Tasks: AI-Generated Commit Messages

**Input**: Design documents from `/specs/002-ai-commit-messages/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: INCLUDED — constitution ("Development Workflow & Quality Gates": new behavior ships with tests covering error paths and edge cases) and the plan's Testing section make them mandatory; the plan further fixes the framework split (`cargo test` in-module/unit + one `#[ignore]`d live test; `vitest` for store and modal).

**Organization**: Tasks grouped per user story; every story is independently implementable and testable on top of Setup + Foundational.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: different files, no dependency on an incomplete task
- **[Story]**: US1…US5 from spec.md
- Shared-file tasks (agent.rs, service.rs, SaveChangesModal.vue, messages.{de,en}.ts, repoStore slice) are deliberately NOT [P] within/between stories touching the same file.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: make the dependency and the governance record exist before code

- [ ] T001 Add `serdes-ai = { git = "https://github.com/janfeddersen-wq/serdesAI", tag = "v0.3.0" }` (facade crate) to `src-tauri/Cargo.toml`, verify `cargo check` compiles the workspace, and commit `Cargo.lock` (binary crate rule, conventions `rust:dependencies`; research R1)
- [ ] T002 [P] Record adoption ADR in `_ai/technical_decisions/261101-2__ADR__serdesai-pinned-git-dependency-commitgen-seam.md` — pin policy + "only `src-tauri/src/commitgen/` may `use serdes_ai`" seam rule, per constitution Principle ("Decisions that constrain work beyond a single feature MUST be recorded as an ADR before implementation starts")

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: settings, draft schema, diff input, event plumbing, command skeleton

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [ ] T003 Define `AiSettings` in `src-tauri/src/commitgen/settings.rs` — fields `baseUrl`, `model`, `apiKeyEnv` (default `"OPENAI_API_KEY"`; `""` ⇒ keyless provider, env lookup skipped — analyze A1), `maxToolCalls` (default 12), `diffBudgetChars` (default 40000), serde camelCase; validation verbatim from data-model E1: "baseUrl MUST parse as `https?://…`; invalid stored value → treated like unconfigured"; `#[cfg(test)]` unit tests for defaults + baseUrl rejection + keyless empty-string acceptance
- [ ] T004 Wire `AppConfig.ai: Option<AiSettings>` in `src-tauri/src/config.rs` using the existing atomic temp-file-rename writer unchanged (absent → feature unavailable, data-model E1 rule 1)
- [ ] T005 [P] Implement `CommitDraft` in `src-tauri/src/commitgen/draft.rs` — verbatim constraints: type enum `feat|fix|refactor|perf|docs|style|test|build|ci|chore|revert`; "subject: non-empty, ≤ 72 chars after trim, no trailing period"; scope "lowercase-hyphen"; body optional; serdesAI `OutputSchema` impl; `render()` = `type(scope): subject` + blank-line body; wire form `{ type, scope, subject, body, rendered }` camelCase; unit tests for render + every subject/type validation rule (covers FR-003 error paths)
- [ ] T006 [P] Add git read helpers to `src-tauri/src/git_runner.rs` exclusively via system CLI (Principle III): `diff` `["diff","HEAD","--no-color"]`, `show` `["show","HEAD:<file>"]`, `log` `["log","-n","10","--format=%s"]`, `status --porcelain=v1 --untracked-files=all`; unit tests with a temp git repo incl. conflict-detection via porcelain codes
- [ ] T007 Implement change-detail collection in `src-tauri/src/commitgen/diff.rs` — per research R4: diff HEAD + first ≤200 lines of each untracked file, "then truncated to `diff_budget_chars` (default 40 000) with a `[... truncated ...]` marker", plus last 10 commit subjects for style; empty change set → "nothing to summarize", conflict state → "merge conflicts — resolve first" (contract C4); unit tests for truncation, untracked cap, empty, conflict
- [ ] T008 Add `AppEvent::CommitGen(CommitGenEvent)` in `src-tauri/src/events.rs` (payload: `runId`, `path`, `phase: started|done|error`, `draft?`, `error?` — camelCase serde); implement observer sink on `EventBus` in `src-tauri/src/watcher.rs`; forward in `src-tauri/src/main.rs` as Tauri event `commitgen:repo`; map to SSE `commitgen-repo` on the existing `GET /api/events` in `src-tauri/src/web.rs`; enforce contract C3 ordering: "Per (runId, path): exactly one started precedes any terminal event; exactly one terminal (done or error) closes the attempt"
- [ ] T009 Add command skeleton: `generate_commit_messages(repo_paths)` in `src-tauri/src/commands/git_commands.rs` — thin delegator per Principle II: validates nothing itself, forwards to `service::generate_commit_messages` (T014) and returns `{runId}` (uuid v4) immediately; mirror route `POST /api/repositories/generate-commit-messages` (202) behind `require_token` in `src-tauri/src/web.rs` (same delegation, no route-layer logic); frontend `generateCommitMessages(paths): Promise<string>` in `src/lib/backend.ts` + `CommitDraft`/`CommitGenEvent` types in `src/types/domain.ts` + `onCommitGenRepo` in `subscribeToEvents` (listen + EventSource)

**Checkpoint**: Foundation ready — user stories can now be built (and by different people, in priority order)

## Phase 3: User Story 1 - Generate per-repo messages, review, commit (Priority: P1) 🎯 MVP

**Goal**: one click → per-repo Conventional draft in editable rows → commit uses each row's final text; generation never commits

**Independent Test**: spec US1: "Configure a model, select two repositories with different pending changes, generate, and verify two distinct draft messages appear; edit one and commit; confirm each repository receives exactly its (possibly edited) message."

### Tests for User Story 1 ⚠️ (write first, watch them fail)

- [ ] T010 [P] [US1] Unit tests in `src-tauri/src/commitgen/agent.rs` `#[cfg(test)]` module with serdesAI's mock/test model: draft lands as `phase=done` with `render()`-shape text; malformed/blank provider output → `phase=error` with reason, never silently committed (spec Edge "malformed/blank")
- [ ] T011 [P] [US1] Vitest `src/stores/__tests__/repoStore.commitgen.test.ts` — verbatim E4 rules: rows Map<path,status,message,dirty,error>; "Events with unknown runId are ignored"; "message… becomes draft.rendered on done, user edits win — any late event for a row the user has typed into is ignored"; slice cleared on modal close
- [ ] T012 [P] [US1] Vitest `src/components/__tests__/SaveChangesModal.test.ts` — per-row textarea seeded with `rendered`; edit survives; FR-006 verbatim: "Commit confirmation MUST require a non-empty message for every selected repository… regardless of whether the text was AI-generated or typed"; commit payload uses each row's text

### Implementation for User Story 1

- [ ] T013 [US1] Implement agent runner in `src-tauri/src/commitgen/agent.rs`: `Agent<RepoCtx, CommitDraft>` with system prompt const (repo name, recent subjects, Conventional rules — research R8 wording) and NO tools registered in MVP (`max_tool_calls: 0`); provider built from settings (`baseUrl`, key from env named by `apiKeyEnv`) via serdesAI OpenAI-compatible provider with base-URL override; serdes-ai-retries with exponential backoff for transient errors; `run_one(path, &settings_snapshot, &observer)` emits started → done/error per T008 contract
- [ ] T014 [US1] Implement `service::generate_commit_messages` in `src-tauri/src/service.rs`: validate paths here — verbatim contract C2 "Every path MUST be a discovered repository…; unknown or duplicate paths are rejected before any task spawns. Empty list → Err(\"no repositories selected\")"; snapshot `AiSettings`; credential rule (contract C4 + A1): non-empty `apiKeyEnv` unset in env → accept run but emit `phase=error` "for **every** row… with reason naming the variable", `apiKeyEnv: ""` → keyless, skip lookup; spawn per-repo tokio tasks (sequential join for MVP — concurrency in US2) delegating to `run_one`
- [ ] T015 [US1] Add `commitgen` slice to `src/stores/repoStore.ts`: `rows: Map<path, RepoDraftRow>`, start-run on invoke, apply T008/T009 events per E4 state machine, expose per-row `message` to the modal, and adapt `commitRepositories` call sites so each repo receives **its row's** final text (was shared message)
- [ ] T016 [US1] Rework `src/components/SaveChangesModal.vue`: replace the single shared message textarea with the per-repo row list (repo name + textarea bound to `row.message`); validation hint per row when empty; keep everything else (file preview, saving flow) unchanged
- [ ] T017 [US1] Add "Generate with AI" button to `src/components/SaveChangesModal.vue` (enabled when `config.ai` present and ≥1 selected dirty repo; calls T009 helper; whole-batch spinner per row state) and i18n keys `save.ai.generate`, `save.ai.rowStatus.*`, `save.ai.errors.*` in `src/lib/i18n/messages.de.ts` and `messages.en.ts` (FR-014: DE and EN in the same task)
- [ ] T018 [P] [US1] Live-provider integration test `src-tauri/tests/commitgen_live.rs` (`#[ignore]`, run via `cargo test -- --ignored`): one real dirty temp repo → asserts draft type/subject rules hold; mirrors sb gsum's live-test practice

**Checkpoint**: MVP — US1 fully functional; quickstart rows 3–4 pass (config a local Ollama, generate 2 repos, edit one, commit)

## Phase 4: User Story 2 - Concurrent generation with live status (Priority: P2)

**Goal**: all repos in flight at once; each row editable the moment its draft lands

**Independent Test**: spec US2: "Select three repositories where one reliably takes noticeably longer; verify the other two rows become editable while the third still shows 'generating'."

- [ ] T019 [US2] Replace sequential join in `src-tauri/src/service.rs` with true concurrency under batch-scoped `Semaphore(8)`: "spawns one tokio task per repo… never holds any lock across `.await`" (research R6, Principle V); SC-001 wall-time behavior noted in code comment
- [ ] T020 [US2] Extend `src/stores/__tests__/repoStore.commitgen.test.ts`: interleaved events across paths (store keys "solely on path"; ordering between paths unspecified — contract C3.4); row becomes `ready`+editable while siblings `generating`; gating: "any row is still generating… then committing is blocked" (US2-AC3)
- [ ] T021 [US2] Row status badge UI in `src/components/SaveChangesModal.vue` (spinner `generating` / check `ready` / ✕ `error`) + disabled commit button while any row is generating

**Checkpoint**: US1 + US2 both work; quickstart row 3 concurrency verified

## Phase 5: User Story 3 - Isolated failure and single-repo retry (Priority: P2)

**Goal**: one repo's failure never spoils the batch; per-row retry

**Independent Test**: spec US3: "Generate for three repositories with an invalid model name configured; all three show failed; fix configuration; retry one row and confirm only that row is regenerated."

- [ ] T022 [US3] Backend retry path in `src-tauri/src/service.rs` + `src-tauri/src/commands/git_commands.rs` (file-safe order after US2's T019 touches the same file): retry re-runs **only that repo** under the same `runId` — emits "a fresh started + terminal pair for the same path under the same runId" (contract C3.2); no aggregate done event exists — store derives completion from rows (C3.3)
- [ ] T023 [US3] Retry affordance in `src/components/SaveChangesModal.vue`: error reason text + per-row retry icon → calls retry for that path only; typed-message-into-error-row makes the row commit-eligible (US3-AC3)
- [ ] T024 [US3] Extend `src/stores/__tests__/repoStore.commitgen.test.ts`: siblings' drafts and edits untouched by one row's retry; retry's `started` clears only that row's dirty flag (E4 rule: "only cleared by a fresh started for that path")

**Checkpoint**: quickstart rows 5–6 pass (kill endpoint mid-run; other row survives; retry isolated)

## Phase 6: User Story 4 - Repository inspection tools (Priority: P3)

**Goal**: ambiguous diffs resolved via bounded repo lookups, sandboxed

**Independent Test**: spec US4: dependency-bump repo whose message references the actual dependency via inspection; lookup count never exceeds the configured budget.

### Tests for User Story 4 ⚠️

- [ ] T025 [P] [US4] Sandbox unit tests in `src-tauri/src/commitgen/tools.rs` `#[cfg(test)]`: verbatim FR-011 — "paths outside it MUST be refused" — test `../../etc/passwd`, absolute outside paths, symlink escape via canonicalize + prefix check
- [ ] T026 [P] [US4] Golden-file prompt test `src-tauri/tests/commitgen_prompt_golden.rs`: system-prompt const (incl. tool guidance sentence) snapshot so wording changes show in review diffs (research R8)

### Implementation for User Story 4

- [ ] T027 [US4] Implement the four `FunctionTool` tools in `src-tauri/src/commitgen/tools.rs` capturing `RepoCtx { path, git }` as serdesAI Deps: `read_file`, `file_at_head` (`show HEAD:<file>`), `recent_history`, `grep` — each "output is line/size capped" (research R3); register in `src-tauri/src/commitgen/agent.rs` and switch MVP `max_tool_calls: 0` → `settings.maxToolCalls` (default 12); on budget exhaustion the run "returns its best-effort draft" (US4-AC2)

**Checkpoint**: quickstart-style check — run on ambiguous repo with `--debug` equivalent log; budget respected

## Phase 7: User Story 5 - Graceful degradation without configuration (Priority: P3)

**Goal**: no/invalid config → disabled control + hint; manual flow untouched

**Independent Test**: spec US5: "Remove the AI section from the configuration; open the dialog; verify the control is disabled with a hint and the manual flow commits normally."

- [ ] T028 [US5] Disabled-state UI in `src/components/SaveChangesModal.vue` + `src/stores/repoStore.ts`: absent `config.ai` → hint `save.ai.notConfigured`; invalid `baseUrl` → `save.ai.badConfig` (data-model E1 rule 2); credential missing at click → per-row reason naming the variable via `save.ai.missingKey` with `{envVar}` interpolation (US5-AC2); add both languages to `src/lib/i18n/messages.{de,en}.ts`
- [ ] T029 [US5] Extend `src/components/__tests__/SaveChangesModal.test.ts`: disabled button + tooltip variants; manual message flow regression unchanged (US5-AC1/AC3, SC-005)

**Checkpoint**: quickstart rows 1–2 pass

## Phase 8: Polish & Cross-Cutting

- [ ] T030 [P] i18n sweep: assert every `save.ai.*` key exists in BOTH `messages.de.ts` and `messages.en.ts`, no hard-coded literals in `SaveChangesModal.vue` (Principle IV)
- [ ] T031 [P] Update `CHANGELOG.md` in the same PR as user-visible behavior (constitution workflow rule)
- [ ] T032 Run all automated gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-features`, frontend typecheck, `pnpm vitest run` (constitution quality gates; quickstart "Automated gates")
- [ ] T033 Execute `specs/002-ai-commit-messages/quickstart.md` scenarios 1–11 incl. web mode (row 9), the SC-003 7-of-10 acceptance sample on real dirty repos, and the row-11 sentinel-credential redaction check (SC-006); record results in the feature dir

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (P1 phase)**: none; T001 blocks all Rust code touching serdesAI (T013+); T002 must land before implementation starts (constitution)
- **Foundational**: needs T001; **blocks every user story**
- **US1 → US2 → US3** sequential by file collision (`service.rs`, `SaveChangesModal.vue`, store slice), even though the stories are conceptually independent
- **US4** can start once Foundational is done but shares `agent.rs` with US1 (T013) — do after US1 merges
- **US5** only needs Foundational + T017 (button exists to be disabled)
- **Polish** after all desired stories

### Shared-file conflicts (why some tasks lack [P])

`service.rs` (T014/T019/T022) · `agent.rs` (T010/T013/T027) · `SaveChangesModal.vue` (T016/T017/T021/T023/T028/T029) · `repoStore.ts` + its test (T011/T015/T020/T024) · `messages.{de,en}.ts` (every UI task writes both languages inside the task itself to keep FR-014 atomic)

### Parallel Opportunities

- T002 ∥ T001; T005 ∥ T006; US1's three test tasks T010/T011/T012 ∥ each other; T018 ∥ everything; T025 ∥ T026; T030 ∥ T031

## Parallel Example: User Story 1

```bash
# After Foundational, write the failing tests together:
Task: "T010 [US1] agent.rs mock-model unit tests (done/error mapping)"
Task: "T011 [US1] repoStore.commitgen.test.ts (E4 state machine)"
Task: "T012 [US1] SaveChangesModal.test.ts (rows + FR-006 gating)"
# T018 live-provider test is independent of all of the above
```

## Implementation Strategy

### MVP First

1. Phase 1 Setup + Phase 2 Foundational
2. Phase 3 US1 (tools OFF, sequential) → **STOP**: quickstart rows 3–4 on two real repos
3. Ship/demo

### Incremental Delivery

US2 (concurrency badges) → US3 (retry) → US4 (tools + budget) → US5 (degradation) → Polish; each increment independently validated with its quickstart rows before merging.

## Notes

- Constraint quotes in task text are verbatim from data-model.md/contracts — implementation must not reinterpret them
- Commit after each task or story phase; stop at any checkpoint
- If serdesAI v0.3.0's mock surface proves untestable, T010/T013 may stub the model call behind a `commitgen`-local trait — the seam rule (T002 ADR) keeps that invisible elsewhere
