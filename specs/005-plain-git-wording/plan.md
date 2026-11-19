# Implementation Plan: Personal Wording Preference — Plain Language vs Git Terminology

**Branch**: `005-plain-git-wording` | **Date**: 2026-11-19 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/005-plain-git-wording/spec.md`
(derived from the accepted design decision
`_ai/technical_decisions/ADR__261119-1__plain-language-base-with-git-wording-overlay.md`)

## Summary

Add a personal, device-local `wording` preference (`plain` | `git`) that is
orthogonal to the existing `language` preference (`de` | `en`). Plain remains
the default and the complete source of truth. Git wording is a small,
typed overlay keyed by message key: `t()` resolves
`overlay[language][key] ?? catalogue[language][key] ?? en[key]`, so a missing or
empty Git variant falls back to the plain label. The overlay covers only short
labels: operation verbs, confirmation/dialog titles, affected status badges and
filters, insight action labels, onboarding operation headings, and glossary
names. Explanations, hints, errors, and the “What Git did” drawer stay plain.

In plain mode, operation hints keep their existing explanation and gain the
canonical Git term as a suffix, for example
`Save changes in this project to Git (git commit)`. The hint is visible on
hover **and** keyboard focus and is referenced with `aria-describedby`, not
only `title`. A compact sibling control beside the existing language switcher
selects the wording. No Rust, backend, config-schema, or UI-kit change is
required.

## Technical Context

**Language/Version**: TypeScript 5.5, Vue 3.4 (`<script setup>`), Pinia 2.2

**Primary Dependencies**: `@topdata-software-gmbh/ui-kit-web-vue3` (`TdMenu`,
`TdMenuItem`, `TdButton`, `TdBadge`, `TdModal`, `usePopover`), `lucide-vue-next`
icons, the existing `backend` client. No new dependency.

**Storage**: Browser/desktop-webview `localStorage` key `topdata.wording.v1`,
values `plain` | `git`, default `plain`. Never in `AppConfig`, never per
repository, no server sync. Uses the same safe storage adapter pattern as the
existing language preference.

**Testing**: Vitest 2.1 + `@vue/test-utils` + `happy-dom` (`pnpm test`), static
type check via `vue-tsc --noEmit` (`pnpm typecheck`). Tests cover overlay
parity/fallback, persistence, independence from language, hint suffixing,
accessible hint wiring, and live label updates.

**Target Platform**: Desktop (Tauri v2) and browser (axum-served Vue UI); both
share the same Vue app, so behavior must be identical in both.

**Project Type**: Single repository — Vue frontend in `src/`, Rust service in
`src-tauri/`. This feature touches the frontend only.

**Performance Goals**: No measurable impact. One synchronous `localStorage`
read at startup and one O(1) overlay lookup per `t()` call. No new backend
calls, timers, or watchers.

**Constraints**: Plain must remain the default and complete; all four
language/wording combinations must render; canonical English Git words are used
in both languages; no literal copy in components; no change to Git output,
commit contents, branch names, or repository data; no kit modification; no
backend or `AppConfig` change; existing explanations, hints, error guidance and
the Git log drawer must not be reworded.

**Scale/Scope**: Two small i18n modules (`wording.ts`, `gitWording.ts`), one
resolution change in `t()`, one hint helper, two app-owned components
(`WordingSwitcher.vue`, `OperationHint.vue`), one sidebar insertion, about 47
overlay keys per language, and focused unit/component tests. No Rust files.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

- **I. Spec-Driven Change** — PASS: `specs/005-plain-git-wording/spec.md` and
  accepted ADR 261119-1 exist; this plan derives from them before code work.
- **II. One Core, Two Frontends** — PASS: frontend-only change. It reads and
  writes a browser preference; it adds no command, event, HTTP route, or
  business logic to either transport, and no ad-hoc payloads or direct
  filesystem access.
- **III. Git Only Through the System CLI** — PASS: no Git operation is added,
  changed, or executed. The preference only changes labels; Git output, commit
  contents, branch names, and repository data are untouched (FR-014).
- **IV. Beginner-Friendly by Default** — PASS: plain wording remains the default
  and the complete catalogue, so the beginner-first voice is preserved. Git
  terminology is an explicit personal opt-in, not a default. All new copy goes
  into the app-owned DE/EN dictionaries; no component literals.
- **V. Bounded, Non-Destructive Action** — PASS: the preference is a synchronous
  local read/write; it cannot modify repository state. No locks, concurrency,
  or destructive controls are involved. Invalid stored values fall back to
  plain without failing startup.

**Result**: No violations; no complexity-tracking entries required.

## Project Structure

### Documentation (this feature)

```text
specs/005-plain-git-wording/
├── plan.md              # This file
├── spec.md              # Feature specification
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── i18n-wording-contract.md
└── tasks.md             # Phase 2 output (/speckit.tasks — not created here)
```

### Source Code (repository root)

```text
src/
├── lib/
│   └── i18n/
│       ├── index.ts                 # t(), operationHint(), overlay resolution
│       ├── wording.ts               # Wording state, storage key, safe adapter
│       ├── gitWording.ts            # Per-language Git overlay (typed)
│       ├── messages.en.ts           # New control labels only; plain stays base
│       └── messages.de.ts           # Same new keys in German
├── components/
│   ├── WordingSwitcher.vue          # Compact sibling of TdLanguageSwitcher
│   ├── OperationHint.vue            # Hover/focus tooltip + aria-describedby
│   ├── Sidebar.vue                  # Mounts WordingSwitcher in footer
│   ├── Toolbar.vue                  # Operation labels/hints
│   ├── RepoListTable.vue            # Row operation labels/hints
│   ├── SaveChangesModal.vue         # Dialog/confirmation labels
│   ├── HelpModal.vue                # Glossary names (keys already used)
│   └── onboarding/OnboardingStepReady.vue
├── lib/
│   ├── repoStatus.ts                # Badge labels resolve through t()
│   └── repoFilters.ts               # Filter labels resolve through t()
└── lib/__tests__/ and components/__tests__/
    ├── wording.test.ts              # State, storage, fallback, parity, hints
    ├── wording-switcher.test.ts     # Control state and live switch
    └── repoStatus.test.ts           # Badge overlay (extend existing)
```

**Structure Decision**: The frontend single-project layout is used. Wording
state and overlay resolution stay in `src/lib/i18n/`; components only call
`t()`, `operationHint()` and `setWording()`. The store is not rewritten: its
existing `t()` call sites and `action.*`/`toast.*` keys resolve through the same
overlay automatically. No Rust file changes.

## Phase 0: Research

See [research.md](./research.md). All decisions are recorded there; there are no
unresolved `NEEDS CLARIFICATION` items.

## Phase 1: Design & Contracts

- Entities, storage shape and resolution precedence:
  [data-model.md](./data-model.md)
- `t()`/`operationHint()`/overlay/component contracts:
  [contracts/i18n-wording-contract.md](./contracts/i18n-wording-contract.md)
- End-to-end validation for all four combinations:
  [quickstart.md](./quickstart.md)

### Implementation outline

1. Add `src/lib/i18n/wording.ts`:
   - `Wording = 'plain' | 'git'`, `WORDINGS`, `WORDING_STORAGE_KEY =
     'topdata.wording.v1'`.
   - A safe storage adapter, a `defaultWording` of `plain`, an invalid-value
     guard, and a reactive `wording` ref plus `setWording()`.
   - A factory (for storage/fallback tests) plus the module singleton used by
     the app, mirroring the kit's `useLanguage` shape without changing the kit.
2. Add `src/lib/i18n/gitWording.ts` with one typed
   `Partial<Record<MessageKey, string>>` per language. Values are identical
   canonical English Git terms; the per-language shape keeps the FR-010
   fallback explicit.
3. Update `t()` in `src/lib/i18n/index.ts` to resolve the Git overlay only when
   `wording.value === 'git'`, ignoring missing or blank overlay values so a
   label can never be empty or a placeholder.
4. Add `operationHint(key, term)` that returns the plain hint unchanged in Git
   mode and appends ` (git <term>)` in plain mode. Use a small `GitTerm` union
   so call sites cannot drift from the canonical terms.
5. Add `WordingSwitcher.vue` and mount it beside `TdLanguageSwitcher` in the
   sidebar footer. Its accessible name includes the current state; its menu
   items expose the active choice with a visible check and a screen-reader-only
   “current” label.
6. Add `OperationHint.vue`, built on the kit's exported `usePopover`, and use it
   for the toolbar operation buttons and the table row operation buttons. Menu
   items keep their always-visible description slot, which carries the same
   hint. An always-present visually hidden description is referenced by
   `aria-describedby`; the visible tooltip is shown on hover and focus-within
   and closes on Escape.
7. Wire the new hint helper into the operation controls:
   toolbar Save/Get latest/Send changes and the Fetch/Stash/Bring back menu
   descriptions; table Save and per-row pull/push controls.
8. Add only the new control labels to `messages.en.ts` / `messages.de.ts`; the
   Git overlay lives in `gitWording.ts`, so the plain catalogues stay the
   source of truth.
9. Add tests for overlay parity, fallback, persistence, invalid storage,
   language independence, hint suffixing, badge overlay and live component
   update. Update `CHANGELOG.md` in the same change.

## Post-Design Constitution Re-check

- **I** — PASS: plan, research, data model, contract and quickstart all derive
  from the accepted spec/ADR.
- **II** — PASS: no transport, command, event or Rust change; the preference is
  frontend-local.
- **III** — PASS: no Git invocation changes; only label resolution changes.
- **IV** — PASS: plain remains default and complete; new copy is in DE/EN
  dictionaries; Git mode is opt-in; explanations stay plain.
- **V** — PASS: read-only local preference with validation and fallback; no
  repository mutation or unbounded work.

**Result**: No new violations.

## Complexity Tracking

> No constitutional violations; table intentionally empty.

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|------------------------------------|
| — | — | — |
