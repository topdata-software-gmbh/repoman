# Phase 1 Data Model: Live Scan Progress & Activity Log

Session-only, non-persisted (spec Assumptions). "Frontend store" = `scanActivity`
slice in `repoStore.ts`; "Backend" = Rust types emitted on the bus.

## E1. ScanRun (frontend aggregate, one per `run_id`)

| Field | Type | Source / rule |
|---|---|---|
| `runId` | string (uuid v4) | from `started` event; generated in `discover_repositories` |
| `roots` | string[] | from `started` |
| `running` | bool | true on `started`, false on `done` |
| `currentDir` | string \| null | last `tick.currentDir` for this run |
| `startedAt` | epoch ms | `started.started_at` |
| `entries` | Map<path, FoundRepoEntry> | insertion order = discovery order |
| `finished` | { count, elapsedMs, error? } \| null | from `done` |

**Validation rules**

- Events with `runId` ≠ current `runId` are ignored (FR-009, R6).
- A second `started` replaces the entire slice (new run).
- Header count = `max(entries.size, finished.count)` after `done` (R5).
- `running` implies `finished === null`; terminal states are never reverted by late events (same `runId` `tick` after `done` is ignored).

**State transitions**

```text
idle ──started──▶ running ──done(err=null)──▶ complete ──(modal closed +10s / new started)──▶ idle
                   │                                 ▲
                   └──────done(err≠null)────▶ failed ┘   (failed clears only on next started)
```

## E2. FoundRepoEntry

| Field | Type | Rule |
|---|---|---|
| `path` | string | unique key within a run; `found` events dedup by path (a nested `.git` below an excluded dir can never arrive twice, but defensive dedup = no-op re-insert) |
| `name` | string | file name of `path` (from `found`) |
| `root` | string | the scan-path root being walked when found (multi-root attribution, FR-010) |
| `kind` | 'plugin' \| 'theme' \| 'other' \| null | null until `inspected` (badge shows a dim "…" placeholder) |

- `inspected` updates `kind` **in place** — never appends (FR-005, US2-AC3).

## E3. ScanProgressEvent (Rust payload, `serde` camelCase like `GitStepEvent`)

```text
ScanProgressEvent {
  phase:        started | found | tick | inspected | done,
  run_id:       String,
  root:         Option<String>,      // found, tick
  path:         Option<String>,      // found, inspected
  name:         Option<String>,      // found
  kind:         Option<RepoKind>,    // inspected
  current_dir:  Option<String>,      // tick
  roots:        Vec<String>,         // started
  started_at:   Option<u64>,         // started (epoch ms)
  found_count:  Option<usize>,       // done
  elapsed_ms:   Option<u64>,         // done
  error:        Option<String>,      // done (failure/interruption)
}
```

Validation: `phase` determines required fields; constructors exist only per phase
(`ScanProgressEvent::found(run_id, root, path, name)` …) so an invalid combination
is unconstructable — the frontend may assume well-formed payloads.

## E4. Emitting responsibilities (per R3)

| Emitter | Phases | Notes |
|---|---|---|
| `service::discover_repositories` | started, inspected, done | `done` on **every** exit path (R9) |
| `scanner::scan_path` via `ScanObserver` | found, tick | tick throttled ≤1/400 ms (R4) |
| `EventBus` | forwards all | capacity 8192 (R5) |

## E5. Relationships

- One ScanRun → many FoundRepoEntry (1:*, keyed by path).
- One ScanRun → many root folders; every `found`/`tick` names its root (FR-010).
- A `done` finalizes exactly one ScanRun; the returned `Vec<Repository>` from the
  command remains the authoritative inventory that populates the existing table —
  entries here are a *view*, not a second source (spec Assumptions).
