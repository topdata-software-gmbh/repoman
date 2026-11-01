# Research: Refresh Button Feedback Toasts

**Feature**: `003-refresh-button-toasts` | **Date**: 2026-11-01

All items below are decisions, not open questions. The design handoff already
validated the direction; this document records the rationale and the codebase
evidence.

## R1 — Where the toast decision lives

**Decision**: Put the toast in a new store action `refreshRepoWithToast(path)`.
Leave `refreshSingleRepo(path)` toast-free and make it return a result.

**Rationale**: `refreshSingleRepo` is called from the live-update watcher
(`src/App.vue:28`) and from `executeGroupAction`, `runRepoAction`,
`commitRepositories`, and `switchGroupBranch` (`src/stores/repoStore.ts:662`,
`:676`, `:705`, `:717`). A toast inside it would fire on every filesystem event
and duplicate the pull/push/commit toasts produced by `summarizeResults`.

**Alternatives considered**:
- Optional `notify` flag on `refreshSingleRepo` — hides the distinction between
  background and user-triggered refresh and is easy to misuse on a hot path.
- Building toast text in `RepoListTable.vue` — moves i18n and message logic out
  of the store, which owns all existing copy.
- Throwing from `refreshSingleRepo` and catching at the button — risks
  unhandled rejections from fire-and-forget background callers.

## R2 — How `refreshSingleRepo` reports failure

**Decision**: Return a discriminated result
`{ ok: true; repo: Repository } | { ok: false; error: string }`; never throw.

**Rationale**: The Rust `refresh_repository` service never rejects for a missing
folder — it returns a `Repository` with `isAccessible: false`. Only transport or
HTTP failures reject. Treating both as `ok: false` lets the caller map one
consistent error path. Never throwing preserves the safety of existing
background callers.

**Alternatives considered**:
- Resolve `null` on error — loses the error text needed for the toast message.
- Keep `Promise<void>` and let the component re-read the row — cannot
  distinguish "refreshed and accessible" from "refreshed but inaccessible".

## R3 — Success toast content

**Decision**: Title `t('toast.refreshDone', { name })`; message joins
`branchLabel(repo)` and every `statusBadges(repo).label` with ` · `.

**Rationale**: `branchLabel` and `statusBadges` already live in
`src/lib/repoStatus.ts` and drive what the row shows. Reusing them guarantees
the toast cannot drift from the table and requires no new status prose.
`repoStatus.ts` imports only i18n, so the store importing it creates no cycle.

**Alternatives considered**:
- A hand-written summary (`'main ist aktuell'`) — new copy and a second source
  of truth.
- A generic success title — less scannable when several projects are involved.

## R4 — Error toast content

**Decision**: Title `t('toast.refreshFailed', { name })`. Message:
- inaccessible repo → `t('status.folderNotFound')`;
- thrown/transport error → `(error.trim().split('\n')[0])`, falling back to
  `t('common.unknownError')` when empty.

**Rationale**: The first-line convention already exists in `summarizeResults`
(`src/stores/repoStore.ts:521`), so failures read consistently across the app.
The folder-not-found label already explains the most common inaccessible case.
An inaccessible repo is a failure from the user's perspective — a success toast
would read as if the refresh worked.

**Alternatives considered**:
- A generic failure title — loses which project failed.
- Raw multi-line error — the toast stack is not designed for stack traces.

## R5 — i18n placement

**Decision**: Add `toast.refreshDone` and `toast.refreshFailed` to
`src/lib/i18n/messages.de.ts` and `messages.en.ts` in the same block as the
existing `toast.*` keys.

**Rationale**: Constitution IV requires complete DE/EN copy before merge and
forbids literals in components. The `MessageKey` union is derived from the
English catalogue, so adding the key there makes a missing German key a compile
error.

**Copy chosen** (matches existing pull/push naming style):

| Key | DE | EN |
|---|---|---|
| `toast.refreshDone` | `{name} aktualisiert` | `{name} refreshed` |
| `toast.refreshFailed` | `{name} konnte nicht aktualisiert werden` | `Could not refresh {name}` |

## R6 — Component wiring and concurrency

**Decision**: Keep `refreshing` local to `RepoListTable.vue` as the
single-flight guard and spinner source; only change what `refresh()` calls.

**Rationale**: The guard already prevents concurrent clicks and drives the
per-row spinner (`RepoListTable.vue:31-39`, `:195`). Moving it to the store
would change unrelated behavior and risk blocking a second row.

## R7 — Testing approach

**Decision**: Store-level Vitest tests with a mocked `../lib/backend` and a
fresh Pinia, asserting `store.toasts` after each call.

**Rationale**: The project has no store test directory yet, but
`src/components/__tests__/onboarding-find-progress.test.ts` establishes the
`vi.mock` + `setActivePinia` pattern. Testing the action directly covers the
success content, inaccessible mapping, thrown-error mapping, and background
silence without coupling to template details. `pnpm typecheck` covers the DE/EN
key completeness and the `RefreshResult` narrowing.

## Resolved Unknowns

| Technical Context item | Resolution |
|---|---|
| Backend behavior on missing folder | Returns `Repository` with `isAccessible: false`; only transport errors reject (R2) |
| Toast infrastructure | Reuse `showToast`, `toasts`, `ToastStack.vue`; no new component (R1) |
| Summary source | `branchLabel` + `statusBadges` from `src/lib/repoStatus.ts` (R3) |
| Error message convention | First trimmed line, matching `summarizeResults` (R4) |
| DE/EN key completeness enforcement | `MessageKey` union derived from English catalogue (R5) |
| Test harness for the store | Mocked backend + `createPinia`/`setActivePinia` (R7) |
