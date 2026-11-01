---
filename: "_ai/backlog/active/261101_0159__DESIGN_HANDOFF__refresh-button-toasts.md"
title: "Design Handoff: Refresh button success/error toasts"
createdAt: 2026-11-01 01:59
updatedAt: 2026-11-01 01:59
status: validated
tags: [ui, toasts, refresh, repoman]
documentType: DESIGN_HANDOFF
---

# Design Handoff: Refresh button success/error toasts

## Context

The per-row refresh button in `RepoListTable.vue` gives no feedback after a
manual refresh. `store.refreshSingleRepo()` silently swallows failures, so the
user cannot tell whether the refresh succeeded. The request: after clicking the
refresh button, show a toast on success and on error.

## Decisions

- **Toast on both outcomes, for manual per-row refresh only.**
  Success and failure of a button-triggered refresh both produce a toast.

- **Success toast content: project name + state summary.**
  Title carries the name (`'{name} aktualisiert'`), message is the state summary.
  Summary = `branchLabel(repo)` plus every `statusBadges(repo)` label, joined
  with ` · ` (e.g. `main · Änderungen nicht gespeichert · 1 zu senden`,
  `main · Auf dem neuesten Stand`). Reuses existing copy; never drifts from the
  row.

- **Toasts are opt-in from the button path, never from `refreshSingleRepo`.**
  `refreshSingleRepo` stays silent because it is shared with the file watcher
  and post-action refreshes. Toasting inside it would duplicate pull/push toasts
  and spam on every filesystem event.

- **Implementation shape: result-returning refresh + dedicated wrapper action.**
  - `type RefreshResult = { ok: true; repo: Repository } | { ok: false; error: string }`.
  - `refreshSingleRepo(path)` always updates the repositories array, returns
    `{ ok: true, repo }` when accessible, `{ ok: false, error }` otherwise, and
    **never throws** (keeps watcher/post-action callers safe).
  - New `refreshRepoWithToast(path)` calls it and emits the toast; the button
    calls this action. Store keeps all i18n copy.

- **Inaccessible repo counts as failure.** Because the backend returns a
  `Repository` with `isAccessible: false` rather than rejecting, `!isAccessible`
  maps to an error toast with the existing `status.folderNotFound` label as the
  message.

- **Error toast: name in title + first line of raw error.**
  Title `'{name} konnte nicht aktualisiert werden'`; message is
  `error.trim().split('\n')[0]` (same convention as `summarizeResults`).

- **New i18n keys** in `messages.de.ts` and `messages.en.ts`:
  - `toast.refreshDone`: `'{name} aktualisiert'` / `'{name} refreshed'`
  - `toast.refreshFailed`: `'{name} konnte nicht aktualisiert werden'` /
    `'Could not refresh {name}'`

## Rejected Alternatives

- **`refreshSingleRepo(path, { notify?: boolean })`** — a boolean flag on a
  shared hot-path function is easy to misuse and hides the distinction between
  background and user-triggered refresh.
- **Component builds the toast text** — moves i18n/message logic into
  `RepoListTable.vue` and duplicates store-owned copy.
- **Throw from `refreshSingleRepo`, catch at the button** — smallest diff, but
  risks unhandled rejections from `App.vue`'s watcher and post-action refreshes
  that call it fire-and-forget.
- **Toast on every refresh** — duplicate toasts after pull/push/commit/branch
  switch and background watcher spam.
- **Generic error title** (`'Aktualisieren fehlgeschlagen'`) — less scannable
  when several projects are involved; name in the title matches pull/push style.
- **Custom prose summary** (e.g. `'main ist aktuell'`) — new keys and drift risk
  versus reusing `branchLabel` + `statusBadges`.
- **Success toast for inaccessible repos** — would read like the refresh worked;
  the folder being gone is a failure from the user's perspective.

## Constraints Discovered

- Toast infrastructure already exists: `showToast` (repoStore.ts:486),
  `ToastStack.vue`, `TdToastStack`, and `toast.*` i18n keys.
- `pull`/`push` already toast through `summarizeResults` (repoStore.ts:521).
- `refreshSingleRepo` is called from `App.vue:28` (watcher) and from
  `executeGroupAction`, `runRepoAction`, `commitRepositories`,
  `switchGroupBranch`.
- The Rust `refresh_repository` service never rejects for a missing folder — it
  returns a `Repository` with `isAccessible: false`. Only transport/HTTP errors
  reject.
- `repoLabel(path)` already exists in the store; `branchLabel` and
  `statusBadges` live in `src/lib/repoStatus.ts`, which imports only i18n (no
  import cycle when the store imports it).
- The button is at `RepoListTable.vue:195`; a local `refreshing` ref already
  guards concurrent clicks and drives the spinner.

## Open Questions

None.

## Scope Boundary

**In:** `RefreshResult` type, `refreshSingleRepo` result return, new
`refreshRepoWithToast` action and export, `RepoListTable.vue` wiring, DE/EN
i18n keys, store-level tests (success content, inaccessible → error, transport
error → first line, no toast from background refresh).

**Out:** backend/Rust changes, pull/push/commit/switch toast behavior, file
watcher behavior, any group-level or global refresh button, toast component
styling/animation.
