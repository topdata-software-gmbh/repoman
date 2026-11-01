# Implementation Plan: AI-Generated Commit Messages

**Branch**: `002-ai-commit-messages` | **Date**: 2026-11-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-ai-commit-messages/spec.md`

## Summary

The Save Changes dialog commits several repositories with one hand-typed shared
message. This feature adds a "Generate with AI" control that produces one
Conventional-Commit draft **per selected repository** from its own pending
changes, previewed in an editable per-repo list; the existing commit flow then
uses each row's final text. Generation never commits anything by itself.

Technical approach: a new `commitgen` module in the Rust core builds a serdesAI
agent per repository (`Agent<RepoCtx, CommitDraft>` with structured output and
four repo-inspection tools under a native `max_tool_calls` budget, mirroring the
proven `sb gsum` semantics). Diff input is collected through `GitRunner` only.
One new Tauri command + one new axum POST route start a run and return a `runId`
immediately; per-repo status (generating/ready/failed) streams over the existing
typed `AppEvent` bus to both transports (ADR__261030-4 pattern of feature 001).
The model/provider come from a new optional `ai` block in `AppConfig`; the
credential is referenced by environment-variable name, never stored.

## Technical Context

**Language/Version**: Rust 2021, MSRV 1.78 (src-tauri, pinned `rust-toolchain`) · TypeScript + Vue 3 (`<script setup>`, Pinia) frontend

**Primary Dependencies**: NEW: `serdes-ai` facade crate, git-pinned at tag `v0.3.0` (agent runtime, OpenAI-compatible provider with base-URL override, tool loop with call budget, serde-validated structured output, retry/backoff) · existing: `tokio`, `serde`, `serde_json`, `ignore`, `uuid` · frontend unchanged (`@tauri-apps/api`, ui-kit WebVue3, `lucide-vue-next`)

**Storage**: None new — drafts and row states are session-only in the Pinia store (same retention rule as the Git step log, ADR__261030-4); the `ai` settings block persists in the existing `AppConfig` JSON via the existing atomic temp-file-rename writer

**Testing**: `cargo test` (src-tauri; unit tests in-module, sandbox/truncation/rendering covered without a live LLM via a mock model) · `vitest` + `@vue/test-utils` (repoStore commitgen slice, SaveChangesModal rows) · one `#[ignore]`d live-provider integration test mirroring sb practice

**Target Platform**: Tauri v2 desktop window and browser via `repoman --web` (axum + SSE) — one frontend build, two transports

**Project Type**: Desktop app with embedded web server (src-tauri + src, single workspace)

**Performance Goals**: batch of N repos finishes in ≈ the slowest wave (semaphore of 8 concurrent agents, SC-001); per-repo request payload bounded at 40 000 chars of change detail (configurable); typical draft within 60 s

**Constraints**: generation MUST NOT write repository state (Principle V); all git reads via `GitRunner` system CLI, no git library (Principle III); tools sandboxed to the repo path with canonicalize+prefix check; credential read from env at request time, never in config/logs/UI (Principle IV/Security); every new string in DE+EN dictionaries; new core logic lives in `service`/`commitgen`, commands and web handlers stay thin transports (Principle II)

**Scale/Scope**: typical batches of 1–20 dirty repositories; 5 new Rust files in one new module, 2 new bus events + 1 command + 1 route, 1 reworked modal section + 1 store slice

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Against `.specify/memory/constitution.md` v1.0.0:

- **I. Spec-Driven Change**: **PASS** — implementation is gated on this accepted spec chain (`spec.md` → plan → tasks); no code predates it.
- **II. One Core, Two Frontends**: **PASS** — `commitgen` + `service::generate_commit_messages` hold all logic; the Tauri command and `POST /api/repositories/generate-commit-messages` are one-line delegators; results travel only as a typed `AppEvent::CommitGen` variant over `emit`/`/api/events` (ADR__261030-4).
- **III. Git Only Through the System CLI**: **PASS** — diff, untracked listing, HEAD file access, and history are collected via `GitRunner` (`git diff HEAD`, `status --porcelain`, `show HEAD:<file>`, `log --format=%s`); serdesAI is an LLM framework and touches no repository.
- **IV. Beginner-Friendly by Default**: **PASS** — every label/hint/error copy goes through `src/lib/i18n` DE+EN; failure rows show next-action text (e.g. missing-credential hint names the variable to set).
- **V. Bounded, Non-Destructive Action**: **PASS** — run is read-only by construction (no git write APIs called); tool budget, diff-size budget, and a concurrency semaphore are all explicit and configurable; config persistence keeps the atomic-write path.
- **Security & Configuration Constraints**: **PASS** — `api_key_env` stores only the variable name; no secret can enter `AppConfig`, logs (tracing fields exclude key and full payloads), or UI.
- **Dependency justification** (conventions `rust:dependencies`): "justify every new crate" — satisfied in research.md R1 with rejected alternatives.

**Gate result: PASS** (pre-research).

## Project Structure

### Documentation (this feature)

```text
specs/002-ai-commit-messages/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   ├── generate-commit-messages-command.md
│   └── commitgen-events.md
└── tasks.md             # Phase 2 output (/speckit.tasks — NOT this command)
```

### Source Code (repository root)

```text
src-tauri/src/
├── commitgen/                   # NEW: the single AI seam module (swap-risk isolated here)
│   ├── mod.rs                   # run_all(paths, &AiSettings, observer) orchestration
│   ├── settings.rs              # AiSettings (provider base_url, model, api_key_env,
│   │                            #   max_tool_calls=12, diff_budget_chars=40000)
│   ├── draft.rs                 # CommitDraft {type,scope,subject,body} + OutputSchema + render()
│   ├── diff.rs                  # change-detail collection via GitRunner + truncation
│   ├── agent.rs                 # Agent<RepoCtx, CommitDraft> wiring, prompt, retries
│   └── tools.rs                 # read_file / file_at_head / recent_history / grep
│                                #   (FunctionTool closures, sandboxed to RepoCtx.path)
├── service.rs                   # MODIFY: generate_commit_messages entry (semaphore=8,
│                                #   tokio::spawn per repo) + CommitGenObserver
├── events.rs                    # MODIFY: AppEvent::CommitGen(CommitGenEvent)
├── watcher.rs                   # MODIFY: EventBus implements CommitGenObserver
├── commands/git_commands.rs     # MODIFY: #[tauri::command] generate_commit_messages
├── config.rs                    # MODIFY: AppConfig.ai: Option<AiSettings>
├── git_runner.rs                # MODIFY: helpers for diff HEAD / show HEAD:<file> / log %s
├── main.rs                      # MODIFY: forward CommitGen → emit("commitgen:repo")
└── web.rs                       # MODIFY: POST /api/repositories/generate-commit-messages
                                 #   + SSE mapping "commitgen-repo"

src/
├── lib/backend.ts               # MODIFY: onCommitGenRepo in subscribeToEvents;
│                                #   generateCommitMessages(paths) invoke/POST
├── types/domain.ts              # MODIFY: CommitDraft, CommitGenEvent, RowState types
├── stores/repoStore.ts          # MODIFY: commitgen slice — rows Map<path, RowState>,
│                                #   startRun/handleEvent/retryRepo, clears on modal close
├── components/
│   └── SaveChangesModal.vue     # MODIFY: shared textarea → per-repo row list
│                                #   (status badge, editable draft, retry icon);
│                                #   "Generate with AI" button, disabled when !config.ai
└── lib/i18n/messages.{de,en}.ts # MODIFY: save.ai.* keys (button, states, hints, errors)
```

**Structure Decision**: Stay in the existing single-workspace layout. All
LLM-framework surface is isolated in `src-tauri/src/commitgen/` — the only module
that may `use serdes_ai` — so the git-pinned young dependency (v0.3.0, parity in
progress) can be swapped without touching services, transports, or UI.

## Post-Design Constitution Re-check

After Phase 1 (research, data model, contracts): **PASS** — no contract exposes
credentials (only `api_key_env` names and error strings), events reuse the typed
bus, drafts are session-only, and the commit step is unchanged except that it
reads per-row text. No violations → Complexity Tracking unused.

## Complexity Tracking

> No constitution violations — table unused.
