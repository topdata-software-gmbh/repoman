# Research: Personal Wording Preference — Plain Language vs Git Terminology

**Feature**: `005-plain-git-wording` | **Date**: 2026-11-19

All items below are decisions, not open questions. The architecture is already
fixed by
[ADR 261119-1](../../_ai/technical_decisions/ADR__261119-1__plain-language-base-with-git-wording-overlay.md);
this document records the concrete choices, the codebase evidence, and the
small deviations that follow from the spec's stronger per-surface requirements.

## R1 — Wording state and persistence

**Decision**: Add an app-owned `src/lib/i18n/wording.ts` module with
`Wording = 'plain' | 'git'`, `WORDINGS = ['plain', 'git']`,
`WORDING_STORAGE_KEY = 'topdata.wording.v1'`, a safe `localStorage` adapter, a
`plain` default, and a reactive `wording` ref plus `setWording()`.
Expose a factory so tests can inject storage; the app uses one module-level
instance, exactly as `src/lib/i18n/index.ts` already does for language.

**Rationale**: ADR 261119-1 chooses a separate axis persisted per device under
`topdata.wording.v1`, following the language key convention. The existing
`useLanguage` in the UI kit already establishes the safe-storage pattern
(`try/catch`, unsupported values ignored, dev warning), and the feature must not
modify the kit. A factory keeps invalid-value, unavailable-storage, and default
fallback testable without module mocking.

**Alternatives considered**:
- Reuse the kit's `useLanguage` — wrong semantics (it writes
  `topdata.language.v1` and sets `<html lang>`); a wording value is not a locale.
- Store it in Pinia — makes a pure i18n concern depend on store lifecycle and
  complicates use from `t()`; the i18n module is already the shared owner of
  language.
- Store it in `AppConfig` — shared project config, server-side schema work, and
  async writes for a per-person, per-browser preference (ADR rejected).
- Detect experience level automatically — surprising and untestable.

## R2 — Overlay shape and resolution

**Decision**: Add `src/lib/i18n/gitWording.ts` exporting
`Record<Language, Partial<Record<MessageKey, string>>>`. `t()` resolves
`overlay[language][key]` first only when the active wording is `git`, then falls
back to `catalogue[language][key]`, then to `en[key]`. A missing, non-string, or
blank overlay value is treated as missing.

**Rationale**: This is the ADR's partial-overlay decision: the full plain
catalogues remain the source of truth, the Git overlay is small and typed by
`MessageKey`, and `messages.de.ts`'s `Record<MessageKey, string>` guarantee is
untouched. Reading `wording.value` inside `t()` makes every template/computed
label update live in Vue 3, with no reload. The per-language maps make FR-010
(“missing Git variant shows the plain label”) a real code path rather than an
untested theoretical one.

**Alternatives considered**:
- Full `en-git`/`de-git` catalogues — duplicates every string four times and
  drifts on the next copy change (ADR rejected).
- Wording encoded in the locale — breaks the existing language axis and leaks
  wording into stored preferences (ADR rejected).
- Inline key variants (`action.push.git`) — pollutes the key space and weakens
  `MessageKey` parity (ADR rejected).
- A single language-agnostic overlay map — fewer lines today, but erases the
  fallback dimension the spec explicitly requires and makes future
  language-specific exceptions a type change.

## R3 — Overlay scope and concrete key inventory

**Decision**: Override short labels on every surface where the operation is
named, plus the status badges and filters that name operation state:

| Group | Keys |
|---|---|
| Toolbar verbs | `toolbar.saveToGit`, `toolbar.getLatest`, `toolbar.sendChanges`, `toolbar.checkForUpdates`, `toolbar.setAside`, `toolbar.bringBack` |
| Table verbs | `table.save`, `table.getLatestFor`, `table.sendFor` |
| Status badges | `status.changesNotSaved`, `status.updatesAvailable`, `status.toSend` |
| Matching filters | `filters.changesNotSaved`, `filters.updatesAvailable`, `filters.toSend` |
| Save dialog | `save.title`, `save.submit`, `save.savedTitle` |
| Glossary names | `help.word.save.name`, `help.word.pull.name`, `help.word.push.name`, `help.word.stash.name` |
| Onboarding headings | `onboarding.ready.saveTitle`, `onboarding.ready.pullTitle`, `onboarding.ready.pushTitle` |
| Action labels | `action.fetch`, `action.pull`, `action.push`, `action.stash`, `action.stashPop` |
| Insight actions | `insight.dirty.actionOne`, `insight.dirty.actionOther` |
| Operation toasts | `toast.fetchDone/Failed/Failure`, `toast.pullDone/Failed/Failure`, `toast.pushDone/Failed/Failure`, `toast.stashDone/Failed/Failure`, `toast.stashPopDone/Failed/Failure` |

That is 47 keys per language. The ADR's “~24 keys” estimate covered the core
verb/glossary set; the spec's FR-007 (“every surface, including confirmation or
failure message”) and FR-013 (status badges) add the toast titles, per-row
accessibility labels, filter mirrors, onboarding headings and insight actions.
The architecture is unchanged; only the overlay table is larger.

**Deliberately not overlaid** (they remain plain in both modes):
- Every `*Hint`, `*description`, `*Text`, error and validation key; the
  “What Git did”/Git-log drawer; raw Git output.
- `help.step3.title`/`help.step4.title` and `help.description`, because they are
  explanatory sentences, not labels.
- File-status vocabulary (`save.status.*`, `review.status.*`) and AI draft
  statuses, which are not operation names.
- `group.saveChanges` and `toast.groupSaved`, because saving a group is not the
  Git commit operation.
- `status.notOnBranch`, `status.localOnly`, `status.upToDate`,
  `status.needsAttention`, and merge/rebase/cherry-pick labels: no short
  operation-name change is needed, and the existing terms are already clear.

**Rationale**: FR-007 requires one consistent term on all surfaces where a
concept is named; overriding only the toolbar would make a button and its toast
disagree. FR-008 keeps explanations untouched. FR-013 keeps short status badges
in the selected register but does not turn them into sentences.

**Alternatives considered**:
- Toolbar-only overlay — violates FR-007 and SC-002.
- Overlay hints and explanatory copy — violates FR-008 and is what the ADR
  explicitly avoids.
- Translate toast titles as full English sentences in both languages — mixes
  full-language copy into a terminology axis and ignores the German plain
  catalogue; the overlay uses short Git terms instead.

## R4 — Canonical Git terms and display forms

**Decision**: Use canonical English Git words in every language, as required by
the spec clarification. Labels use Title Case; hint suffixes use lowercase
command spelling.

| Concept | Label | Hint suffix | Status/filter form |
|---|---|---|---|
| Commit | `Commit` | `(git commit)` | `Uncommitted changes` |
| Fetch | `Fetch` | `(git fetch)` | — |
| Pull | `Pull` | `(git pull)` | `{count} behind` / `Behind` |
| Push | `Push` | `(git push)` | `{count} ahead` / `Ahead` |
| Stash | `Stash` | `(git stash)` | — |
| Stash pop | `Stash pop` | `(git stash pop)` | — |

`save.savedTitle` uses `Committed`; `onboarding.ready.saveTitle` and
`help.word.save.name` use `Commit`. Toast forms use `<Term> complete`,
`<Term> failed`, and the bare `<Term>` for failure summaries.

**Rationale**: The clarification says canonical English Git words in both
languages, while German plain labels stay German. Short forms keep controls
compact; the hint suffix is lower-case because it names a command
(`git commit`), not a label. “Stash pop” maps directly to `git stash pop`;
“Unstash” is common speech but not the command word.

**Alternatives considered**:
- German-translated jargon (`Übernehmen`, `Abholen`) — directly contradicts the
  clarification.
- `Check for updates`/`Get latest`/`Send changes` in Git mode — defeats the
  purpose for the Git audience.
- `Unstash` — not the canonical command and would not match “What Git did”.
- Full `git commit` as the button label — too long and not how Git users name
  the operation.

## R5 — Plain-mode hint bridge

**Decision**: Add `operationHint(key: MessageKey, term: GitTerm): string`.
It returns `t(key)` in Git mode and `` `${t(key)} (git ${term})` `` in plain
mode. Apply it to the operation controls' hints only:
`toolbar.saveHint`, `toolbar.getLatestHint`, `toolbar.sendHint`,
`toolbar.checkForUpdatesHint`, `toolbar.setAsideHint`,
`toolbar.bringBackHint`, `table.saveHint`, `table.getLatestForHint`, and
`table.sendForHint`.

**Rationale**: FR-005 requires plain labels' hints to also name the Git term;
ADR 261119-1 specifies the exact suffix shape. Keeping hint text out of the
overlay means Git mode never rewords an explanation (FR-008). A `GitTerm` union
prevents typos and keeps the suffix aligned with the overlay values.

**Alternatives considered**:
- Add a second set of “plain+Git” hint keys — doubles copy and creates a third
  source of truth.
- Overlay hint values to include the suffix only in plain mode — impossible with
  a wording overlay (it activates in Git mode, not plain mode).
- Append the term to every description and onboarding text — violates FR-008.

## R6 — Accessible reveal of the hint

**Decision**: Add an app-owned `OperationHint.vue` using the kit's exported
`usePopover`. It wraps a single control, exposes slot props for
`aria-describedby` and `title`, keeps an always-present visually hidden
description element for assistive technology, and teleports a separate
`aria-hidden` visual tooltip that is shown on mouse enter and
`focusin`/`focus-within`, hidden on mouse leave, `focusout`, or Escape. Use it
for toolbar operation buttons and table row operation buttons. Menu items use
their existing always-visible description slot, which now carries the same
suffixed hint, so opening the menu with the keyboard reveals it without a
second tooltip.

**Rationale**: The spec clarification rejects hover-only. Native `title` is not
reliably exposed on keyboard focus and is weak for assistive technology, and
the kit has no lightweight tooltip component (its `TdHint` is a dialog-like
panel with actions). `usePopover` is already exported, handles collision and
overflow clipping, and keeps the helper app-owned, so the kit remains
unmodified (ADR decision 7). `aria-describedby` points at the tooltip text so
screen readers announce the same explanation plus Git term.

**Alternatives considered**:
- `title` only — fails keyboard-focus and assistive-technology requirements.
- `aria-label` only — announces the term but does not reveal it visibly on
  focus.
- `TdHint` — designed as a dismissible guidance dialog, not a hover/focus hint;
  would add buttons and focus management the feature does not need.
- Inline hint text under each control — changes toolbar/table layout and
  duplicates copy.
- A single global hint region — adds cross-component state and is harder to
  associate with the focused control via `aria-describedby`.

## R7 — Wording control

**Decision**: Add `WordingSwitcher.vue` as a compact sibling of
`TdLanguageSwitcher` in the sidebar footer, between the language switcher and
the theme toggle. Use `TdMenu` + `TdButton` with a short trigger label
(“Plain”/“Git” in English, “Einfach”/“Git” in German). The trigger's
`aria-label` and `title` include the localized “Change wording” label and the
active option. The menu has two options with full labels, a visible check for
the active one, and a visually hidden localized “current” word. Selecting calls
`setWording()`; no settings screen or new navigation is added.

**Rationale**: FR-012 requires the control to be discoverable next to the
language choice, labelled, keyboard-operable, and to expose its current state.
A menu matches the existing language switcher's interaction pattern and stays
narrow in the footer. The footer already contains four compact controls, so a
full segmented control or native select would be too wide.

**Alternatives considered**:
- Put the switch in the Help modal — the glossary explains terms; it is not
  where a preference is discovered (ADR rejected).
- A full segmented control — clearer, but too wide for the footer.
- A native `TdSelect` — visually heavy and inconsistent with the language
  switcher.
- Hide it in a new settings screen — violates the “next to language” requirement
  and adds scope.

## R8 — Reactivity, independence and live updates

**Decision**: `t()` reads the reactive `wording` ref, so all template and
computed labels re-render immediately when `setWording()` changes it. Store
operation toasts are built when the operation finishes, so a running operation
that completes after the switch uses the new wording. The save dialog title and
submit label use `t()` in the template and update live. Language and wording
never write each other's keys; `setLanguage()` and `setWording()` are
independent.

**Rationale**: FR-009 and the spec edge cases require no reload and no mixed
leftovers after switching language and wording in quick succession. Because the
plain catalogue and the per-language Git overlay are resolved at render time,
every control, badge, dialog title and glossary name follows the final pair.
Already-visible transient toasts keep the text emitted when the operation
finished, matching the existing language behavior; the next operation uses the
new wording.

**Alternatives considered**:
- Copy labels into component state on mount — would not update live and would
  create a second source of truth.
- Reload the app on switch — violates FR-009.
- Store localized strings in Pinia — duplicates `t()` and breaks reactivity.
- Make already-visible toast text reactive — requires changing the toast
  contract to carry keys/params and is not needed for the label surfaces the
  spec enumerates; the existing language behavior is the precedent.

## R9 — Testing strategy

**Decision**: Add/extend Vitest tests at the i18n and component boundaries:

1. `wording.test.ts`:
   - default is plain with no stored value;
   - `setWording('git')` writes `topdata.wording.v1` and updates the ref;
   - an invalid stored value falls back to plain;
   - unavailable/throwing storage does not break startup;
   - `t()` returns overlay values in Git mode and plain values otherwise;
   - a missing or blank overlay entry falls back to the plain catalogue;
   - overlay key sets are identical in `en` and `de`, every overlay key exists
     in both catalogues, and every value is non-empty;
   - language and wording combinations resolve independently;
   - keys outside the overlay (for example `table.review`,
     `status.folderNotFound`, `toolbar.saveHint`, `gitLog.title`) render
     identically in both modes;
   - `operationHint()` appends `(git …)` only in plain mode.
2. `wording-switcher.test.ts`: trigger accessible name includes the current
   state; selecting the other option calls `setWording`; a label rendered from
   `t()` updates without remounting.
3. `wording-hint.test.ts` (or a focused Toolbar/RepoListTable mount): the
   control's `aria-describedby` points at the tooltip and the tooltip contains
   the Git suffix in plain mode; the suffix is absent in Git mode.
4. Extend `repoStatus.test.ts`: badges show `Uncommitted changes`, `2 behind`,
   `2 ahead` in Git mode and the existing plain labels after switching back.
5. Extend `i18n.test.ts` for the new catalogue keys and the overlay parity
   guard, keeping the existing DE/EN key-parity assertion.

**Rationale**: The project already uses Vitest with `happy-dom`, `vi.mock` and
Pinia setup in `src/stores/__tests__`, and the existing `i18n.test.ts` already
guards catalogue parity. Testing resolution and fallback directly is cheaper
and more stable than snapshotting every component; focused component tests cover
the accessibility wiring that cannot be proven by a string helper.

**Alternatives considered**:
- One end-to-end snapshot of every screen in both modes — brittle and noisy.
- Only manual testing — misses fallback, parity and a11y regressions.
- Testing through the Rust server — the feature is frontend-only and the
  behavior is identical in both transports.

## R10 — Documentation, changelog and scope boundary

**Decision**: Link ADR 261119-1 from the plan; do not create a new ADR. Update
`CHANGELOG.md` in the same change because user-visible copy and a new preference
are added. No README or configuration-schema change is required. No Rust,
Tauri command, axum route, `AppConfig`, migration, or UI-kit file is touched.

**Rationale**: Constitution IV requires same-change copy and the governance
section requires an ADR for decisions that constrain work beyond one feature;
that ADR already exists. The change deliberately stops at the frontend because
the preference is per device/browser and never shared through project
configuration.

**Alternatives considered**:
- A new ADR per implementation detail — duplicates the accepted decision.
- Server-side persistence — out of scope and contradicts FR-003.
- Translating the Git log/raw output — explicitly out of scope (FR-008, FR-014).

## Resolved Unknowns

| Technical Context item | Resolution |
|---|---|
| Persistence mechanism/key | `localStorage`, `topdata.wording.v1`, default `plain` (R1) |
| Overlay structure | Per-language `Partial<Record<MessageKey, string>>`, fallback to plain (R2) |
| Which labels move | 47 short-label keys across controls, toasts, badges, dialogs, glossary, onboarding, insights (R3) |
| Exact Git terms | Canonical English: Commit/Fetch/Pull/Push/Stash/Stash pop; Uncommitted changes/Behind/Ahead (R4) |
| Hint bridge | `operationHint()` appends `(git …)` only in plain mode (R5) |
| Keyboard/AT reveal | `OperationHint` tooltip on hover/focus + `aria-describedby`; menu descriptions for menu items (R6) |
| Control placement | Sibling of `TdLanguageSwitcher` in the sidebar footer (R7) |
| Live update | Reactive `t()`; store toasts resolve at operation completion (R8) |
| Backend impact | None: no Rust, config, transport, or Git-operation change (R10) |
