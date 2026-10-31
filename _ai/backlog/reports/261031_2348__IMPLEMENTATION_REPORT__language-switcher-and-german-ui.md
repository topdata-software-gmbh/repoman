---
filename: "_ai/backlog/reports/261031_2348__IMPLEMENTATION_REPORT__language-switcher-and-german-ui.md"
title: "Report: Language switcher and German UI (design system + repo-manager)"
createdAt: 2026-10-31 23:48
updatedAt: 2026-10-31 23:48
planFile: "_ai/backlog/active/261031_2243__IMPLEMENTATION_PLAN__language-switcher-and-german-ui.md"
specFile: ""
project: "repo-manager"
status: completed
filesCreated: 11
filesModified: 32
filesDeleted: 0
tags: [i18n, german, vue, design-system, accessibility]
documentType: IMPLEMENTATION_REPORT
---

# Report: Language switcher and German UI

## Summary

Added `useLanguage` and `TdLanguageSwitcher` to `@topdata-software-gmbh/ui-kit-web-vue3`
(preference persistence under `topdata.language.v1`, `<html lang>` sync, compact menu
switcher). Repo-manager gained an app-owned typed translation runtime (`src/lib/i18n/`)
with complete English and German (informal "du") catalogues, `t()`/`tPlural()` helpers, a
German default with a pre-paint bootstrap in `index.html`, and every Vue-owned string —
templates, attributes, confirms, validation, store insights, and toasts — externalised to
catalogue keys. The switcher sits in the sidebar footer beside the theme toggle.

## Files Changed

### Created

Design system (`/topdata/topdata-design-system`):

- `packages/ui-kit-web-vue3/src/composables/useLanguage.ts` — language preference primitive with storage adapter, supported-list guard, `<html lang>` sync.
- `packages/ui-kit-web-vue3/src/composables/useLanguage.test.ts` — 6 tests (default, stored, fallback, persist, reject unsupported, custom adapter).
- `packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.vue` — ghost trigger with active short code, `TdMenu` of endonyms, active check mark.
- `packages/ui-kit-web-vue3/src/components/TdLanguageSwitcher.test.ts` — 7 tests incl. emit/no-emit, disabled options, token-only CSS, axe check.
- `packages/ui-kit-web-vue3/src/styles/components/language-switcher.css` — token-only styles for code chip and check mark.
- `_ai/specs/SPEC__261031-1__td-language-switcher.md` — Requirement/Scenario spec.

Repo-manager (`/topdata/repoman`):

- `src/lib/i18n/index.ts` — runtime: `language`, `setLanguage`, `t()`, `tPlural()`, one shared `useLanguage` instance (default `de`).
- `src/lib/i18n/messages.en.ts` — English catalogue (source of truth, `as const`, exports `MessageKey`).
- `src/lib/i18n/messages.de.ts` — German catalogue typed `Record<MessageKey, string>`, informal "du".
- `src/lib/__tests__/i18n.test.ts` — key-parity, interpolation, plural, reactivity tests.
- `_ai/technical_decisions/ADR__261031-1__app-owned-translations-and-dictionary-free-kit.md` — durable boundary ADR.

### Modified

Design system:

- `packages/ui-kit-web-vue3/src/types.ts` — added `TdLanguageOption`.
- `packages/ui-kit-web-vue3/src/index.ts` — component/composable exports + `language-switcher.css` import.
- `packages/ui-kit-web-vue3/README.md` — Language section, component and traceability rows.
- `playground/src/App.vue` — switcher in header and a documented actions-section demo.
- `_ai/specs/index.md` — added SPEC__261031-1.
- `packages/ui-component-registry.yaml` — `updated_at: "2026-10-31"`.
- `CHANGELOG.md` — Unreleased/Added entries.

Repo-manager:

- `index.html` — `lang="de"` and inline bootstrap that reads `topdata.language.v1` before first paint.
- `src/main.ts` — imports `./lib/i18n` so the runtime applies at startup.
- `src/App.vue` — live-updates toast title via `t()`.
- `src/components/Sidebar.vue` — full translation, `TdLanguageSwitcher` in the footer with a narrowing `chooseLanguage` handler.
- `src/components/Toolbar.vue`, `RepoTable.vue`, `RepoListTable.vue`, `AllReposPage.vue` — all copy via `t()`/`tPlural()`; filter options became computed; badge icons switched from label comparison to stable `badge.kind`.
- `src/components/HelpModal.vue`, `GroupEditorModal.vue`, `AddScanPathModal.vue`, `SaveChangesModal.vue`, `GitLogDrawer.vue` — full translation (drawer chrome only; backend transcript untouched, source comment added).
- `src/components/OnboardingWizard.vue` and `src/components/onboarding/*.vue` — full translation; steps/grouping options became computed.
- `src/lib/repoStatus.ts` — `StatusBadge.kind` (`attention`/`operation`), labels via `t()`/`tPlural()`.
- `src/lib/onboarding.ts` — `suggestGroups` gained `fallbackRootName` (default `'Projects'` keeps existing tests calling the 3-arg form).
- `src/stores/repoStore.ts` — `ACTION_COPY` replaces `ACTION_LABELS`; translated `summarizeResults`, all toasts, insight computed, validation errors, and the translated fallback group name; unused `projectWord` helper removed.
- `src/lib/__tests__/repoStatus.test.ts` — pinned to English, plus a German badge test.
- `README.md` — new "Languages" section.
- `CHANGELOG.md` — Unreleased/Added entries.
- `_ai/technical_decisions/README.md` — ADR index entry.

### Deleted

None.

## Key Changes

- **Kit primitives**: `useLanguage` (per-call ref, mirrors `useTheme`, custom storage adapter, rejects unsupported codes) and `TdLanguageSwitcher` (menu of endonyms, active code on trigger, `update:modelValue`, axe-clean).
- **App catalogue**: two locales enforced key-identical at compile time via `Record<MessageKey, string>`; runtime interpolation with unknown placeholders left intact; `tPlural` on `.one`/`.other` bases.
- **German default**: pre-paint inline bootstrap sets `<html lang>` from storage (fallback `de`) so there is no English flash; `useLanguage` keeps it in sync afterwards.
- **Stable status kind**: icons and Vue `:key`s no longer depend on translated label text (`badge.kind === 'attention' | 'operation'`).
- **Store copy**: insights react instantly to language switches (D9); toasts keep their creation language.

## Deviations from Plan

- `MessageKey` is defined and exported in `messages.en.ts` and re-exported by `index.ts`
  (the plan defined it in `index.ts` while `messages.de.ts` imports it from `messages.en.ts`).
- `TdLanguageSwitcher.vue` declares `DEFAULT_LANGUAGES` in a module-scope `<script>` block —
  `defineProps` defaults cannot reference `<script setup>` locals (compiler error otherwise).
- Sidebar wires the switcher through `chooseLanguage(value: string)` to narrow the emit to the
  `Language` union; the plan's direct `@update:model-value="setLanguage"` fails typecheck.
- `ACTION_COPY` carries a third key `failed` (e.g. `toast.fetchFailed`) for the outer catch of
  `executeGroupAction`/`runRepoAction`, replacing the old `${verb} failed` template; the branch
  switch catch uses `toast.switchToFailed` with `{branch}`. All keys were already in the catalogues.
- `save.workingOn` is interpolated as one sentence; the branch name no longer gets its own
  `font-mono` span (a translated string cannot split around markup).
- `empty.noProjectsInGroup.text` interpolates the group name as plain text (per plan), dropping
  the inline `text-fg` span styling of the group name.
- Toolbar's "How this works" menu entry uses `sidebar.howThisWorks` (only catalogue key with that wording).
- READMEs/kit docs: `Language` added as an `##` section (peer of `Theme`) instead of the plan's `###`.
- ADR frontmatter uses `deciders: [Marc]` to match the existing ADRs in the repo.
- The repo-manager working tree contains a concurrent, unrelated rename workstream
  (`git-repo-manager` → `repoman` in `package.json`, `src-tauri/`, `scripts/`, `src/lib/backend.ts`,
  and plan-doc edits). Those files were not touched by this implementation and should be
  committed separately.

## Technical Decisions

D1–D10 from the plan are all confirmed as implemented; ADR 261031-1 records the durable
boundary. No new catalogue keys were needed beyond those planned (the six `filters.*`
chip-label keys were already included in Phase 4).

## Testing Notes

```bash
cd /topdata/topdata-design-system && pnpm typecheck && pnpm test && pnpm --filter @topdata-software-gmbh/ui-kit-web-vue3 build
cd /topdata/repoman && pnpm typecheck && pnpm test && pnpm build
```

Results at completion:

- Design system: typecheck clean, 19 files / 183 tests passed (13 new), kit build succeeded.
- Repo-manager: typecheck clean, 7 files / 47 tests passed (6 new), `pnpm build` succeeded.

Manual checklist (from the plan, still to walk through in the running app):

1. Fresh profile (clear `localStorage`): German UI, `<html lang="de">`, sidebar switcher shows `DE`.
2. Switch to English: shell, table, modals, onboarding, and the visible insight banner update without reload; `<html lang="en">`.
3. Reload keeps English; switch back to German persists.
4. Check modal footers for clipped long German labels ("Einrichtung abschließen").
5. Toast pushed in German keeps its language after switching (expected per D9).

## Usage Examples

- Add a string: put the key in `src/lib/i18n/messages.en.ts`, add the German twin in
  `messages.de.ts` (a missing key fails `pnpm typecheck`), call `t('ns.key', { param })`
  or `tPlural('ns.key', n)`; plurals use `<base>.one`/`<base>.other`.
- Switch language programmatically: `import { setLanguage } from './lib/i18n'`.
- Stored preference: `localStorage['topdata.language.v1']` (`"de"` | `"en"`).

## Documentation Updates

- ADR `261031-1` created and indexed in `_ai/technical_decisions/README.md`.
- Repo-manager `README.md` gained a "Languages" section.
- Kit `README.md` gained Language section + component/traceability rows; `SPEC__261031-1`
  created and indexed; registry `updated_at` bumped.
- `CHANGELOG.md` in both repos updated under Unreleased/Added.
- `_ai/conventions.md` untouched (generated by `ruleman export`).

## Next Steps

- Optional: localize the Git transcript (`git-actions.yaml` `title_de`/`comment_de`) —
  deliberately out of scope (D8).
- Optional: label-override props for kit-internal English accessible names (ADR consequence).
- Optional: persist language in `AppConfig` if the preference should follow the user
  across machines (D5 revisit).
