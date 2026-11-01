# Phase 0 Research: AI-Generated Commit Messages

**Feature**: `specs/002-ai-commit-messages` | **Date**: 2026-11-01

No NEEDS CLARIFICATION markers remain from the plan; every entry below records a
decision made during design (validated against the serdesAI source at
`v0.3.0`, the `sb gsum` CLI precedent, and repoman's existing transports).

## R1. LLM framework: serdesAI as pinned git dependency

- **Decision**: Add `serdes-ai = { git = "https://github.com/janfeddersen-wq/serdesAI", tag = "v0.3.0" }` (facade crate re-exporting the workspace) behind the `commitgen` seam module. Commit `Cargo.lock` (binary crate).
- **Rationale**: Rust-native tokio agent runtime matching repoman's stack; native per-run tool-call budget (`max_tool_calls` in run context/usage limits) replaces `sb gsum`'s hand-rolled counter; serde-validated structured output gives a typed draft instead of free-text parsing; multi-provider with base-URL override covers OpenRouter/Ollama via one OpenAI-compatible client; retry/backoff crate ships in-workspace.
- **Alternatives considered**: (a) raw `async-openai` calls — no tool loop, no output validation, we'd rebuild gsum's machinery in unsafe-to-drift form; (b) `rig` — smaller ecosystem, no native call budget; (c) shelling out to `sb gsum` — couples a desktop app to a user's Python CLI install and loses per-repo streaming; (d) sidecar HTTP microservice — extra moving part for zero benefit at this scope. Risk note: crate is pre-1.0 with ongoing parity work → pinning + single-seam isolation (plan Structure Decision) is the mitigation.

## R2. Output shape: structured `CommitDraft`, rendered in Rust

- **Decision**: `Agent<RepoCtx, CommitDraft>` with `CommitDraft { type, scope: Option, subject, body: Option }` implementing serdesAI's `OutputSchema`; final text rendered by `draft.render()` as `type(scope): subject` + optional blank-line body.
- **Rationale**: FR-003 (Conventional Commit form) becomes a schema constraint the framework enforces + retries, not a prompt hope; scope/type normalization testable without a live model.
- **Alternatives considered**: free-text message with regex post-processing — silent format drift impossible to gate; JSON-in-prompt parsing — re-implements what `serdes-ai-output` already validates.

## R3. Repository inspection: four `FunctionTool`s with `RepoCtx` deps

- **Decision**: Tools mirror the proven `sb gsum` set — `read_file`, `file_at_head`, `recent_history`, `grep` — built as serdesAI `FunctionTool` closures capturing `Deps = RepoCtx { path, git }`. Every path arg is canonicalized and prefix-checked against `RepoCtx.path`; outputs are line/size capped. Budget: `AiSettings.max_tool_calls` (default 12) passed as the run's `max_tool_calls`; on exhaustion the agent must answer best-effort.
- **Rationale**: FR-010/011; the CLI precedent showed these four resolve most ambiguous diffs; sandboxing is the only isolation boundary in a desktop app.
- **Alternatives considered**: expose serdesAI's generic filesystem/bash toolset — far larger blast radius; MCP file server — indirection without sandboxing benefit; no tools at all — rejected by design session (quality cliff on version bumps/renames).

## R4. Diff collection: `git diff HEAD` + bounded untracked content

- **Decision**: Change detail = `GitRunner::run(diff, ["diff", "HEAD", "--no-color"])` + first N lines (≤ 200/file) of each untracked file from `status --porcelain --untracked-files=all`, then truncated to `diff_budget_chars` (default 40 000) with a `[... truncated ...]` marker; plus last 10 commit subjects (`log -n 10 --format=%s`) for style.
- **Rationale**: must summarize exactly what the existing commit step would include (Assumption in spec); tracked+untracked covers the CLI-verified scope; truncation keeps FR-009/edge-case "huge diffs never fail the run" true; `%s` subjects give cheap style alignment (FR-003).
- **Alternatives considered**: staged-only diff (wrong scope — repoman commits the full working tree); libgit2 (Principle III violation); word-diff with hunk headers (richer but ~2× tokens for marginal quality — revisit in a later feature, not needed for v1).

## R5. Run model: fire-and-forget command + typed-bus events

- **Decision**: One request per batch: Tauri `generate_commit_messages(repo_paths)` / web `POST /api/repositories/generate-commit-messages` returns `{runId}` immediately; per-repo lifecycle streams as `AppEvent::CommitGen` (Tauri `commitgen:repo`, SSE `commitgen-repo`) with phases `started | done | error`, exactly like feature 001's observer pattern (`CommitGenObserver` implemented by the `EventBus`).
- **Rationale**: Principle II (one core, two thin transports, ADR__261030-4); a request/response would block axum on minutes-long agent runs and force timeout tuning; per-row streaming satisfies FR-007/US-2 directly.
- **Alternatives considered**: one invoke per repo (frontend loop) — rejected in the design session; N parallel HTTP requests — auth/rate noise, worse on desktop; polling status endpoint — chattier and loses immediacy.

## R6. Concurrency: tokio tasks under a `Semaphore(8)`

- **Decision**: `service::generate_commit_messages` spawns one tokio task per repo; tasks acquire a batch-scoped semaphore of 8 (`CONCURRENCY` constant, env-tunable later); never holds any lock across `.await`.
- **Rationale**: FR-007 concurrency with SC-001 wall-time; the repo already bounds group actions with semaphores (`service.rs` precedent); 8 keeps provider rate limits sane while covering the typical 1–20 batch in ≤3 waves.
- **Alternatives considered**: unbounded spawn — provider 429 storms on big batches; sequential — violates US-2; per-provider token buckets — overengineered for v1.

## R7. Configuration & credential handling

- **Decision**: Optional `ai` block in `AppConfig` (`config.rs`, same atomic writer): `{ provider baseUrl, model, apiKeyEnv: "OPENAI_API_KEY" default, maxToolCalls: 12, diffBudgetChars: 40000 }`. At run start the named env var is read on the server process; absent → per-repo `error` phase with reason naming the variable. v1: no settings UI; file-edit only (spec out-of-scope).
- **Rationale**: Principle Security (no plaintext secrets ever); "disabled-but-visible button when unconfigured" (US-5) needs only `config.ai.is_none()`; Tauri env access differs from a browser's — on web the repoman **server's** environment is the credential source, which the spec's FR-013 wording already matches ("read at request time" by the app that commits).
- **Alternatives considered**: store key in AppConfig (rejected — Security section); OS keyring/stronghold plugin (rejected for v1 scope; a later MINOR can add a settings UI + keyring without contract changes); reuse sb's `model-assignments.toml` (rejected in design session — silent cross-tool coupling).

## R8. Prompt & failure semantics

- **Decision**: Single system prompt (repo name, style subjects, tool guidance, Conventional Commit rules, "inspect only when the change detail is insufficient") stored as one Rust const, golden-file tested. Provider/model errors are retried by serdes-ai-retries with exponential backoff; after exhaustion (or empty/malformed draft) the row enters `error` with a human-readable reason + retry affordance; retry re-runs only that repo under the same `runId`.
- **Rationale**: FR-008, US-3, edge case "malformed provider output"; golden test makes prompt wording visible in review diffs.
- **Alternatives considered**: model fallback chains (spec out-of-scope; the settings shape already leaves a seam); auto-retry in the app (hides provider pain — user-triggered retry is the honest affordance).

**Status**: complete — no unresolved unknowns; plan Constitution Check stands.
