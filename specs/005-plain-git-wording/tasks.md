---

description: "Task list for feature implementation"
---

# Tasks: Personal Wording Preference — Plain Language vs Git Terminology

**Input**: Design documents from `/specs/005-plain-git-wording/`

**Prerequisites**: plan.md (required), spec.md (required for user stories), research.md, data-model.md, contracts/

**Tests**: Included because the plan, `research.md` (R9) and `quickstart.md` explicitly require Vitest coverage for storage, fallback, parity, hints, badges and the control. Write each test task before its implementation task and confirm it fails first.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **Single project (this feature)**: Vue frontend under `src/`, no Rust changes.
- i18n modules live in `src/lib/i18n/`, app-owned components in `src/components/`, library helpers in `src/lib/`.
- Tests live in `src/lib/__tests__/` and `src/components/__tests__/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the baseline is green before touching the frontend.

- [X] T001 Confirm baseline `pnpm test` and `pnpm typecheck` pass using the scripts defined in `package.json`; record any pre-existing failures so they are not mistaken for regressions.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core wording state that every story reads. No storage yet — persistence is added in US3.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T002 Create `src/lib/i18n/wording.ts` with `Wording = 'plain' | 'git'`, `WORDINGS = ['plain', 'git'] as const`, `WORDING_STORAGE_KEY = 'topdata.wording.v1'`, a module-level reactive `wording` ref defaulting to `plain`, and `setWording()` that accepts only `plain` or `git`, ignores unsupported values with a development-only warning, and never throws.

**Checkpoint**: Foundation ready — US1, US2 and US3 can start in priority order.

---

## Phase 3: User Story 1 - Keep plain words, still see the real Git term (Priority: P1) 🎯 MVP

**Goal**: Plain remains the default, and every operation hint reveals the matching Git command on hover **and** keyboard focus, with an accessible description.

**Independent Test**: With no stored preference, all operation labels are plain; hovering or focusing a toolbar/table operation control shows the existing explanation plus `(git commit|fetch|pull|push|stash|stash pop)`, and `aria-describedby` points at the same hint.

### Tests for User Story 1 (OPTIONAL - only if tests requested) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T003 [P] [US1] Write failing `operationHint()` tests in `src/lib/__tests__/wording.test.ts`: plain mode appends ` (git commit)`, ` (git fetch)`, ` (git pull)`, ` (git push)`, ` (git stash)` and ` (git stash pop)` to the unchanged base hint; Git mode returns the base hint unchanged; the hint key itself is never overlaid.
- [X] T004 [P] [US1] Write failing component tests in `src/components/__tests__/operation-hint.test.ts` for `OperationHint.vue`: the wrapped control receives an `aria-describedby` that resolves to an always-present hidden description; the tooltip text includes the Git suffix in plain mode and omits it in Git mode; the tooltip opens on `focusin` and closes on Escape; two instances use unique description ids.

### Implementation for User Story 1

- [X] T005 [US1] Add the `GitTerm` union (`commit`, `fetch`, `pull`, `push`, `stash`, `stash pop`) and `operationHint(key: MessageKey, term: GitTerm): string` to `src/lib/i18n/index.ts` per `specs/005-plain-git-wording/contracts/i18n-wording-contract.md` §3: return `t(key)` in Git mode and `` `${t(key)} (git ${term})` `` in plain mode.
- [X] T006 [US1] Create `src/components/OperationHint.vue` using the kit's exported `usePopover`: accept a single default-slot control, expose `describedBy` and `title` slot props, render a visually hidden description element with that id, and teleport an `aria-hidden` visual tooltip shown on pointer enter and `focusin`/`focus-within` and hidden on leave, `focusout`, or Escape.
- [X] T007 [US1] Wire `OperationHint` around the toolbar Save, Get latest and Send changes buttons and replace `t()` with `operationHint()` for the Fetch, Stash and Bring back `#description` slots in `src/components/Toolbar.vue` (hint keys and terms per contract §7).
- [X] T008 [P] [US1] Wire `OperationHint` around the table row Save, Get latest and Send changes buttons in `src/components/RepoListTable.vue` using `table.saveHint`/`commit`, `table.getLatestForHint`/`pull` and `table.sendForHint`/`push`, preserving the existing `aria-label` text and disabled behavior.

**Checkpoint**: User Story 1 is fully functional and testable independently — plain labels plus keyboard/AT-reachable Git hints, with no wording switch required.

---

## Phase 4: User Story 2 - A Git power user switches to real terminology (Priority: P2)

**Goal**: Selecting Git terminology replaces every short operation label, dialog title, badge, filter, toast title, glossary name, onboarding heading and insight action immediately, without a reload.

**Independent Test**: Open the wording menu and select Git terminology; verify toolbar/table labels, save dialog title and submit, save toast title, operation toast titles, badges/filters, glossary names, onboarding headings and insight action labels all use the same canonical Git terms, and switching back restores plain exactly.

### Tests for User Story 2 (OPTIONAL - only if tests requested) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T009 [P] [US2] Write failing overlay tests in `src/lib/__tests__/i18n.test.ts`: `gitWording.en` and `gitWording.de` have identical key sets; every overlay key exists in both plain catalogues; every overlay value is a non-empty string; `t()` returns the overlay in Git mode and the plain catalogue in plain mode; a missing or blank overlay value falls back to the plain label; German plain and German Git combinations resolve independently.
- [X] T010 [P] [US2] Write failing badge/filter tests in `src/lib/__tests__/repoStatus.test.ts` and `src/lib/__tests__/repoFilters.test.ts`: Git mode yields `Uncommitted changes`, `{count} behind`, `{count} ahead` and the filters `Uncommitted changes`, `Behind`, `Ahead`; switching back to plain restores the existing labels.
- [X] T011 [P] [US2] Write failing `src/components/__tests__/wording-switcher.test.ts` tests: the trigger's accessible name includes the localized “Change wording” label and the active option; opening the menu shows exactly “Plain language” and “Git terminology”; selecting Git calls `setWording('git')`; a label rendered from `t()` updates without remounting.

### Implementation for User Story 2

- [X] T012 [P] [US2] Create `src/lib/i18n/gitWording.ts` exporting `GitWording = Partial<Record<MessageKey, string>>` and `gitWording: Record<Language, GitWording>`, populated with exactly the 47 keys and canonical values in `specs/005-plain-git-wording/contracts/i18n-wording-contract.md` §4, identical in `en` and `de`.
- [X] T013 [US2] Update `t()` in `src/lib/i18n/index.ts` so Git mode resolves `gitWording[language][key]` when it is a non-blank string, then the active-language plain catalogue, then `en`; plain mode keeps the existing catalogue-then-`en` order; interpolate parameters only after the template is selected.
- [X] T014 [P] [US2] Add the new control copy keys to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts` (same keys in both): `sidebar.changeWording`, `wording.plain`, `wording.git`, `wording.plainShort`, `wording.gitShort`, `wording.current`. The Git overlay itself stays in `gitWording.ts`.
- [X] T015 [US2] Create `src/components/WordingSwitcher.vue` per contract §5 using `TdMenu` + `TdButton`: compact trigger showing the active short label, `aria-label`/`title` = localized “Change wording: <active full label>”, `aria-haspopup="menu"` and `aria-expanded`; two menu options with a visible check and visually hidden “current” word on the active one; selecting calls `setWording()` and closes the menu without reload or touching the language key.
- [X] T016 [US2] Mount `WordingSwitcher` in `src/components/Sidebar.vue` between `TdLanguageSwitcher` and the theme toggle inside `TdSidebarFooter`, passing no language state and importing only the wording control.
- [X] T017 [US2] Audit every overlaid surface to ensure it renders through `t()` and picks up the overlay automatically: `src/components/SaveChangesModal.vue` (`save.title`, `save.submit`, `save.savedTitle`), `src/components/HelpModal.vue` (the four glossary names), `src/components/onboarding/OnboardingStepReady.vue` (three headings), and `src/stores/repoStore.ts` (`action.*`, `toast.*`, `insight.dirty.action*`); replace any literal or copied label found.

**Checkpoint**: User Stories 1 and 2 both work independently; Git mode updates live and preserves plain explanations.

---

## Phase 5: User Story 3 - The preference is personal, persistent and language-independent (Priority: P3)

**Goal**: The wording choice survives restarts, falls back to plain for invalid/unavailable storage, and combines freely with German/English without sharing state through projects or config.

**Independent Test**: Set Git wording, recreate the module from the same storage and confirm Git; set an invalid stored value and confirm plain; create two factories with separate storage maps and confirm they do not share a value; switch language and confirm the wording key is unchanged.

### Tests for User Story 3 (OPTIONAL - only if tests requested) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T018 [US3] Write failing persistence tests in `src/lib/__tests__/wording.test.ts` using an injected in-memory storage: no stored value resolves to `plain`; `setWording('git')` writes exactly `topdata.wording.v1 = 'git'`; `setWording('plain')` writes `'plain'`; a stored invalid value resolves to `plain`; a throwing/disabled storage does not throw; an unsupported value passed to `setWording` is ignored and does not overwrite storage; two factory instances with separate storage maps stay isolated; changing language never writes `topdata.wording.v1` and changing wording never writes `topdata.language.v1`.

### Implementation for User Story 3

- [X] T019 [US3] Extend `src/lib/i18n/wording.ts` with the `WordingStorage`, `UseWordingOptions` and `UseWording` interfaces and a `useWording()` factory that reads the initial value through a safe `localStorage` adapter, ignores unsupported values, swallows read/write failures, defaults to `plain`, and writes only `topdata.wording.v1` on `setWording()`; keep the module singleton `wording`/`setWording` exported and used by the app.
- [X] T020 [US3] Verify in `src/lib/i18n/wording.ts` and `src/lib/i18n/index.ts` that startup reads the stored preference before the first `t()` call, that invalid stored values never leave an empty label, and that the language and wording setters only touch their own keys; fix any ordering or cross-key write found.

**Checkpoint**: All three user stories work independently and together; the preference is device-local and restart-safe.

---

## Phase 6: Polish & Cross-Cutting Concerns

**Purpose**: Evidence, documentation and scope guards that span all stories.

- [X] T021 [P] Update `CHANGELOG.md` under `## [Unreleased]` / `### Added` with the new plain/Git wording preference, the `(git …)` hint bridge and its accessible reveal.
- [X] T022 Run `pnpm typecheck` and `pnpm test` from the repository root and fix every failure in `src/lib/i18n/`, `src/lib/__tests__/` and `src/components/`; confirm the full suite is green.
- [X] T023 Run the `quickstart.md` scenarios 1–8 through `./scripts/dev-up.sh` in a fresh browser profile: confirm all four language/wording combinations, live switching with an open dialog, persistence after reload, invalid stored value fallback, keyboard-only control use, and the plain explanations remaining unchanged in Git mode.
- [X] T024 [P] Scope guard: run `git diff --stat` and confirm no changes under `src-tauri/`, no changes to `AppConfig`, axum routes, typed events, `src/lib/backend.ts`, or the UI-kit package; confirm Git output, commit content, branch names and file names are untouched and revert any accidental edit.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — can start immediately.
- **Foundational (Phase 2)**: Depends on Setup — BLOCKS all user stories.
- **User Stories (Phase 3+)**: All depend on Foundational completion.
  - US1 (P1) is the MVP and should be completed first.
  - US2 (P2) adds the overlay and control and can proceed after US1 or in parallel once the Phase 2 module exists.
  - US3 (P3) hardens persistence and independence and can proceed after Phase 2; it does not depend on US1/US2 code, but its tests should run against the final overlay.
- **Polish (Phase 6)**: Depends on all desired user stories being complete.

### User Story Dependencies

- **US1 (P1)**: After Phase 2. No dependency on US2/US3. Delivers plain labels plus accessible Git hints.
- **US2 (P2)**: After Phase 2. Uses the same `wording` ref but does not require US1's hint component; independently demonstrable by switching labels.
- **US3 (P3)**: After Phase 2. Uses only `src/lib/i18n/wording.ts` and storage; independently demonstrable by recreating state and changing language.

### Within Each User Story

- Tests first and failing before implementation (T003/T004, T009/T010/T011, T018).
- State/helpers before components; components before integration.
- Each story's checkpoint is independently validatable.

### Parallel Opportunities

- T003 and T004 touch different test files.
- After T005 lands, T006 (component) and T008 (table wiring) can proceed in parallel; T007 depends on T006's slot contract.
- T009, T010 and T011 touch different test files.
- T012 (overlay file), T014 (catalogues) can be written in parallel; T013 depends on T012.
- T015 depends on T014; T016 depends on T015.
- T018 is independent of T019's final shape but should be written first.
- T021 and T024 touch unrelated files and can run in parallel near the end.

---

## Parallel Example: User Story 1

```bash
# Write the helper and component tests first, in parallel:
Task: "Write failing operationHint tests in src/lib/__tests__/wording.test.ts"
Task: "Write failing OperationHint accessibility tests in src/components/__tests__/operation-hint.test.ts"

# After T005 creates operationHint and T006 creates the component:
Task: "Wire OperationHint into src/components/Toolbar.vue"
Task: "Wire OperationHint into src/components/RepoListTable.vue"
```

## Parallel Example: User Story 2

```bash
# Write the three test suites in parallel:
Task: "Extend src/lib/__tests__/i18n.test.ts with overlay parity and fallback"
Task: "Extend src/lib/__tests__/repoStatus.test.ts and repoFilters.test.ts with Git badges/filters"
Task: "Write src/components/__tests__/wording-switcher.test.ts"

# Build the independent files in parallel:
Task: "Create src/lib/i18n/gitWording.ts with the 47-key overlay"
Task: "Add the six control copy keys to messages.en.ts and messages.de.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: baseline green.
2. Complete Phase 2: `src/lib/i18n/wording.ts` state and default.
3. Complete Phase 3: `operationHint()` plus `OperationHint.vue` and the toolbar/table wiring.
4. **STOP and VALIDATE**: Test US1 independently — plain labels, hover/focus hint with `(git …)`, `aria-describedby`, Escape.
5. Deploy/demo if ready: this is the lowest-risk increment and preserves today's beginner-first UI.

### Incremental Delivery

1. Setup + Foundational → shared wording state ready.
2. US1 → plain labels with a Git term reveal → validate → demo (MVP).
3. US2 → typed overlay, wording control and live Git labels → validate all surfaces → demo.
4. US3 → safe persistence, invalid-value fallback and independence → validate restart/language switching.
5. Polish → changelog, full typecheck/test, quickstart pass, scope guard.

### Parallel Team Strategy

1. One developer completes Setup + Foundational.
2. After Phase 2:
   - Developer A: US1 (`operationHint`, `OperationHint.vue`, Toolbar/RepoListTable wiring).
   - Developer B: US2 (`gitWording.ts`, `t()` resolution, catalogues, `WordingSwitcher.vue`, Sidebar).
   - Developer C: US3 (`wording.ts` storage/factory and isolation tests).
3. Merge at the Phase 6 gates; resolve any `src/lib/i18n/index.ts` overlap between T005 and T013 by landing US1's helper before US2's resolution change, or by rebasing.

---

## Notes

- [P] tasks touch different files and have no dependency on incomplete tasks.
- [Story] labels map every story-phase task to its spec story for traceability.
- Tests are required by this feature's plan/research/quickstart; verify they fail before implementing.
- Keep plain catalogues as the source of truth; never add explanatory copy to `gitWording.ts`.
- Do not modify `src-tauri/`, `AppConfig`, routes/events, `src/lib/backend.ts`, or the UI kit.
- Commit after each task or logical group and stop at each checkpoint to validate the story independently.
