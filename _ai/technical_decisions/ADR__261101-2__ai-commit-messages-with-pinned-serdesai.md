---
title: "AI Commit Messages with a Pinned serdesAI and a Read-Only Tool Sandbox"
status: Accepted
date: 2026-11-01
deciders: [Marc]
tags: [ai, llm, commit-messages, serdesai, security, tauri]
adrId: 261101-2
---

# AI Commit Messages with a Pinned serdesAI and a Read-Only Tool Sandbox

## Context

The Save dialog only accepted one shared message for every selected project, which
produces vague history across multi-repo groups. The feature needs per-project
Conventional Commit drafts generated from each project's own pending changes,
reviewed and edited before committing.

Constraints that shaped the decision:

- The generator must never write to a repository; committing stays a separate,
  user-triggered action.
- Provider failures must be isolated per project, never abort a batch, and never
  leak the API key into config, events, logs, or the browser.
- The LLM framework is young; the rest of the codebase must not depend on it.
- Web server mode reads the credential from the server process environment.

## Decision

Use the `serdes-ai` Rust agent framework pinned to git tag `v0.2.6`
(`default-features = false`, `features = ["openai"]`), confined to
`src-tauri/src/commitgen/` behind a small service seam.

- `commitgen::run_all` mints a `runId`, emits `started` for every selected project,
  then fans out with a batch-scoped `Semaphore(8)`; `retry_repo` reuses the same
  `runId` for a single project.
- Structured output is a `CommitDraft` (`type/scope/subject/body`) validated by an
  `output_validator_fn`; the backend renders the final message so DE/EN and
  trimming never diverge.
- Provider failures are retried with exponential backoff for retryable errors;
  malformed answers are re-asked up to twice, and a request cap prevents a
  misbehaving provider from looping forever.
- Four read-only inspection tools (`read_file`, `file_at_head`, `recent_history`,
  `grep`) are registered with `tool_with_executor`. Every lookup is canonicalized
  and prefix-checked against the repository root, and claims one unit from a
  per-attempt budget (`maxToolCalls`, default 12). The framework's own usage limit
  is only a backstop because it aborts instead of allowing a best-effort answer.
- Events flow over the existing typed `AppEvent` bus (ADR__261030-4) as
  `commitgen:repo` / SSE `commitgen-repo`; the start command and web route answer
  immediately with `{ runId }` (HTTP 202 on web).
- The credential is read from the environment variable **named** by
  `ai.apiKeyEnv` at run start; the value is passed straight to the model and never
  stored or logged.

## Consequences

Positive:

- The UI and services never import the framework; swapping it only touches
  `commitgen/`.
- Per-project isolation, live status, retry, and manual fallback all follow from
  the event-per-path contract.
- The key never reaches the config file or the browser; web mode behaves the same
  as desktop.

Negative:

- The framework is pinned to a git tag and `rust-version` moved to 1.80 because of
  it; upgrades require re-verifying the API surface.
- The framework's tool-call limit is a hard abort, so the project enforces its own
  lookup budget and keeps the framework limit as headroom.
- Prompt wording is pinned by a golden file; any wording change shows up in review
  diffs.

## Alternatives

- A hand-rolled HTTP client for the OpenAI API: more code to own, no structured
  output validation or tool loop.
- Server-side templated commit messages (no model): does not summarize unfamiliar
  changes.
- Letting the model call git directly: violates the read-only guarantee.

## Related

- ADR__261030-4 (typed event bus for desktop and web)
- `specs/002-ai-commit-messages/{spec,plan,research,data-model}.md`
