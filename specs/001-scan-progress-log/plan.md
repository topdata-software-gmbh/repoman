# Implementation Plan: Live Scan Progress & Activity Log

**Branch**: `001-scan-progress-log` | **Date**: 2026-11-01 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-scan-progress-log/spec.md`

## Summary

Repository scans (onboarding step 0 and main-view refresh) currently block in a
single silent walk; the UI shows only a spinner. This feature streams progress —
current directory, found/inspected repositories, completion and failure — over the
existing typed `AppEvent` bus (ADR__261101-1), renders a condensed live line during
onboarding, and a clickable "Suche läuft…" chip that opens a live **Scan activity
modal** (header: current dir + count + elapsed; append-only log; summary on done).

Technical approach: `RepoScanner::scan_path` gains a `ScanObserver` sink (modeled on
the existing `GitObserver` trait), `service::discover_repositories` emits
`started`/`inspected`/`done` around the walk+inspect phases, `AppEvent` gains a
`ScanProgress` variant mapped to the Tauri event `scan:progress` and SSE event name
`scan-progress`, and the Pinia store keeps a session-only activity log that
`ScanActivityModal.vue`, the sidebar chip, and `OnboardingStepFind.vue` all read.

## Technical Context

**Language/Version**: Rust 2021 (Tauri v2 backend, src-tauri/) · TypeScript + Vue 3 (`<script setup>`, Pinia) frontend

**Primary Dependencies**: `ignore` (WalkBuilder), `tokio` (spawn_blocking, channels, Semaphore), `serde`, `uuid` · `@tauri-apps/api` (listen/invoke), `@topdata-software-gmbh/ui-kit-web-vue3`, `lucide-vue-next`

**Storage**: None new — activity log is session-only in the Pinia store (same retention rule as the Git step log, ADR__261030-4)

**Testing**: `cargo test` (src-tauri) · `vitest` + `@vue/test-utils` (src/lib/__tests__, src/components/__tests__)

**Target Platform**: Tauri desktop window (WebKitGTK on Linux) and browser via `repoman --web` (axum + SSE) — one frontend build, two transports

**Project Type**: Desktop app with embedded web server (src-tauri + src, single workspace)

**Performance Goals**: ≥2 progress updates/sec during a walk; scan wall-clock overhead ≤5% (SC-005); ≤500 rendered log rows with exact counts (FR-013)

**Constraints**: tick events throttled to ≤1 per 400 ms; found/inspected/done never dropped (unbounded); no new HTTP endpoint (single `/api/events`); scanner must not depend on Tauri types; DE + EN strings for all new text (FR-012)

**Scale/Scope**: roots with up to ~100k directories and ~10k repositories per scan; 1 new Rust event payload + trait, 1 new Vue component, store + wiring changes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is the **unfilled stock template** (no principles
defined), so there are no constitution gates to pass or fail. Project-local
governance from `_ai/technical_decisions` is honored explicitly instead:

- ADR__261030-4 (typed event bus, one transport per platform, session-only logs): **PASS** — this feature adds one variant to the existing bus, reuses `/api/events`, persists nothing.
- ADR__261101-1 (scan progress streaming): **PASS** — this plan is its implementation.
- ADR__261031-1 (app-owned translations, dictionary-free kit): **PASS** — all new copy goes through `src/lib/i18n` DE + EN message files.

**Gate result: PASS** (no violations; Complexity Tracking unused).

## Project Structure

### Documentation (this feature)

```text
specs/001-scan-progress-log/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/           # Phase 1 output
│   └── scan-progress-event.md
└── tasks.md             # Phase 2 output (/speckit.tasks — NOT this command)
```

### Source Code (repository root)

```text
src-tauri/src/
├── scanner.rs                   # MODIFY: ScanObserver sink in scan_path; tick throttle + found hook
├── git_log.rs                   # REFERENCE: GitObserver trait pattern to mirror
├── scan_log.rs (or models.rs)   # NEW/EXT: ScanProgressEvent payload + ScanObserver impl for EventBus
├── events.rs                    # MODIFY: AppEvent::ScanProgress(ScanProgressEvent)
├── watcher.rs                   # MODIFY: EventBus implements ScanObserver (publish)
├── service.rs                   # MODIFY: discover_repositories emits started/inspected/done(+error)
├── commands/git_commands.rs     # MODIFY: pass EventBus into the scan call
├── main.rs                      # MODIFY: forward ScanProgress → emit("scan:progress")
└── web.rs                       # MODIFY: map ScanProgress → SSE event "scan-progress"

src/
├── lib/backend.ts               # MODIFY: onScanProgress in subscribeToEvents (listen + EventSource)
├── types/domain.ts              # MODIFY: ScanProgressEvent / ScanActivity types
├── stores/repoStore.ts          # MODIFY: scanActivity slice (entries, counts, finished, modalOpen)
├── components/
│   ├── ScanActivityModal.vue    # NEW: header (dir/count/elapsed/summary) + append-only log (cap 500)
│   ├── Sidebar.vue              # MODIFY: "Suche läuft…" chip → button opening the modal (FR-015)
│   ├── AllReposPage.vue         # MODIFY: empty-state line bound to live progress instead of static text
│   └── onboarding/OnboardingStepFind.vue  # MODIFY: condensed live line (dir + count)
└── lib/i18n/messages.{de,en}.ts # MODIFY: new keys for all strings above
```

**Structure Decision**: Stay within the existing single-workspace layout
(`src-tauri` + `src`). No new packages, no new top-level directories beyond
`specs/001-scan-progress-log/` (docs) — new code lands in the modules that already
own each concern.

## Complexity Tracking

> No constitution violations — table unused.
