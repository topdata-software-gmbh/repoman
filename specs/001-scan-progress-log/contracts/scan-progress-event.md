# Contract: `scan:progress` event stream

This feature adds **no new HTTP endpoint and no new Tauri command**. It defines one
new event on the existing two transports (ADR__261030-4) plus the UI surfaces that
consume it.

## C1. Transport mapping

| Transport | Channel | Event name |
|---|---|---|
| Desktop (Tauri) | `handle.emit` in `main.rs` forwarder | `scan:progress` |
| Web (axum SSE) | existing single `GET /api/events` stream | SSE event `scan-progress` |
| Frontend helper | `subscribeToEvents({ onScanProgress })` in `src/lib/backend.ts` | both, behind one handler |

Payload: JSON `ScanProgressEvent` (identical on both transports; SSE sends it as
the `data:` JSON string, same convention as `git-step`).

## C2. Payload schema (JSON, camelCase)

```json
{
  "phase":      "started | found | tick | inspected | done",
  "runId":      "string (uuid v4)",
  "roots":      ["string"],            // phase=started
  "startedAt":  1730457600000,         // phase=started, epoch ms
  "root":       "string",              // phase=found|tick — the root being walked
  "path":       "string",              // phase=found|inspected — repo path
  "name":       "string",              // phase=found — repo folder name
  "kind":       "plugin|theme|other",  // phase=inspected
  "currentDir": "string",              // phase=tick
  "foundCount": 47,                    // phase=done — authoritative total
  "elapsedMs":  134021,                // phase=done
  "error":      "string|null"          // phase=done — set on failure/interruption
}
```

## C3. Ordering & frequency guarantees

1. Exactly one `started` before any other event of a run; exactly one `done`
   terminalizes the run. After `done`, no further events carry that `runId`.
2. `tick`: at most 1 per 400 ms and at least 1 per 500 ms while a root is being
   walked (heartbeat floor, SC-001); droppable by design (broadcast lag).
3. `found`: never throttled; may be dropped **only** in extreme broadcast-lag
   bursts — `done.foundCount` is authoritative, consumers must reconcile counts
   with `max(seen, foundCount)`.
4. `inspected`: at most once per previously-`found` path; may be dropped like
   `found` (row keeps the "kind pending" badge state).
5. Within a run, no ordering guarantee between `found` paths from concurrent
   processing except real-time arrival order.

## C4. TypeScript mirror (`src/types/domain.ts`)

```ts
export interface ScanProgressEvent {
  phase: 'started' | 'found' | 'tick' | 'inspected' | 'done';
  runId: string;
  roots?: string[];
  startedAt?: number;
  root?: string;
  path?: string;
  name?: string;
  kind?: RepoKind;
  currentDir?: string;
  foundCount?: number;
  elapsedMs?: number;
  error?: string | null;
}
```

## C5. UI contract (consumer behavior, from spec FRs)

- **Sidebar chip**: rendered only while `store.isScanning || store.scanActivity.summaryVisible`;
  click → `openScanModal()`; stays clickable ≈10 s after `done` (FR-003, FR-015).
- **ScanActivityModal**: header = `currentDir` (middle-ellipsis, `title`=full) +
  found count + elapsed (1 Hz while running) **or** summary when `finished`
  (FR-006/007); body = up to newest 500 `entries` + "…und N mehr" (FR-013);
  auto-scroll pauses on user scroll-up; error state replaces spinner text (FR-008).
- **OnboardingStepFind / AllReposPage empty state**: single condensed line
  `Prüfe {dir}… · {count}` fed by the same store slice (FR-001/002, FR-011).
- All labels via `t('scan.*')` DE+EN (FR-012).
