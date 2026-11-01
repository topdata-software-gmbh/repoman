---
filename: "/topdata/repoman/_ai/backlog/reports/261101_0355__IMPLEMENTATION_REPORT__ai-commit-messages.md"
title: "Report: AI commit messages (spec 002)"
createdAt: 2026-11-01 03:55
updatedAt: 2026-11-01 03:55
repoRoot: "/topdata/repoman"
project: "repoman"
status: complete
filesCreated: 2
filesModified: 15
filesDeleted: 0
tags: [ai, llm, commit-messages, serdesai, tauri, vue, security]
documentType: IMPLEMENTATION_REPORT
---

# Report: AI commit messages (spec 002)

## Summary

Implemented `specs/002-ai-commit-messages` end to end. The Save dialog now offers
**✨ Suggest with AI**: one Conventional Commit draft per selected project,
generated from that project's own pending changes, streamed over the typed event
bus, editable per row, and committed per row. Generation is read-only; the commit
step is unchanged in behaviour apart from taking each row's text. All 57 planned
tasks are complete except the manual GUI scenario pass (T054).

## Prompt used

`/implement-plan` on `specs/002-ai-commit-messages/tasks.md` ("Execute an
implementation plan across all phases without asking for confirmation"), continued
across sessions. No mid-plan questions were asked.

## What was built

### Backend (`src-tauri/src/commitgen/`, the only `serdes_ai` consumer)

- `settings.rs`: `AiSettings` (`baseUrl`, `model`, `apiKeyEnv`, `maxToolCalls` 12,
  `diffBudgetChars` 40000), validation, additive `AppConfig.ai`.
- `draft.rs`: `CommitDraft` + wire form (`rendered`), schema validation
  (11 types, ≤72 chars, no trailing period, lowercase-hyphen scope), rendering.
- `diff.rs`: bounded change detail (`git diff HEAD`, untracked content capped at
  200 lines/file, last 10 subjects, 40 000-char budget with truncation marker),
  conflict/nothing-to-save/unreadable errors.
- `agent.rs`: OpenAI-compatible client from settings, pinned system prompt with a
  golden file, structured output + validator, retryable-error backoff, request cap
  (no hang on unparseable answers), typed failure taxonomy with stable reason codes
  that never contain the key, single `#[ignore]`d live-provider test.
- `tools.rs`: four read-only tools (`read_file`, `file_at_head`, `recent_history`,
  `grep`), canonicalized sandbox to the repository root (refuses `..`, absolute
  paths, symlink escapes), per-attempt lookup budget.
- `mod.rs`: immediate `runId` + spawned batch, `Semaphore(8)` fan-out, `started`
  for every row before the first terminal event, per-path isolation, same-`runId`
  single-repo retry, event types.

### Service, transports, security

- `service::generate_commit_messages` validates empty/duplicate/unknown paths and
  `ai-not-configured` before spawning, then delegates (Principle II).
- Tauri command `generate_commit_messages` → `{ runId }`; web
  `POST /api/repositories/generate-commit-messages` → **202** `{ runId }`, behind
  the existing token middleware. Retry requests carry `runId`.
- Missing credential is accepted per contract: every row gets an `error` naming
  the environment variable; no value is persisted, logged, or sent to the browser.

### Frontend

- `types/domain.ts` mirrors, `backend.generateCommitMessages` + `commitgen:repo` /
  SSE `commitgen-repo` subscription with malformed-frame guards.
- Pinia `commitgen` slice: run id, per-path rows, dirty-wins-over-late-draft,
  retry only its row, event buffering for events that beat the ACK, `canCommit`,
  session-only reset on close, dropped rows for de-selected projects.
- `SaveChangesModal.vue`: per-row badges/spinner/textarea, generate button gated
  by AI state (missing vs invalid hints), localized next-action errors, retry only
  for retryable reasons, per-row commit, manual path unchanged.
- DE/EN keys for all new strings.

## Deviations from the plan

1. **serdesAI tag**: the plan pinned `v0.3.0`; that tag does not exist. Pinned
   `v0.2.6` instead and raised `rust-version` to 1.80 (`LazyLock` in a transitive
   crate; the plan assumed a `rust-toolchain` file that does not exist).
2. **Deps type**: the agent runs with `Deps = RepoCtx` and builds fresh
   dependencies (and a fresh lookup budget) per attempt, because retries need a
   value per `run()` call.
3. **Tool budget**: the framework's `UsageLimits` hard-aborts at the limit instead
   of answering best-effort, so the project enforces `maxToolCalls` itself and
   leaves the framework limit as headroom (`max + 4`).
4. **`RepoCtx` location**: lives in `agent.rs` (not `tools.rs`) because the agent
   and the tools share it; `tools.rs` imports it.
5. **Retry `runId`**: implemented by passing `runId` on the start/retry request so
   retry events reuse the active run, as the contract requires.
6. **`run_all` signature**: returns the `runId` synchronously and spawns the batch
   (the contract requires an immediate 202/ACK while events stream).

## Validation results

Automated gates (all green):

- `cargo test --all-features`: **76 passed, 0 failed, 1 ignored** (live provider).
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- `pnpm vitest run`: **101 passed** across 13 files.
- `pnpm typecheck`: clean.
- `pnpm build`: success.
- Seam check: `serdes_ai` appears only under `src-tauri/src/commitgen/`.

Coverage highlights: prompt golden file, validation/normalization, truncation and
conflict handling, concurrency ordering and wall time, >8 queueing, per-path
isolation, same-`runId` retry, missing-credential per row, sandbox refusals,
budget exhaustion, read-only guarantee (`git status` unchanged), store state
machine and component flows, DE/EN key parity.

Not run here: **T054** (manual quickstart rows in the GUI, including web SSE and
the language switcher). Recommended before release; see Next steps.

## Security notes

- The key is read from the server process environment at run start and only passed
  to the model client. Config stores the variable **name** only. Events carry
  `phase`/`runId`/`path`/draft/reason; the debug log line logs only phase, path and
  run id.
- Tool paths are canonicalized and prefix-checked; symlink and `..` escapes are
  refused without leaking content.
