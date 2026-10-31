---
filename: "/topdata/repoman/_ai/backlog/reports/261031_2348__IMPLEMENTATION_REPORT__language-switcher-and-german-ui.md"
title: "Report: Language switcher and German UI (design system + repo-manager)"
createdAt: 2026-10-31 23:48
updatedAt: 2026-11-01 00:04
repoRoot: "/topdata/repoman"
project: "repo-manager"
status: complete
filesCreated: 4
filesModified: 22
filesDeleted: 0
tags: [i18n, german, vue, design-system, accessibility]
documentType: IMPLEMENTATION_REPORT
---

# Report: Language switcher and German UI

## Summary

Added `useLanguage` and `TdLanguageSwitcher` to `@topdata-software-gmbh/ui-kit-web-vue3`
(preference persisted under `topdata.language.v1`, `<html lang>` sync, compact menu
switcher). Repo-manager gained an app-owned typed translation runtime
(`/topdata/repoman/src/lib/i18n/`) with complete English and German (informal "du")
catalogues, `t()`/`tPlural()` helpers, a German default with a pre-paint bootstrap in
`/topdata/repoman/index.html:2`, and every Vue-owned string — templates, attributes,
confirms, validation, store insights, toasts — externalised to catalogue keys. The
switcher sits in the sidebar footer beside the theme toggle.

## Prompt used

`/implement-plan` invoked with the plan file
`/topdata/repoman/_ai/backlog/active/261031_2243__IMPLEMENTATION_PLAN__language-switcher-and-german-ui.md`
("Execute an implementation plan across all phases without asking for confirmation").
All plan decisions D1–D10 were confirmed in the preceding design conversation; no
mid-plan prompts were issued.

## Files Changed

`filesCreated` / `filesModified` count `repoRoot` (`/topdata/repoman`) project source
files only. Design-system repo files, in-repo documentation, and this report are
listed in the bodies below but not counted.

### Created

Repo-manager source:

- `/topdata/repoman/src/lib/i18n/index.ts:36` — runtime: `language`, `setLanguage`,
  `t()` (`:36`), `tPlural()` (`:47`), one shared `useLanguage` instance, default `de`.
- `/topdata/repoman/src/lib/i18n/messages.en.ts:6` — English catalogue, single source
  of truth (`as const`), exports `MessageKey`.
- `/topdata/repoman/src/lib/i18n/messages.de.ts:7` — German catalogue typed
  `Record<MessageKey, string>`, informal "du".
- `/topdata/repoman/src/lib/__tests__/i18n.test.ts` — key-parity, interpolation,
  plural, and reactivity tests (5 tests).

Repo-manager documentation (not counted):

- `/topdata/repoman/_ai/technical_decisions/ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md`
  — durable kit-boundary ADR.

Design-system source (`/topdata/topdata-design-system`, separate repo, not counted):

- `.../packages/ui-kit-web-vue3/src/composables/useLanguage.ts:64` — language
  preference primitive: storage adapter, supported-list guard, `<html lang>` sync.
- `.../packages/ui-kit-web-vue3/src/composables/useLanguage.test.ts` — 6 tests.
- `.../packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.vue` — ghost
  trigger with active short code, `TdMenu` of endonyms, active check mark.
- `.../packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.test.ts` — 7 tests
  incl. emit/no-emit, disabled options, token-only CSS, axe check.
- `.../packages/ui-kit-web-vue3/src/styles/components/language-switcher.css` —
  token-only styles for the code chip and check mark.
- `.../_ai/specs/SPEC__261031-1__td-language-switcher.md` — Requirement/Scenario spec.

### Modified

Repo-manager source:

- `/topdata/repoman/index.html:2` — `lang="de"` plus inline bootstrap reading
  `topdata.language.v1` before first paint (`:15`).
- `/topdata/repoman/src/main.ts:7` — imports `./lib/i18n` so the runtime applies at startup.
- `/topdata/repoman/src/App.vue` — live-updates toast title via `t()`.
- `/topdata/repoman/src/components/Sidebar.vue:40` — full translation;
  `TdLanguageSwitcher` in the footer with narrowing `chooseLanguage` handler (`:40`).
- `/topdata/repoman/src/components/Toolbar.vue`, `RepoTable.vue`, `RepoListTable.vue`,
  `AllReposPage.vue` — all copy via `t()`/`tPlural()`; filter options became computed;
  badge icons switched from label comparison to stable `badge.kind`.
- `/topdata/repoman/src/components/HelpModal.vue`, `GroupEditorModal.vue`,
  `AddScanPathModal.vue`, `SaveChangesModal.vue`, `GitLogDrawer.vue` — full
  translation (drawer chrome only; backend transcript untouched, source comment added).
- `/topdata/repoman/src/components/OnboardingWizard.vue` and
  `/topdata/repoman/src/components/onboarding/OnboardingStepFind.vue`,
  `OnboardingStepChoose.vue`, `OnboardingStepGroups.vue`, `OnboardingStepReady.vue` —
  full translation; step/grouping options became computed.
- `/topdata/repoman/src/lib/repoStatus.ts:5` — `StatusBadgeKind` (`attention`/
  `operation`), labels via `t()`/`tPlural()`.
- `/topdata/repoman/src/lib/onboarding.ts:89` — `suggestGroups` gained
  `fallbackRootName` (default `'Projects'` keeps the existing 3-arg test calls valid).
- `/topdata/repoman/src/stores/repoStore.ts:78` — `ACTION_COPY` replaces
  `ACTION_LABELS`; translated `summarizeResults`, toasts, insight computed,
  validation errors, translated fallback group name; unused `projectWord` helper removed.
- `/topdata/repoman/src/lib/__tests__/repoStatus.test.ts` — pinned to English, plus a
  German badge test.

Repo-manager documentation (not counted):

- `/topdata/repoman/README.md` — new "Languages" section.
- `/topdata/repoman/CHANGELOG.md` — Unreleased/Added entries.
- `/topdata/repoman/_ai/technical_decisions/README.md` — ADR index entry.

Design-system files (`/topdata/topdata-design-system`, not counted):

- `.../packages/ui-kit-web-vue3/src/types.ts` — added `TdLanguageOption`.
- `.../packages/ui-kit-web-vue3/src/index.ts` — component/composable exports +
  `language-switcher.css` import.
- `.../packages/ui-kit-web-vue3/README.md` — Language section, component and
  traceability rows.
- `.../playground/src/App.vue` — switcher in the header and a documented actions-section demo.
- `.../_ai/specs/index.md` — added SPEC__261031-1.
- `.../packages/ui-component-registry.yaml` — `updated_at: "2026-10-31"`.
- `.../CHANGELOG.md` — Unreleased/Added entries.

### Deleted

None.

## Key Changes

- **Kit primitives**: `useLanguage` (per-call ref, mirrors `useTheme`, custom storage
  adapter, rejects unsupported codes) and `TdLanguageSwitcher` (menu of endonyms,
  active code on trigger, `update:modelValue`, axe-clean).
- **App catalogue**: two locales kept key-identical at compile time via
  `Record<MessageKey, string>`; runtime interpolation leaves unknown placeholders
  intact; `tPlural` resolves `<base>.one`/`.other` by count.
- **German default**: the pre-paint inline script in `/topdata/repoman/index.html`
  sets `<html lang>` from storage (fallback `de`) so there is no English flash;
  `useLanguage` keeps it in sync afterwards.
- **Stable status kind**: icons and Vue `:key`s no longer depend on translated label
  text (`badge.kind === 'attention' | 'operation'`).
- **Store copy**: the insight banner reacts instantly to language switches; visible
  toasts keep their creation language (plan decision D9).

## Deviations from Plan

- `MessageKey` is defined and exported in `/topdata/repoman/src/lib/i18n/messages.en.ts`
  and re-exported by `index.ts` — the plan defined it locally in `index.ts` while
  `messages.de.ts` imports it from `messages.en.ts`.
- `TdLanguageSwitcher.vue` declares `DEFAULT_LANGUAGES` in a module-scope `<script>`
  block: `defineProps` defaults cannot reference `<script setup>` locals (compiler
  error otherwise).
- Sidebar wires the switcher through `chooseLanguage(value: string)`
  (`/topdata/repoman/src/components/Sidebar.vue:40`) to narrow the emit to the
  `Language` union; the plan's direct `@update:model-value="setLanguage"` fails typecheck.
- `ACTION_COPY` (`/topdata/repoman/src/stores/repoStore.ts:78`) carries a third key
  `failed` (e.g. `toast.fetchFailed`) for the outer catch of
  `executeGroupAction`/`runRepoAction`, replacing the old `${verb} failed` template;
  the branch-switch catch uses `toast.switchToFailed` with `{branch}`. All keys were
  already in the catalogues.
- `save.workingOn` is interpolated as one sentence; the branch name lost its own
  `font-mono` span (a translated string cannot be split around markup).
- `empty.noProjectsInGroup.text` interpolates the group name as plain text (per
  plan), dropping the inline `text-fg` span styling.
- Toolbar's "How this works" menu entry reuses `sidebar.howThisWorks` (only catalogue
  key with that wording).
- Kit README: `Language` added as a `##` section (peer of `## Theme`) instead of the
  plan's `###`.
- ADR frontmatter uses `deciders: [Marc]` to match the existing ADRs in the repo.
- The repoman working tree contains a concurrent, unrelated rename workstream
  (`git-repo-manager` → `repoman` in `/topdata/repoman/package.json`,
  `/topdata/repoman/src-tauri/`, `/topdata/repoman/scripts/`,
  `/topdata/repoman/src/lib/backend.ts`, and plan-doc edits). Those files were not
  touched by this implementation and should be committed separately.

No layout fixes were forced by German length so far; long labels were verified only
via build/typecheck — the manual UI walk (see Testing Notes) is still pending a human.

## Technical Decisions

Plan decisions D1–D10 are confirmed as implemented; ADR
`/topdata/repoman/_ai/technical_decisions/ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md`
records the durable boundary (dictionary-free kit, app-owned typed catalogues, no
`vue-i18n` yet, `localStorage` persistence, Vue-only copy in scope). No new catalogue
keys were needed beyond those planned — the six `filters.*` chip-label keys were
already included in the Phase 4 catalogue.

## Testing Notes

```bash
# Design system (typecheck, tests, kit build)
pnpm -C /topdata/topdata-design-system typecheck
pnpm -C /topdata/topdata-design-system test
pnpm -C /topdata/topdata-design-system --filter @topdata-software-gmbh/ui-kit-web-vue3 build

# Repo-manager (typecheck, tests, production build)
pnpm -C /topdata/repoman typecheck
pnpm -C /topdata/repoman test
pnpm -C /topdata/repoman build
```

Results at completion:

- Design system: typecheck clean; 19 files / 183 tests passed (13 new); kit build succeeded.
- Repo-manager: typecheck clean; 7 files / 47 tests passed (6 new); `pnpm build` succeeded.

Manual checklist still to walk through in the running app:

1. Fresh profile (clear `localStorage`): German UI, `<html lang="de">`, sidebar switcher shows `DE`.
2. Switch to English: shell, tables, modals, onboarding, and the visible insight
   banner update without reload; `<html lang="en">`.
3. Reload keeps English; switching back to German persists.
4. Check modal footers for clipped long German labels ("Einrichtung abschließen").
5. A toast pushed in German keeps its language after switching (expected, D9).

## Usage Examples

- Add a string: insert the key in `/topdata/repoman/src/lib/i18n/messages.en.ts`, add
  the German twin in `/topdata/repoman/src/lib/i18n/messages.de.ts` (a missing key
  fails `pnpm -C /topdata/repoman typecheck`), then call
  `t('ns.key', { param })` / `tPlural('ns.key', n)`; plurals use `<base>.one`/`<base>.other`.
- Switch language programmatically:
  `import { setLanguage } from '/topdata/repoman/src/lib/i18n'`.
- Stored preference: `localStorage['topdata.language.v1']` (`"de"` | `"en"`).
- Kit consumer (any TopData Vue app):
  `useLanguage({ defaultLanguage: 'de', supported: ['de', 'en'] })` +
  `<TdLanguageSwitcher :model-value="language" @update:model-value="setLanguage" />`.

## Documentation Updates

- ADR `261031-1` created and indexed in `/topdata/repoman/_ai/technical_decisions/README.md`.
- Repo-manager `README.md` gained a "Languages" section.
- Kit `README.md` gained the Language section plus component and traceability rows;
  `SPEC__261031-1` created and indexed in `.../_ai/specs/index.md`; registry
  `updated_at` bumped in `.../packages/ui-component-registry.yaml`.
- `CHANGELOG.md` in both repos updated under Unreleased/Added.
- `/topdata/repoman/_ai/conventions.md` untouched (generated by `ruleman export`).
- Plan archived to
  `/topdata/repoman/_ai/backlog/archive/261031_2243__IMPLEMENTATION_PLAN__language-switcher-and-german-ui.md`
  (`status: completed`, `completedAt: 2026-10-31 23:50`).

## Next Steps

- Optional: localize the Git transcript (`title_de`/`comment_de` in
  `/topdata/repoman/src-tauri/macros/git-actions.yaml`) — deliberately out of scope (D8).
- Optional: label-override props for the kit's internal English accessible names
  (recorded ADR consequence).
- Optional: persist language in `AppConfig` if the preference should follow the user
  across machines (D5 revisit; needs Rust schema work).
- Pending human action: the manual UI walk above, and the commit decision — no
  commits were created (repo workflow: commit only on explicit request).
