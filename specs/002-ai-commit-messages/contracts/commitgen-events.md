# Contract: `commitgen:repo` event stream

One new event on the existing two transports (ADR__261030-4 pattern, same as
feature 001's `scan:progress`), carrying per-repository generation lifecycle.
There is no second result channel for drafts.

## C1. Transport mapping

| Transport | Channel | Event name |
|---|---|---|
| Desktop (Tauri) | `handle.emit` in `main.rs` forwarder from `AppEvent::CommitGen` | `commitgen:repo` |
| Web (axum SSE) | existing single `GET /api/events` stream | SSE event `commitgen-repo` |
| Frontend helper | `subscribeToEvents({ onCommitGenRepo })` in `src/lib/backend.ts` | both, behind one handler |

Payload: JSON `CommitGenEvent` (identical on both transports; SSE sends it as
the `data:` JSON string, same convention as `git-step` / `scan-progress`).

## C2. Payload schema (JSON, camelCase)

```json
{
  "runId":  "string (uuid v4)",
  "path":   "/abs/path/to/repo",
  "phase":  "started | done | error",
  "draft":  {                       // phase=done only
    "type": "feat",
    "scope": "modal|null",
    "subject": "add AI commit message generation",
    "body": "…|null",
    "rendered": "feat(modal): add AI commit message generation"
  },
  "error":  "string|null"           // phase=error only — human-readable reason
}
```

Field rules mirror `data-model.md` E2/E3; `rendered` is computed by the backend
(`draft.render()`) and is the only string the UI seeds into a row.

## C3. Ordering & lifecycle guarantees

1. Per `(runId, path)`: exactly one `started` precedes any terminal event;
   exactly one terminal (`done` **or** `error`) closes the attempt.
2. Retry = a fresh `started` + terminal pair for the same `path` under the same
   `runId`. No event for a `path` may be emitted while an attempt for it is
   still open.
3. After the last open attempt for every `path` of a run reaches a terminal
   event, the run is complete — there is no aggregate `done` event; the store
   derives completion from its rows (keeps retry semantics local, FR-008).
4. Events are throttled nowhere (per-repo events are rare by construction);
   ordering between different `path`s is unspecified — consumers key solely on
   `path`.

## C4. Consumers

| Surface | Behavior |
|---|---|
| `repoStore` commitgen slice | applies transitions per data-model E4; ignores unknown `runId` |
| `SaveChangesModal.vue` rows | badge `generating` → editable draft / error + retry icon |
| i18n | `error` strings are *reasons*, rendered through `save.ai.errors.*` DE/EN with a next-action hint (Principle IV) |
