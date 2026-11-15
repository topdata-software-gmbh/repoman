# Phase 1 Data Model: Read-Only Change Viewer

All viewer state is **session-only**; nothing here is persisted in `AppConfig`.
"Backend" types are Rust DTOs serialized `camelCase`; "frontend" types mirror
them in `src/types/domain.ts`. A review never mutates repository state.

## E1. ReviewSession (frontend aggregate, one per open viewer)

| Field | Type | Rule |
|---|---|---|
| `open` | boolean | true while the modal is mounted |
| `mode` | `'unsaved' \| 'save'` | fixed for the whole session |
| `origin` | `'row' \| 'saveChanges' \| 'activity'` | affects only the back path; session behavior is identical |
| `projects` | `Array<{ path, name }>` | deduplicated, ordered; ≥1. Row/Save-Changes order follows the caller; activity has one project |
| `activeIndex` | number | index into `projects`; always valid while `open` |
| `displayMode` | `'sideBySide' \| 'unified' \| 'raw'` | default `sideBySide`; preserved across files and projects (FR-008, US5) |
| `comparisons` | `Map<path, ProjectComparison \| null>` | `null` = not loaded yet |
| `previews` | `Map<string, FilePreview \| null>` | key = `` `${projectPath}\u0000${filePath}` ``; `null` = loading/not loaded |
| `selectedFileByProject` | `Map<path, string \| null>` | remembered per project; restored on return (US2-AC3, SC-006) |
| `staleProjects` | `Set<path>` | set by `repo:fs-changed`; content is never replaced automatically (FR-015) |
| `loadingProject` | boolean | one summary request at a time |
| `loadingFile` | boolean | one preview request at a time |
| `saveRefs` | `Map<runId, SaveRef>` | session map shared with the activity drawer (E7) |

**Validation rules**

- Opening replaces any previous session; closing clears all viewer maps
  (save references survive, because the activity log survives).
- `activeIndex` changes by exactly one action per previous/next hop.
- Selecting a file loads its preview once; returning to it reuses the cached
  preview until *Aktualisieren* or project navigation discards it.
- `displayMode` survives project/file changes for the rest of the session.
- A staleness flag only shows the refresh prompt; it never swaps content.
- A failed project stays isolated; moving away and back shows the same error
  state without blocking other projects (FR-013, US2-AC4).

**State transitions**

```text
viewer idle ──open(row|saveChanges|activity)──▶ project loading
    project loading ──ok──▶ ready/empty
    project loading ──error──▶ unreadable/unavailable (isolated)
    ready/empty ──file select──▶ file loading ──ok──▶ text|image|binary|notice
    ready ──fs-changed──▶ ready+stale ──Aktualisieren──▶ project loading
    any ──Fertig|Escape──▶ viewer idle (session cleared)
```

## E2. ProjectComparison (backend → frontend)

| Field | Type | Rule |
|---|---|---|
| `path` | string | absolute repository path |
| `name` | string | last path segment; plain display name |
| `mode` | `'unsaved' \| 'save'` | echoed from the request |
| `state` | `'ready' \| 'empty' \| 'unreadable' \| 'unavailable'` | non-`ready` states carry no files |
| `files` | `DiffFileEntry[]` | ordered by display path (case-insensitive); empty unless `ready` |
| `summary.fileCount` | number | `files.length`; renames count once |
| `summary.addedLines` | number | sum of numstat additions; binary/unknown contribute 0 |
| `summary.removedLines` | number | sum of numstat deletions; binary/unknown contribute 0 |
| `saved` | `{ hash, message, timestamp } \| null` | `save` mode only; read with `git show -s --format=%s%x00%ct` |
| `error` | `{ code, detail? } \| null` | `code` is stable (`not-a-repository`, `unreadable`, `save-not-found`, `head-not-found`); `detail` is technical and never shown verbatim |

**Validation rules**

- `state = 'empty'` means a readable repository with zero reviewable changes
  (US1-AC4); it is not an error.
- `state = 'unavailable'` is used when a requested saved state no longer
  resolves (FR-017 edge case).
- `summary` is always present (zeros for non-ready states) so the header never
  jumps or renders `undefined`.
- The project header shows only `name`, file count, and added/removed totals —
  never a hash, branch, or status code (SC-003).

## E3. DiffFileEntry (one row in the plain-language file list)

| Field | Type | Rule |
|---|---|---|
| `path` | string | display path (new path for a rename) |
| `oldPath` | string \| null | set only for renames |
| `status` | `'new' \| 'modified' \| 'deleted' \| 'renamed' \| 'conflict'` | mapped to *Neu*, *Geändert*, *Gelöscht*, *Umbenannt*, *Konflikt* |
| `kind` | `'text' \| 'image' \| 'binary'` | decided from numstat binary marker, NUL sample, and image extension/sniffing |
| `whitespaceOnly` | boolean | true only for a tracked text file whose normal diff is non-empty and whose whitespace-insensitive diff is empty |
| `addedLines` | number | from numstat; 0 for binary or unreadable stats |
| `removedLines` | number | from numstat; 0 for binary or unreadable stats |

**Validation rules**

- `status = 'conflict'` **only** when porcelain reports an unmerged path **and**
  `detect_operation_state` is `merging`, `rebasing`, or `cherry-picking`
  (spec clarification; FR-006).
- Untracked files are `new`; deleted files are `deleted`; rename detection uses
  `-M` plus the porcelain rename pair.
- The list never exposes single-letter porcelain codes (FR-006, SC-003).

## E4. FilePreview (backend → frontend, one selected file)

Common fields: `path`, `oldPath`, `status`, `kind`, `whitespaceOnly`,
`addedLines`, `removedLines`, `notice` (`'too-large' | 'unsupported' |
'corrupt' | 'missing' | null`).

| Variant | Fields | Rule |
|---|---|---|
| text | `text.rows: DiffLine[]`, `text.truncated`, `text.truncationReason: 'bytes' \| 'lines' \| null`, `text.raw: string \| null`, `text.language: string \| null` | rows stop at the first of 2 MB or 5,000 diff lines; `raw` is the unified technical form and is omitted when over the cap; `language` is a highlight.js id or `plaintext` |
| image | `image.state: 'ready' \| 'too-large' \| 'unsupported' \| 'corrupt' \| 'missing'`, `image.old`, `image.new` | each side is `{ dataUrl, width, height, bytes } \| null`; bounds: 10 MB and 8,000×8,000 px per side |
| binary | `binary.oldBytes`, `binary.newBytes` | rendered as *"Binärdatei geändert"* with a size change |

**Validation rules**

- At least one of `text`, `image`, `binary` is present; exactly one matches
  `kind`.
- A missing side (deleted or newly added file) is `null`, never a broken image.
- An oversized/corrupt image sets `notice`/`image.state` and carries no data URL.
- Truncation is explicit (`truncated` + reason) so the UI can say the rest is not
  displayed (FR-016).
- Preview bytes are produced by the core; the frontend never reads a repository
  file directly.

## E5. DiffLine (text row / fold)

```text
DiffLine = DiffRow | DiffFold

DiffRow {
  kind: 'same' | 'changed' | 'added' | 'removed',
  oldNumber: number | null,
  newNumber: number | null,
  oldText: string | null,
  newText: string | null
}

DiffFold {
  kind: 'fold',
  oldStart: number,
  newStart: number,
  count: number,
  rows: DiffRow[]        // unchanged rows, already bounded
}
```

**Validation rules**

- `same` has both texts and both numbers; `added` has only the new side;
  `removed` has only the old side; `changed` has both sides on one paired row.
- Folds contain only `same` rows and are collapsed by default in the friendly
  view; expanding reveals exactly those rows (FR-007).
- Unified mode derives one column from the same rows in document order; raw mode
  shows `text.raw` (or the same rows when raw is unavailable due to truncation).
- The default/friendly views never render `@@`, `HEAD`, hashes, or status codes
  (SC-003); raw mode is the only place they may appear.

## E6. SaveRef (frontend, session-only)

| Field | Type | Rule |
|---|---|---|
| `runId` | string | the commit run's `runId`, matching its Git step events |
| `repoPath` | string | repository that was saved |
| `commitHash` | string | HEAD hash read immediately after a successful save |
| `savedAt` | string | ISO timestamp captured by the frontend when the result arrives |

**Rules**

- `repoStore.commitRepositories` records one `SaveRef` per successful commit
  result that carries both `runId` and `commitHash`.
- The activity drawer offers *Ansehen* for a `commit` run when a `SaveRef`
  exists, or when the run's repository still has a resolvable `HEAD` (fallback).
- A failed commit creates no `SaveRef`.
- The map is cleared only by an app reload, never by closing the viewer.

## E7. OperationResult (existing type, additive)

| Field | Type | Rule |
|---|---|---|
| `repoPath` | string | unchanged |
| `success` | boolean | unchanged |
| `output` | string | unchanged |
| `error` | string \| null | unchanged |
| `runId` | string \| null | set by `execute_macro`; matches the Git step run for direct commit calls |
| `commitHash` | string \| null | set by `commit_changes` after a successful commit |

**Compatibility**: both new fields are optional with a default, camelCase
serialized, and ignored by existing consumers. Group actions expose `runId` but
never `commitHash`; only commit runs feed E6.

## E8. Backend request DTOs

```text
ReviewProjectRequest {
  path: string,
  mode: 'unsaved' | 'save',
  commitHash?: string        // save mode: exact saved state; absent = current HEAD
}

ReviewFileRequest {
  path: string,
  mode: 'unsaved' | 'save',
  commitHash?: string,
  filePath: string,          // E3.path
  oldPath?: string           // E3.oldPath for renames
}
```

**Validation rules**

- `path` must resolve to a Git repository (`GitRunner::resolve_git_dir`).
- `filePath` must appear in the project's current inventory; arbitrary paths are
  rejected, so a request cannot read outside the reviewed change set.
- `mode = 'save'` with an unresolvable `commitHash` → `ProjectComparison.state =
  'unavailable'` (and the same for file requests), never a crash.
- `mode = 'unsaved'` ignores `commitHash`.

## E9. Relationships

- One ReviewSession → 1..N projects; each project has at most one remembered
  selected file and at most one cached `ProjectComparison`.
- One ProjectComparison → 0..N `DiffFileEntry`; one selected entry → exactly one
  `FilePreview`.
- One `DiffFileEntry` → one `FilePreview` per time it is opened.
- One successful save → one `SaveRef`; one `SaveRef` → one saved-state review
  (`commit^..commit`).
- Project summaries and file previews are views of Git state, not a second
  source of truth; the repository remains authoritative.
