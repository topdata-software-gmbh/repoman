---
filename: "_ai/backlog/reports/261101_0044__BUGFIX_REPORT__viewport-safe-popovers.md"
title: "Bugfix: Viewport-safe popover layer (Escape the Box)"
createdAt: 2026-11-01 00:44
updatedAt: 2026-11-01 00:44
project: "topdata-design-system + repo-manager"

severity: medium
impact: "Every dropdown near the viewport bottom (RepoMan sidebar footer '...' menu, language switcher, toolbar 'more Git actions') opened off-screen and visibly shifted the whole app layout upward."
status: complete

fixState: verified
environments:
  local: verified
  staging: n/a
  production: not-deployed

fixType: root-cause
regressionTest: true
deployTarget: "topdata-design-system branch 001-viewport-safe-popovers (consumed by RepoMan via link: dependency)"
incident: false

filesCreated: 6
filesModified: 10
filesDeleted: 0
tags: [bugfix, ui-kit, popover, tauri, design-system, spec-kit]
documentType: BUGFIX_REPORT

relatedPlan: "specs/001-viewport-safe-popovers/plan.md (topdata-design-system)"
---

# Bugfix: Viewport-safe popover layer (Escape the Box)

## 1. Summary

RepoMan (Tauri 2 + Vue 3) showed broken dropdowns at the bottom of the window:
the sidebar footer "..." menu and language switcher opened downward past the
window edge, were clipped, and the underlying page visibly shifted up while a
menu was open. Cause: the UI kit's `TdMenu` panel is `position: absolute`
inside the trigger's own wrapper, always opening downward (`top: calc(100% +
…)`), with no collision handling — near the viewport bottom it overflows the
document and stretches the flex layout. Fix: a generic, shared popover layer in
the design system — new `usePopover` composable (`@floating-ui/dom`, teleported
to `body`, `position: fixed`, flip/shift/max-height/trigger-tracking) adopted
by `TdMenu`, which `TdLanguageSwitcher` composes; zero public-API change.
Verified locally (193 kit tests, typecheck, build); manual Tauri smoke pending.

## 2. Prompt / trigger

> the dropdowns at the bottom go in wrong direction and destroy layout
> (everyhing shifts up) .. lets discuss a generic solution .. maybe using spec kit?
(plus two screenshots: clipped white panel below "Erneut suchen DE …", and
"Einrichtung erneut starten" menu hanging off the window bottom edge)

## 3. Reproduction

- **Environment**: Linux, Tauri 2 (WebKitGTK), RepoMan at `3c3465c`, kit at
  `0636920` (linked as `link:../topdata-design-system/packages/ui-kit-web-vue3`).
- **Steps** (deterministic): 1) open RepoMan in a short window; 2) click the
  "..." (or "DE") button in the sidebar footer.
- **Expected**: menu fully visible (opening upward near the bottom edge), page
  does not move.
- **Actual**: menu renders below the trigger, runs off-screen, and surrounding
  content shifts up while it is open.

## 4. Root cause

`packages/ui-kit-web-vue3/src/components/TdMenu.vue` rendered
`.td-menu__panel` (`position: absolute; top: calc(100% + …)`) **inside** the
`position: relative` `.td-menu` wrapper in the component tree. Two defects:
(a) the open direction is hardcoded downward — no space detection, so bottom
triggers overflow the viewport; (b) as a statically positioned descendant the
panel extends the document's scrollable area, so the window-height flex layout
of the Tauri shell reflows (the "shift up"). Ancestor `overflow`/`transform`
can additionally clip it. **Why it escaped**: kit tests and the playground only
exercised menus in mid-page contexts; no test asserted panel-viewport
containment or layout stability.

## 5. Evidence

| | Before | After |
|---|---|---|
| Panel in DOM | inside `.td-menu` subtree | teleported, `document.body` child (`test: teleports the panel to document.body`) |
| Positioning | static `top: calc(100% + …)`, always below | inline `position: fixed` + `left/top` from Floating UI (`test: positions the panel fixed via the popover layer`) |
| Direction | fixed down | `bottom-start|end` + `flip`/`shift`/`size` middleware (`test: registers flip/shift/size middleware`) |
| Outside-click vs detached panel | n/a | panel subtree ignored (`test: stays open when a pointerdown lands on the teleported panel`) |
| Trigger leaving viewport | n/a | menu auto-closes (`test: invokes onTriggerHidden`) |

## 6. Fix

`fixType: root-cause`. New shared composable `usePopover` (Constitution V: one
implementation) wraps `@floating-ui/dom`: `strategy: "fixed"`, `offset(4)`,
`flip`, `shift`, `size` (maxHeight + inner scroll), `autoUpdate`, and a
rect check that closes the menu when the trigger fully leaves the viewport.
`TdMenu.vue` mounts its panel through `<Teleport to="body">` and binds the
composable; `menu.css` switches to `position: fixed` and drops the
`--start/--end` CSS offsets (placement now computed). This removes the
mechanism entirely rather than capping heights locally; `TdSelect` (native
`<select>`) was checked and is unaffected. Process: spec-kit — new
`.specify/` setup, constitution Principle II "Escape the Box", feature spec +
plan + tasks under `specs/001-viewport-safe-popovers/`.

## 7. Files changed

Created (topdata-design-system):
- `packages/ui-kit-web-vue3/src/composables/usePopover.ts` — shared positioning engine
- `packages/ui-kit-web-vue3/src/composables/usePopover.test.ts` — 7 unit tests (mocked floating-ui)
- `_ai/specs/SPEC__261101-1__viewport-safe-popovers.md` — kit spec, Requirement/Scenario
- `specs/001-viewport-safe-popovers/{spec,plan,tasks}.md` — spec-kit artifacts (branch `001-viewport-safe-popovers`)

Modified:
- `packages/ui-kit-web-vue3/src/components/TdMenu.vue` — Teleport + usePopover + panel-aware outside click + `scrollIntoView`
- `packages/ui-kit-web-vue3/src/components/TdMenu.test.ts` — 3 portal/positioning tests + 1 scroll test
- `packages/ui-kit-web-vue3/src/styles/components/menu.css` — fixed positioning, border-box, no static top
- `packages/ui-kit-web-vue3/src/index.ts` — export `usePopover` + types
- `packages/ui-kit-web-vue3/package.json` + `pnpm-lock.yaml` — add `@floating-ui/dom@^1.8`
- `packages/ui-kit-web-vue3/README.md` — "Popover positioning contract" section, traceability row
- `packages/ui-component-registry.yaml` — kit notes updated
- `_ai/specs/index.md` — new spec listed
- `playground/src/App.vue` — "Viewport edge audit" demo (bottom-start, tall menu at bottom-end, language switcher at edge)

Also created but tooling-only: `.specify/`, `.opencode/command/` (spec-kit
scaffold — production-irrelevant). **RepoMan: no source changes** — consumed
via `link:` dependency.

## 8. Verification

| Check | Before | After | Evidence |
|---|---|---|---|
| Kit unit tests | 188 pass | 193 pass (20 files) | `pnpm vitest run` in ui-kit-web-vue3 |
| Typecheck | clean | clean | `pnpm vue-tsc --noEmit` (kit + playground) |
| Build | — | succeeds | `pnpm build` (vite + dts) |
| RepoMan regression | 47 pass | 47 pass | `pnpm test` + `pnpm typecheck` in /topdata/repoman |
| Portal placement logic | not covered | 11/11 in TdMenu.test.ts | names in §5 |
| **Not verified** | | | Manual `pnpm tauri dev` smoke (tasks T014) — automated happy-dom tests cannot assert real geometry; playground edge audit (SC-002) not yet run in a browser |

## 9. Deployment & rollback

- **Precondition**: user runs the Tauri smoke test above (T014).
- **Deploy**: commit + push branch `001-viewport-safe-popovers` in
  `topdata-design-system`; merge per team flow. RepoMan picks the fix up on its
  next install/build (`link:` dependency, kit already rebuilt into `dist/`).
- **Rollback**: revert the kit commit(s); RepoMan needs no coordinated
  rollback (no API change). `pnpm install` restores the old lockfile; delete
  `.specify/`, `.opencode/command/`, `specs/001-…` if full removal wanted.
- **Post-deploy check**: open all three RepoMan menus at the window bottom;
  panels open upward, page static.

## 10. Blast radius

Every consumer of `TdMenu`/`TdLanguageSwitcher` gets the new behavior — only
RepoMan today (per registry), TradeGuard later. Shared code touched: outside
pointerdown handler (now also ignores panel subtree — any app relying on
click-through to the panel would notice; none do), Escape/Tab/focus cycle
unchanged, z-index 30 now on a body-level node (could paint above app-local
overlays — RepoMan's TdModal-based dialogs sit at higher modal stacking and
were not exercised with an open menu simultaneously). Apps that had added
local CSS workarounds for the old clipping would need to remove them; RepoMan
has none.

## 11. Prevention / follow-ups

- [ ] Run T014 (Tauri smoke) and T015 (CHANGELOG entry + commit) — owner: user/marc.
- [x] Regression tests added for portal + placement wiring (`usePopover.test.ts`, `TdMenu.test.ts`).
- [x] Constitution Principle II "Escape the Box" codifies the invariant for future components; plan gate requires it.
- [ ] Future popover components (tooltip, dropdown) must use `usePopover` — enforce in review (Constitution V).
- [ ] Optional: a geometry-capable browser test runner (Playwright project) for real flip/shift assertions — currently only the playground audit covers geometry.

## 12. Residual risks & explicitly not changed

- happy-dom cannot validate real collision geometry; flip correctness rests on
  Floating UI (battle-tested) + the playground audit not yet executed.
- `visibility: hidden` until first position update could theoretically leave
  a panel invisible if positioning throws — fail-safe choice (a hidden panel
  beats layout destruction); no telemetry if that happens.
- Not changed: `TdSelect` (native popup), RepoMan app code, `td-menu`
  transition CSS (none exists), stacked/simultaneous menus (out of scope),
  no `placement` override prop (YAGNI decision, spec clarification).

## 13. Links

- Feature spec/plan/tasks: `specs/001-viewport-safe-popovers/` (design-system repo)
- Kit spec: `_ai/specs/SPEC__261101-1__viewport-safe-popovers.md`
- Constitution: `.specify/memory/constitution.md` (Principle II)
- Related implementation report (prior session, same UI migration): `_ai/backlog/reports/261031_2348__IMPLEMENTATION_REPORT__language-switcher-and-german-ui.md`
