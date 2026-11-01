# Contract: Store Refresh Actions & Toast Copy

**Feature**: `003-refresh-button-toasts` | **Date**: 2026-11-01

This project exposes no public network API for this feature. The contracts below
are the internal frontend interfaces that must stay stable across the feature
and its tests.

## 1. `RefreshResult` (exported type)

```ts
export type RefreshResult =
  | { ok: true; repo: Repository }
  | { ok: false; error: string };
```

**Guarantees**
- The union is exhaustive; callers narrow with `result.ok`.
- No other fields are present on either variant.

## 2. `refreshSingleRepo(path: string): Promise<RefreshResult>`

**Contract**
- Resolves — never rejects — for every input path.
- On a successful refresh (`Repository.isAccessible === true`):
  - updates the matching entry in `repositories` in place; if the path is not
    present, appends it and keeps the array sorted by `path`;
  - resolves `{ ok: true, repo }` with the refreshed repository.
- On an inaccessible result (`isAccessible === false`):
  - updates the entry exactly as above;
  - resolves `{ ok: false, error }` where `error` explains the folder is not
    found.
- On a thrown transport/backend error:
  - leaves the repositories array unchanged;
  - resolves `{ ok: false, error: String(error) }`.
- Emits **no** toast in any case.

**Consumers that keep the silent behavior**: `App.vue` live-update watcher,
`executeGroupAction`, `runRepoAction`, `commitRepositories`,
`switchGroupBranch`, and any future background caller.

## 3. `refreshRepoWithToast(path: string): Promise<void>`

**Contract**
- Calls `refreshSingleRepo(path)` exactly once.
- On `ok: true`, emits exactly one `success` toast:
  - `title = t('toast.refreshDone', { name })` where `name` is the project name
    (`repo.name`, falling back to `repoLabel(path)` when needed);
  - `message = [branchLabel(repo), ...statusBadges(repo).map(b => b.label)].join(' · ')`.
- On `ok: false`, emits exactly one `error` toast:
  - `title = t('toast.refreshFailed', { name })`;
  - `message = error.trim().split('\n')[0] || t('common.unknownError')`;
  - when the failure is an inaccessible repository, `message` is
    `t('status.folderNotFound')`.
- Never rejects; never mutates the repositories array beyond what
  `refreshSingleRepo` does.
- Is exported from the store so the component can call it.

## 4. Component contract — `RepoListTable.vue`

- The per-row refresh button's click handler calls
  `store.refreshRepoWithToast(repo.path)`.
- The local `refreshing` guard and spinner behavior are unchanged.
- The component adds no toast copy of its own.

## 5. i18n key contract

Both catalogues must define all keys; the `MessageKey` union is derived from the
English catalogue, so a missing English key fails type checking.

| Key | Parameters | DE | EN |
|---|---|---|---|
| `toast.refreshDone` | `name` | `{name} aktualisiert` | `{name} refreshed` |
| `toast.refreshFailed` | `name` | `{name} konnte nicht aktualisiert werden` | `Could not refresh {name}` |

Reused existing keys (must not change): `status.folderNotFound`,
`common.unknownError`, and every label returned by `branchLabel` /
`statusBadges`.

## 6. Non-goals (contract boundaries)

- No change to `backend.refreshRepository(path, groupIds)` or its Rust command.
- No change to `ToastStack.vue`, `TdToastStack`, toast timing, or styling.
- No new toast for pull, push, commit, branch switch, or live-update refreshes.
