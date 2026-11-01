# Data Model: Refresh Button Feedback Toasts

**Feature**: `003-refresh-button-toasts` | **Date**: 2026-11-01

This feature introduces no persisted data. The entities below are in-memory
shapes used by the store and the toast stack.

## RefreshResult

The outcome of one repository refresh, returned by `refreshSingleRepo(path)`.

| Variant | Field | Type | Meaning |
|---|---|---|---|
| success | `ok` | `true` | The repository was fetched successfully |
| success | `repo` | `Repository` | The refreshed repository, with `isAccessible: true` |
| failure | `ok` | `false` | Refresh failed or the folder is unavailable |
| failure | `error` | `string` | Raw error text (transport) or the folder-not-found explanation |

**Invariant**: `refreshSingleRepo` always resolves to one of these variants; it
never rejects. The repositories array is updated for both success and the
inaccessible-result case.

**State transitions**

```text
[idle row] --click refresh--> [refreshing] --ok:true--> [idle row, success toast]
                                             --ok:false-> [idle row, error toast]
```

The row returns to `idle` in all cases (the component's `finally` clears the
spinner).

## Repository (existing, unchanged)

Used read-only to build toast copy. Relevant fields:

| Field | Type | Used for |
|---|---|---|
| `name` | `string` | Toast title |
| `path` | `string` | Locating/updating the row, `repoLabel` fallback |
| `isAccessible` | `boolean` | Success vs error classification |
| `state.currentBranch` | `string` | `branchLabel(repo)` |
| `state.isDetachedHead` | `boolean` | `branchLabel(repo)` |
| `state` (whole object) | `GitRepoState` | `statusBadges(repo)` labels |

## Toast (existing, unchanged)

Produced through `showToast(kind, title, message?)`.

| Field | Type | Constraints |
|---|---|---|
| `id` | `number` | Store-assigned, monotonically increasing |
| `kind` | `'success' \| 'error' \| 'info'` | Refresh uses `success` or `error` only |
| `title` | `string` | Contains the project name; from `toast.refreshDone` / `toast.refreshFailed` |
| `message` | `string?` | Success: joined summary; error: reason |

## Validation Rules

- **VR-001**: Exactly one toast per accepted refresh click (the component guard
  rejects clicks while a refresh is running).
- **VR-002**: Success requires an accessible repository; `!isAccessible` maps to
  the error variant with `status.folderNotFound` as the message.
- **VR-003**: Error message is the trimmed first line of the raw error; empty
  text falls back to `common.unknownError`.
- **VR-004**: Success summary always starts with the branch label and joins the
  remaining status labels with ` · ` even when there are zero status labels.
- **VR-005**: Background refreshes (`refreshSingleRepo` direct calls) produce no
  toast.
- **VR-006**: A failed refresh leaves any existing `Repository` entry in place.

## Relationships

```text
RepoListTable row --click--> refreshRepoWithToast(path)
                                └─> refreshSingleRepo(path) : RefreshResult
                                      ├─> updates Repository in repositories[]
                                      └─> RefreshResult consumed to build Toast
                                            └─> showToast -> toasts[] -> ToastStack
```

`refreshRepoWithToast` is the only path from a user click to a refresh toast;
`refreshSingleRepo` is shared and remains side-effect-free with respect to
toasts.
