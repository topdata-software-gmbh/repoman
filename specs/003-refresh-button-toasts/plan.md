# Implementation Plan: Refresh Button Feedback Toasts

**Branch**: `003-refresh-button-toasts` | **Date**: 2026-11-01 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/003-refresh-button-toasts/spec.md`
(derived from design handoff `_ai/backlog/active/261101_0159__DESIGN_HANDOFF__refresh-button-toasts.md`)

## Summary

Give the per-row refresh button visible feedback: one success or error toast per
accepted click. The store's shared silent refresh becomes result-returning (it
still never throws and never toasts), and a new opt-in store action wraps it to
build the toast from existing branch/status label helpers. Background callers
(file watcher, post-action refreshes) keep the silent path, so pull/push/commit
toasts and watcher events are unchanged. All copy lives in the existing DE/EN
dictionaries.

## Technical Context

**Language/Version**: TypeScript 5.5, Vue 3.4 (`<script setup>`), Pinia 2.2

**Primary Dependencies**: `@topdata-software-gmbh/ui-kit-web-vue3` (toast stack,
buttons, badges), `lucide-vue-next` icons, existing `backend` client. No new
dependency.

**Storage**: N/A — toast and repository state are in-memory; the existing
`localStorage` repository cache is untouched.

**Testing**: Vitest 2.1 + `@vue/test-utils` + `happy-dom` (`pnpm test`), static
type check via `vue-tsc --noEmit` (`pnpm typecheck`). Store tests use
`createPinia`/`setActivePinia` and a mocked `../lib/backend`.

**Target Platform**: Desktop (Tauri v2) and browser frontend; both share the
same Vue store, so behavior must be identical in both.

**Project Type**: Single repository — Vue frontend in `src/`, Rust service in
`src-tauri/`. This feature touches the frontend only.

**Performance Goals**: No measurable impact; one toast object per click, no new
backend calls (the refresh call already exists).

**Constraints**: No backend/Rust change; no change to pull/push/commit/switch or
file-watcher toast behavior; shared refresh must remain non-throwing; refresh
button's existing concurrency guard must remain effective; all copy in both DE
and EN.

**Scale/Scope**: One row action in `RepoListTable.vue`, two store actions, two
i18n keys per language, one new type, store tests. No group/global refresh.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Spec-Driven Change** — PASS: `specs/003-refresh-button-toasts/spec.md`
  exists and this plan derives from it before code work.
- **II. One Core, Two Frontends** — PASS: no Rust or transport change; the
  frontend uses the already-declared `refresh_repository` command through the
  typed `backend` client. No ad-hoc payloads or filesystem access added.
- **III. Git Only Through the System CLI** — PASS: no Git operation added or
  changed; refresh remains a read-only observation.
- **IV. Beginner-Friendly by Default** — PASS: both DE and EN keys added in the
  same change; title/message reuse existing translated labels; no literals in
  components.
- **V. Bounded, Non-Destructive Action** — PASS: refresh only observes; the
  existing per-row `refreshing` guard prevents concurrent clicks; shared refresh
  never throws and keeps existing row data on failure.

**Result**: No violations, no complexity-tracking entries required.

## Project Structure

### Documentation (this feature)

```text
specs/003-refresh-button-toasts/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── store-refresh-contract.md
└── tasks.md             # Phase 2 output (/speckit.tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── stores/
│   └── repoStore.ts                 # RefreshResult, refreshSingleRepo, refreshRepoWithToast
├── components/
│   └── RepoListTable.vue            # Button calls refreshRepoWithToast
├── lib/
│   ├── repoStatus.ts                # Existing branchLabel/statusBadges (unchanged)
│   └── i18n/
│       ├── messages.de.ts           # toast.refreshDone / toast.refreshFailed
│       └── messages.en.ts           # same keys in English
└── stores/__tests__/
    └── refreshRepoToast.test.ts     # New store-level behavior tests
```

**Structure Decision**: The frontend single-project layout is used. Business
logic stays in `src/stores/repoStore.ts`; the component only calls the new
action. No Rust files change.

## Phase 0: Research

See [research.md](./research.md). All decisions are recorded there; there are no
unresolved `NEEDS CLARIFICATION` items.

## Phase 1: Design & Contracts

- Entity/result shapes: [data-model.md](./data-model.md)
- Store action contract: [contracts/store-refresh-contract.md](./contracts/store-refresh-contract.md)
- End-to-end validation: [quickstart.md](./quickstart.md)

### Implementation outline

1. Add `RefreshResult` type and change `refreshSingleRepo(path)` to return it
   (`{ ok: true, repo }` for an accessible repo, `{ ok: false, error }` for an
   inaccessible repo or a thrown transport error), keeping the existing
   array-update behavior and never throwing.
2. Add `refreshRepoWithToast(path)`: calls `refreshSingleRepo`, then on success
   shows a success toast whose title is `t('toast.refreshDone', { name })` and
   whose message joins `branchLabel(repo)` with each `statusBadges(repo)` label
   using ` · `; on failure shows an error toast titled
   `t('toast.refreshFailed', { name })` whose message is the existing
   folder-not-found label for an inaccessible repo or the trimmed first line of
   the raw error (falling back to `t('common.unknownError')`).
3. Export `refreshRepoWithToast` from the store and switch
   `RepoListTable.vue`'s `refresh()` to call it.
4. Add `toast.refreshDone` and `toast.refreshFailed` to both dictionaries.
5. Add store tests for success content, inaccessible result, thrown error, and
   background refresh silence.

## Post-Design Constitution Re-check

- **I** — PASS: plan and artifacts are derived from the accepted spec.
- **II** — PASS: only the frontend store/component change; transport contract is
  reused unchanged.
- **III** — PASS: no Git invocation change.
- **IV** — PASS: DE/EN keys are part of the planned change; labels are reused.
- **V** — PASS: read-only operation, guard retained, failure keeps row data.

**Result**: No new violations.

## Complexity Tracking

> No constitutional violations; table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|------------------------------------|
| — | — | — |
