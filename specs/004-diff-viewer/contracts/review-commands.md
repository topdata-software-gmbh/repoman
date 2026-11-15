# Contract: `review_project` / `review_file` commands and routes

Two read-only requests power the viewer. Neither writes to the repository, the
configuration, or Git history. All Git calls happen in the Rust core; transports
only delegate.

## C1. Transport mapping

| Transport | Surface | Shape |
|---|---|---|
| Desktop (Tauri) | `invoke("review_project", { path, mode, commitHash? })` and `invoke("review_file", { path, mode, commitHash?, filePath, oldPath? })` in `commands/git_commands.rs` | thin delegators → `service::review_project` / `service::review_file` |
| Web (axum) | `POST /api/repositories/review` and `POST /api/repositories/review-file` | same handler core; behind the existing `require_token` middleware, like `/api/repositories/changes` |
| Frontend helper | `reviewProject(request)` / `reviewFile(request)` in `src/lib/backend.ts` | one function each, two transports behind it |

**Rule (Principle II)**: neither transport may run Git, parse porcelain output,
inspect images, or apply caps — all of that lives in `review.rs`/`service`.

## C2. `review_project` request

```json
{ "path": "/srv/projects/theme", "mode": "unsaved" }
```

```json
{ "path": "/srv/projects/theme", "mode": "save", "commitHash": "9f2c…" }
```

- `path` MUST resolve to a Git repository. `mode` is `unsaved` or `save`.
- `commitHash` is optional in `save` mode; absent means the project's current
  `HEAD`. It is ignored in `unsaved` mode.
- `unsaved` covers all uncommitted work: modified, deleted, and renamed files
  (staged or unstaged) plus untracked files, excluding ignored files (FR-002).

## C3. `review_project` response

```json
{
  "path": "/srv/projects/theme",
  "name": "theme",
  "mode": "unsaved",
  "state": "ready",
  "files": [
    {
      "path": "src/app.js",
      "oldPath": null,
      "status": "modified",
      "kind": "text",
      "whitespaceOnly": false,
      "addedLines": 12,
      "removedLines": 3
    },
    {
      "path": "dist/new-logo.png",
      "oldPath": null,
      "status": "new",
      "kind": "image",
      "whitespaceOnly": false,
      "addedLines": 0,
      "removedLines": 0
    }
  ],
  "summary": { "fileCount": 2, "addedLines": 12, "removedLines": 3 },
  "saved": null,
  "error": null
}
```

- `state` is `ready`, `empty`, `unreadable`, or `unavailable` (data-model E2).
  Non-`ready` states return `files: []` and zeroed summary.
- `summary.fileCount` counts a rename once; added/removed totals come from
  numstat (binary/unknown files contribute 0).
- `saved` is `{ "hash", "message", "timestamp" }` in `save` mode and `null`
  otherwise.
- No response field exposes a porcelain status letter, `HEAD`, `@@`, or a commit
  hash in the default UI path (SC-003). `saved.hash` is metadata, not rendered in
  the friendly header.

## C4. `review_file` request

```json
{
  "path": "/srv/projects/theme",
  "mode": "unsaved",
  "filePath": "src/app.js",
  "oldPath": null
}
```

```json
{
  "path": "/srv/projects/theme",
  "mode": "save",
  "commitHash": "9f2c…",
  "filePath": "src/app.js"
}
```

- `filePath` MUST be an entry returned by the matching `review_project` call;
  arbitrary paths are rejected.
- `oldPath` MUST be supplied only for a rename entry.
- `mode`/`commitHash` MUST match the project request; a saved-state review uses
  the same resolved commit for the summary and every file.

## C5. `review_file` response

```json
{
  "path": "src/app.js",
  "oldPath": null,
  "status": "modified",
  "kind": "text",
  "whitespaceOnly": false,
  "addedLines": 12,
  "removedLines": 3,
  "notice": null,
  "text": {
    "rows": [
      {
        "kind": "same",
        "oldNumber": 1,
        "newNumber": 1,
        "oldText": "…",
        "newText": "…"
      },
      {
        "kind": "fold",
        "oldStart": 2,
        "newStart": 2,
        "count": 40,
        "rows": []
      },
      {
        "kind": "changed",
        "oldNumber": 42,
        "newNumber": 42,
        "oldText": "…",
        "newText": "…"
      }
    ],
    "truncated": false,
    "truncationReason": null,
    "raw": "@@ -42,1 +42,1 @@\n…",
    "language": "javascript"
  }
}
```

Image variant:

```json
{
  "path": "assets/logo.png",
  "oldPath": null,
  "status": "modified",
  "kind": "image",
  "whitespaceOnly": false,
  "addedLines": 0,
  "removedLines": 0,
  "notice": null,
  "image": {
    "state": "ready",
    "old": { "dataUrl": "data:image/png;base64,…", "width": 800, "height": 400, "bytes": 51200 },
    "new": { "dataUrl": "data:image/png;base64,…", "width": 800, "height": 420, "bytes": 53000 }
  }
}
```

Binary variant:

```json
{
  "path": "vendor/library.dll",
  "oldPath": null,
  "status": "modified",
  "kind": "binary",
  "whitespaceOnly": false,
  "addedLines": 0,
  "removedLines": 0,
  "notice": "unsupported",
  "binary": { "oldBytes": 1048576, "newBytes": 1048576 }
}
```

- Exactly one of `text`, `image`, `binary` is present and matches `kind`.
- Text stops at the first of **2 MB** preview text or **5,000 diff lines**;
  `truncated` + `truncationReason` are set and the UI shows the rest-not-displayed
  notice (FR-016). `raw` may be `null` when over the cap.
- Images are bounded to **10 MB** and **8,000×8,000 px**; over-limit, corrupt, or
  unsupported bytes set `notice`/`image.state` and carry no data URL (FR-010).
- A deleted or newly added side is `null`; the UI shows only the available side.
- The response never contains raw Git shorthand for the friendly views; `raw` is
  used only when the user explicitly chooses the technical view.

## C6. Refusals and error mapping

| Condition | Transport result | Viewer mapping |
|---|---|---|
| `path` is not a repository / cannot be opened | command `Err("not-a-repository")` or project `state: "unreadable"` | project error state naming the project and the next action; other projects stay reviewable (FR-013) |
| Saved `commitHash` no longer resolves | project `state: "unavailable"`, `error.code: "save-not-found"` | plain-language unavailable notice (FR-017) |
| `mode: "save"` but the repository has no `HEAD` | project `state: "unavailable"`, `error.code: "head-not-found"` | unavailable notice |
| `filePath` not in the inventory | command `Err("unknown-file")` | inline file notice; the list stays usable |
| Requested file vanished between summary and preview | file `notice: "missing"` | inline "no longer available" notice |
| Preview exceeds bounds | success with `truncated`/`notice` | explicit too-large/truncated notice, no hang (SC-007) |

Error strings sent to the UI are **stable codes**, not prose; `src/lib/review.ts`
maps each code to DE+EN copy with a next action (Principle IV). Technical detail
may be logged by the core but is not displayed.

## C7. Invariants

- **Read-only**: no `add`, `commit`, `reset`, `checkout`, `restore`, `clean`,
  `stash`, `apply`, or any other mutating Git subcommand is invoked.
- **Bounded**: text and image payloads obey C5; the number of file entries is not
  capped (FR-022).
- **Isolated**: one unreadable project/file never fails the whole session.
- **Deterministic**: a response is a snapshot of the requested revision/worktree
  at request time; a later filesystem change does not alter it.
- **Two transports**: the desktop and web responses are the same DTOs and the
  same caps; no transport-specific fields.
