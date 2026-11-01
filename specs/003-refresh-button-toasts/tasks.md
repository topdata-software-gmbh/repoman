---

description: "Task list for Refresh Button Feedback Toasts"
---

# Tasks: Refresh Button Feedback Toasts

**Input**: Design documents from `specs/003-refresh-button-toasts/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/store-refresh-contract.md, quickstart.md

**Tests**: Included — plan.md §Technical Context/Testing and the constitution's "Development Workflow & Quality Gates" require vitest coverage for new behavior including error paths; quickstart.md §Automated gates makes `pnpm test` and `pnpm typecheck` blocking. The spec itself does not request TDD, so tests are grouped at the end of each story phase rather than written first.

**Organization**: Tasks are grouped by user story — US1 (P1 success toast), US2 (P1 error toast), US3 (P2 background silence). The result-returning shared refresh is foundational because both toast stories and the background-silence story depend on its non-throwing contract.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no pending dependency)
- **[Story]**: US1 / US2 / US3
- Exact file paths are included in every implementation and test task

## Path Conventions

Single repository (plan.md §Project Structure): Vue/TypeScript frontend in `src/`, Rust service in `src-tauri/`. This feature touches the frontend only — no Rust file changes.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the baseline is green before touching the shared store.

- [X] T001 Verify baseline green before any change: from the repository root run `pnpm test` and `pnpm typecheck`; record the results so any later failure is attributable to this feature (constitution quality gates)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Make the shared refresh result-returning and non-throwing without changing its background/toast behavior. Every user story builds on this contract.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Add `export type RefreshResult = { ok: true; repo: Repository } | { ok: false; error: string }` to `src/stores/repoStore.ts` and change `refreshSingleRepo(path: string): Promise<RefreshResult>` (`src/stores/repoStore.ts:610`) to: resolve `{ ok: true, repo: updated }` when `updated.isAccessible`; for `isAccessible === false` update the row exactly as today and resolve `{ ok: false, error: t('status.folderNotFound') }` (FR-005); on a thrown transport error leave `repositories` unchanged and resolve `{ ok: false, error: String(error) }`; never reject (FR-008); emit **no** toast (FR-007); keep the existing update-in-place/append-and-sort-by-`path` behavior
- [X] T003 [P] Preserve every silent background caller after the signature change: confirm the watcher call in `src/App.vue:28` and `executeGroupAction`, `runRepoAction`, `commitRepositories`, `switchGroupBranch` in `src/stores/repoStore.ts` ignore the returned `RefreshResult`; make no toast additions and no behavior changes, adjusting only if the type checker complains (FR-007/FR-008, spec US3)
- [X] T004 [P] Create the store-test harness in `src/stores/__tests__/refreshRepoToast.test.ts`: `vi.mock('../../lib/backend', …)` with a mocked `backend.refreshRepository`, `setActivePinia(createPinia())` in `beforeEach`, `setLanguage` from `../../lib/i18n`, and a `makeRepo(overrides)` fixture whose shape matches `Repository` in `src/types/domain.ts` (accessible by default, `state.currentBranch = 'main'`, clean, no conflicts, no remotes); no assertions yet
- [X] T005 [P] Add the `toast.refreshDone` key in the same `toast.*` block of `src/lib/i18n/messages.en.ts` (~line 386) and `src/lib/i18n/messages.de.ts` (~line 386): EN `'{name} refreshed'`, DE `'{name} aktualisiert'` (contract §5, FR-002/FR-010)

**Checkpoint**: `refreshSingleRepo` returns `RefreshResult`, never throws, and emits no toast; the test harness and success-toast copy exist. Story implementation can begin.

---

## Phase 3: User Story 1 - See that a manual refresh succeeded (Priority: P1) 🎯 MVP

**Goal**: One click on a row's refresh button produces one success toast naming the project with its branch and status labels as the message (FR-001–FR-003, US1).

**Independent Test**: Call `store.refreshRepoWithToast(path)` with a mocked accessible repository and verify exactly one `success` toast whose title contains the project name and whose message is `main · Änderungen nicht gespeichert` (dirty) or `main · Auf dem neuesten Stand` (clean).

### Implementation for User Story 1

- [X] T006 [US1] Add `async function refreshRepoWithToast(path: string): Promise<void>` to `src/stores/repoStore.ts`: import `branchLabel`/`statusBadges` from `../lib/repoStatus`, call `refreshSingleRepo(path)` exactly once, and on `ok: true` call `showToast('success', t('toast.refreshDone', { name: repo.name || repoLabel(path) }), [branchLabel(repo), ...statusBadges(repo).map((badge) => badge.label)].join(' · '))`; on `ok: false` do nothing yet (US2 adds the error branch); never reject; add `refreshRepoWithToast` to the store's returned object alongside `refreshSingleRepo` (contract §3, research R3)
- [X] T007 [P] [US1] Change the per-row `refresh()` handler in `src/components/RepoListTable.vue:30-37` to `await store.refreshRepoWithToast(path)`; keep the existing `refreshing` single-flight guard, the `try/finally` spinner reset, and add no toast copy to the component (contract §4)
- [X] T008 [P] [US1] Add success-path tests to `src/stores/__tests__/refreshRepoToast.test.ts`: accessible dirty repo on `main` with `setLanguage('de')` yields exactly one `success` toast with title `{name} aktualisiert` and message `main · Änderungen nicht gespeichert`; clean repo yields `main · Auf dem neuesten Stand`; `setLanguage('en')` yields `{name} refreshed`; no-remote repo yields branch plus `status.localOnly`; a path absent from `repositories` is appended and still named via `repoLabel`; assert `store.toasts` length is 1 in each case (FR-001–FR-003, SC-001)
- [X] T009 [US1] Run `pnpm test` and `pnpm typecheck` from the repository root; US1 is done only when both pass

**Checkpoint**: A successful button-triggered refresh shows the correct localized success toast, and no other behavior changed.

---

## Phase 4: User Story 2 - See that a manual refresh failed (Priority: P1)

**Goal**: The same action emits one error toast naming the project and giving the failure reason for inaccessible projects and thrown transport errors (FR-004–FR-006, US2).

**Independent Test**: Mock `backend.refreshRepository` to resolve an inaccessible repository, then to reject with a multi-line error; verify each call produces one `error` toast with the project name in the title, the folder-not-found label for the first case, and the trimmed first error line for the second.

### Implementation for User Story 2

- [X] T010 [P] [US2] Add the `toast.refreshFailed` key in the same `toast.*` block of `src/lib/i18n/messages.en.ts` (~line 386) and `src/lib/i18n/messages.de.ts` (~line 386): EN `'Could not refresh {name}'`, DE `'{name} konnte nicht aktualisiert werden'` (contract §5, FR-004/FR-010)
- [X] T011 [US2] Extend the `ok: false` branch of `refreshRepoWithToast` in `src/stores/repoStore.ts`: resolve the display name exactly as the success branch, call `showToast('error', t('toast.refreshFailed', { name }), result.error.trim().split('\n')[0] || t('common.unknownError'))`, emit exactly one error toast, never reject, and never remove or invalidate the existing row (FR-004–FR-006, FR-009; research R4)
- [X] T012 [P] [US2] Add error-path tests to `src/stores/__tests__/refreshRepoToast.test.ts`: an inaccessible repository resolves `{ ok: false, error: t('status.folderNotFound') }` and yields one error toast with the folder-not-found message while the row stays in `store.repositories`; a rejected `refreshRepository` with `'fatal: boom\ndetails'` yields one error toast with message `'fatal: boom'`; a rejected call with `'   '` yields `t('common.unknownError')`; assert DE title `{name} konnte nicht aktualisiert werden` and EN title `Could not refresh {name}` and `store.toasts.length === 1`; assert `refreshRepoWithToast` resolves rather than rejects (FR-004–FR-006, FR-009, SC-001/SC-003)
- [X] T013 [US2] Run `pnpm test` and `pnpm typecheck` from the repository root; US2 is done only when both pass

**Checkpoint**: Success and failure clicks both produce exactly one localized toast; a failed refresh leaves row data intact.

---

## Phase 5: User Story 3 - Background refreshes stay quiet (Priority: P2)

**Goal**: Watcher events and the internal refresh step of pull/push/commit/branch-switch never create refresh toasts, and a background failure never rejects or drops a row (FR-007–FR-009, US3).

**Independent Test**: Call `store.refreshSingleRepo(path)` directly for success, inaccessible, and rejected cases; assert zero toasts, an unchanged `repositories` array on rejection, and that the promise resolves in every case.

### Implementation for User Story 3

- [X] T014 [P] [US3] Audit and, if needed, adjust the background paths so they remain toast-free: `src/App.vue:28` (`onRepoChanged`), `executeGroupAction`/`runRepoAction`/`commitRepositories`/`switchGroupBranch` in `src/stores/repoStore.ts`; `refreshSingleRepo` must stay toast-free, `Promise.all` calls must tolerate the result without surfacing it, and `summarizeResults` toast counts for pull/push/commit/branch-switch must be unchanged
- [X] T015 [P] [US3] Add background-silence tests to `src/stores/__tests__/refreshRepoToast.test.ts`: direct `refreshSingleRepo` for an accessible repo, an inaccessible repo, and a rejected call each leave `store.toasts.length === 0`; after a rejection `store.repositories` is unchanged and the promise resolves; assert no unhandled rejection (FR-007–FR-009, VR-005/VR-006, SC-002)
- [X] T016 [US3] Run `pnpm test` and `pnpm typecheck` from the repository root; US3 is done only when both pass

**Checkpoint**: Only the button path can produce a refresh toast; every background refresh stays silent and safe.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: User-visible documentation, full gates, and the end-to-end quickstart checks.

- [X] T017 [P] Add a `CHANGELOG.md` entry for the user-visible behavior (one success/error toast per manual refresh click, localized in German and English) per the constitution's changelog gate
- [X] T018 [P] Verify copy completeness and component cleanliness: `pnpm typecheck` proves `toast.refreshDone`/`toast.refreshFailed` exist in both catalogues via the `MessageKey` union; grep `src/components/RepoListTable.vue` and `src/stores/repoStore.ts` to confirm no hard-coded toast copy was introduced (FR-010, SC-004)
- [X] T019 Run the full blocking gates from the repository root: `pnpm test` and `pnpm typecheck`; confirm all quickstart store-level scenarios 1–4 pass and that no existing suite regressed
- [X] T020 Perform quickstart.md §Manual UI validation steps 2–5 on both the desktop and web frontends (accessible refresh, removed-folder error, German/English switch, and a pull plus a watcher event showing no refresh toast); record the outcome in the PR description

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: after T001 — BLOCKS all user stories (`RefreshResult`, non-throwing refresh, harness, and success copy)
- **US1 (Phase 3)**: after Phase 2 — MVP; creates `refreshRepoWithToast` and wires the button
- **US2 (Phase 4)**: after US1 — extends the same store action with its `ok: false` branch (shared file; see the shared-file constraint below)
- **US3 (Phase 5)**: after Phase 2 — independently testable; its tests live in the same test file as US1/US2
- **Polish (Phase 6)**: after all delivered stories

### User Story Dependencies

- **US1 (P1)**: needs only Phase 2 — no dependency on US2/US3
- **US2 (P1)**: needs Phase 2 plus the `refreshRepoWithToast` function introduced by US1; it extends that function's failure branch
- **US3 (P2)**: needs only Phase 2 (the non-throwing result contract); can be validated without US1/US2, though its tests append to the shared test file

### Within Each User Story

- i18n key → store branch → component wiring (US1 only) → tests → `pnpm test` + `pnpm typecheck` gate
- `refreshSingleRepo` stays the single source of row updates; toast text is built only in `refreshRepoWithToast`
- Do not let a failed refresh remove or mutate the existing row (FR-009)

### Parallel Opportunities

- T003, T004, T005 are mutually independent after T002 (different files)
- After T006/T005: T007 (component) and T008 (store tests) can run in parallel
- T010 (i18n) can start as soon as Phase 2 is done, but must land before T012
- T014 (background audit) and T015 (background tests) touch different files and can run in parallel after Phase 2
- Polish T017 (CHANGELOG) and T018 (copy/completeness verification) can run in parallel

### Shared-File Constraints (do NOT parallelize these pairs)

- `src/stores/repoStore.ts`: T002 → T006 → T011 → T014
- `src/stores/__tests__/refreshRepoToast.test.ts`: T004 → T008 → T012 → T015
- `src/lib/i18n/messages.en.ts` / `messages.de.ts`: T005 → T010

---

## Parallel Example: User Story 1

```bash
# after T006 (store action exists) and T005 (i18n keys exist) — disjoint files:
Task: "T007 [US1] wire RepoListTable.refresh() to refreshRepoWithToast in src/components/RepoListTable.vue"
Task: "T008 [US1] success-path store tests in src/stores/__tests__/refreshRepoToast.test.ts"
```

## Parallel Example: Foundational

```bash
# after T002 — all touch different files:
Task: "T003 [P] confirm background callers ignore RefreshResult in src/App.vue and src/stores/repoStore.ts"
Task: "T004 [P] create test harness in src/stores/__tests__/refreshRepoToast.test.ts"
Task: "T005 [P] add toast.refreshDone to src/lib/i18n/messages.en.ts and messages.de.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (T001)
2. Complete Phase 2 (T002–T005) — the result contract, silent background callers, harness, success copy
3. Complete Phase 3 (T006–T009)
4. **STOP and VALIDATE**: an accessible refresh click shows exactly one localized success toast; `pnpm test` + `pnpm typecheck` green
5. Deploy/demo if ready

### Incremental Delivery

1. Phase 1 + Phase 2 → shared contract frozen, background behavior unchanged
2. US1 → success feedback (MVP) → validate quickstart scenario 1
3. US2 → failure feedback → validate quickstart scenarios 2–3
4. US3 → silence regression guard → validate quickstart scenario 4
5. Polish → full gates, changelog, manual DE/EN + pull/watcher checks

### Parallel Team Strategy

This is a small frontend-only feature and the store/test files are shared, so parallel work is limited. One developer should own `src/stores/repoStore.ts` end-to-end (T002 → T006 → T011 → T014); a second developer can own i18n (T005 → T010) and a third the component/wiring plus test authoring (T007/T008/T012/T015) following the shared-file order above.

---

## Notes

- Literals repeated verbatim where they matter: success title `{name} aktualisiert` / `{name} refreshed`; failure title `{name} konnte nicht aktualisiert werden` / `Could not refresh {name}`; summary separator ` · `; inaccessible error message `status.folderNotFound`; empty-error fallback `common.unknownError`.
- `refreshSingleRepo` is called by the live watcher and by pull/push/commit/branch-switch; it must never toast and never throw (FR-007/FR-008).
- `refreshRepoWithToast` must produce exactly one toast per call; the component's `refreshing` guard is the only concurrency control and stays local to `RepoListTable.vue`.
- Error-message convention (first trimmed line) matches `summarizeResults` in `src/stores/repoStore.ts:521`.
- No Rust, transport, `ToastStack.vue`, or toast styling changes.
- Commit after each task or logical group; run the story gate before moving to the next priority.
