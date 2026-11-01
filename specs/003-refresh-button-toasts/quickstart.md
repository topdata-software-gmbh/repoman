# Quickstart & Validation: Refresh Button Feedback Toasts

**Feature**: `003-refresh-button-toasts` | **Date**: 2026-11-01

Use this guide to prove the feature end-to-end. Contracts and shapes are defined
in [contracts/store-refresh-contract.md](./contracts/store-refresh-contract.md)
and [data-model.md](./data-model.md); do not duplicate them here.

## Prerequisites

- Node + `pnpm` installed; dependencies present (`pnpm install` if `node_modules`
  is missing).
- For the desktop/manual check: the Tauri toolchain, or run the web mode used by
  the project's existing start script.
- App language switchable between German and English (existing language
  switcher).

## Automated gates (blocking)

Run from the repository root:

```bash
pnpm test          # vitest run
pnpm typecheck     # vue-tsc --noEmit
```

Expected:
- All tests pass, including the new store refresh-toast tests.
- Type checking passes; this also proves every new i18n key exists in both
  catalogues and that `RefreshResult` is narrowed correctly at both call sites.

## Store-level validation scenarios

These correspond to the tests described in the plan; they can be run directly
with Vitest.

1. **Success content** — given a repository on `main` with unsaved changes,
   calling `refreshRepoWithToast(path)` yields one `success` toast whose title
   contains the project name and whose message is
   `main · Änderungen nicht gespeichert` (language set to `de`).
2. **Inaccessible result** — given `refreshRepository` resolves a repository with
   `isAccessible: false`, the action yields one `error` toast whose message is
   the existing "folder not found" label, and the row remains in the store.
3. **Thrown transport error** — given `refreshRepository` rejects with a
   multi-line error, the action yields one `error` toast whose message is only
   the first line, trimmed.
4. **Background silence** — calling `refreshSingleRepo(path)` directly (the
   watcher/post-action path) adds zero toasts, and after a rejected refresh the
   repositories array is unchanged and no rejection escapes.

## Manual UI validation

1. Start the app (existing dev/start script) with at least one accessible
   project and one project whose folder has been removed.
2. Click the refresh icon on the accessible row.
   - Expect a success toast naming the project, message = branch plus status
     labels (e.g. `main · Auf dem neuesten Stand`).
   - The row spinner appears during the call and clears afterwards.
3. Click refresh on the removed-folder row.
   - Expect an error toast naming the project with the "folder not found"
     message; the row stays visible and dimmed as before.
4. Switch the language to English and repeat steps 2–3.
   - Expect `{name} refreshed` / `Could not refresh {name}` and English summary
     labels.
5. Trigger a pull or let the file watcher fire on a changed repository.
   - Expect the existing pull toast (or no toast for a pure watcher event) and
     **no** "refreshed" toast.

## Done criteria

- Automated gates green.
- All four store-level scenarios pass.
- Manual checks 2–5 behave as described in both languages.
- No change in pull/push/commit/branch-switch toast counts.
