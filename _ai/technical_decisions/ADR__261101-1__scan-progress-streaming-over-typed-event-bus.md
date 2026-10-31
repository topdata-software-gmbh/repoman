---
title: "Scan Progress Streaming over the Typed Event Bus"
status: Accepted
date: 2026-11-01
deciders: [Marc]
tags: [events, scanner, tauri, ux, observability]
adrId: 261101-1
---

# Scan Progress Streaming over the Typed Event Bus

## Context

RepoScanner walks a directory tree synchronously and returns results only when the
full walk finishes, so every scan (onboarding and refresh) shows a static spinner
with no feedback about which directory is being processed or what has been found.
Large roots (e.g. /topdata) take up to a minute, which reads as a hang.
A progress + log view was designed (live "Scan activity" modal, clickable
"Suche läuft…" chip, condensed inline line during onboarding). The mechanism by
which the blocking walker reports progress to the frontend needs to be pinned
down, because it will be the template for all future long-running operations.

## Decision

Stream scan progress through the existing typed `AppEvent` bus (ADR__261030-4).

- `RepoScanner::scan_path` gains an event sink (callback / unbounded channel) and
  emits `started`, `found` (per `.git` hit, never dropped), `tick` (current
  directory, throttled to at most one per 400 ms), and `done` (count, elapsed,
  optional error).
- The async service layer forwards the channel into `AppEvent::ScanProgress`;
  the desktop build maps it to the Tauri event `scan:progress`, the web build to
  the existing single SSE endpoint with event name `scan-progress`.
- Inspection runs sequentially per repository and emits `inspected` (kind, git
  state) so log lines update in place.
- The frontend keeps the log session-only in the Pinia store (same retention rule
  as the Git step log); the modal caps rendering at the last 500 entries while
  counts stay exact.

## Consequences

Positive:

- One notification mechanism for both transports; no new endpoint, no polling.
- The scanner stays testable (sink injection) and decoupled from Tauri types.
- Reusable shape: future long-running operations add a variant, not a pipeline.

Negative:

- The blocking walk must be refactored from "collect and return" to "emit and
  collect"; existing tests need updating.
- The 400 ms tick throttle is an extra constant to tune if webviews lag.
- Store-side log state adds memory proportional to repos found (bounded by the
  render cap only for display, not for data).

## Alternatives Considered

- **Shared `Arc<Mutex<ScanState>>` polled from the frontend**: loses append-only
  ordering (requires diffing), adds latency and a second feedback mechanism next
  to the bus. Rejected.
- **Pass `tauri::AppHandle` into the scanner and emit directly**: couples the
  scanner to Tauri and bypasses the bus pattern established in ADR__261030-4.
  Rejected for consistency and testability.
- **Show progress only after the walk (per-repo inspection only)**: still leaves
  the most confusing phase (the walk of huge trees) silent. Rejected.

## Related Decisions

- ADR__261030-4__typed-event-bus-for-desktop-and-web.md (this extends that bus)
