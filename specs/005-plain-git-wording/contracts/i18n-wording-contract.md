# Contract: Wording State, Overlay Resolution and Accessible Hints

**Feature**: `005-plain-git-wording` | **Date**: 2026-11-19

This project exposes no public network API for this feature. The contracts below
are the internal frontend interfaces that must stay stable across the feature
and its tests. They are app-owned; the shared UI kit is not modified.

## 1. Wording state — `src/lib/i18n/wording.ts`

```ts
export type Wording = 'plain' | 'git';

export const WORDINGS: readonly Wording[] = ['plain', 'git'];
export const WORDING_STORAGE_KEY = 'topdata.wording.v1';

export interface WordingStorage {
  get(key: string): string | null;
  set(key: string, value: string): void;
}

export interface UseWordingOptions {
  defaultWording?: Wording;
  supported?: readonly Wording[];
  storage?: WordingStorage;
}

export interface UseWording {
  wording: Readonly<Ref<Wording>>;
  setWording(next: Wording): void;
}

export function useWording(options?: UseWordingOptions): UseWording;
export const wording: Readonly<Ref<Wording>>;
export function setWording(next: Wording): void;
```

**Guarantees**
- The module singleton defaults to `plain` when `topdata.wording.v1` is absent,
  invalid, or unreadable.
- `setWording('git')` writes exactly `topdata.wording.v1 = "git"`;
  `setWording('plain')` writes `"plain"`.
- Storage read/write failures are swallowed; they never throw into startup.
- Unsupported values passed to `setWording()` are ignored with a development
  warning and do not overwrite the stored value.
- The factory is available for tests; the app uses the singleton so every
  component sees the same ref.
- `wording` is export-compatible with `Readonly<Ref<Wording>>` and is read inside
  `t()`, so Vue templates/computeds track changes.

## 2. Git overlay — `src/lib/i18n/gitWording.ts`

```ts
import type { Language } from './index';
import type { MessageKey } from './messages.en';

export type GitWording = Partial<Record<MessageKey, string>>;

export const gitWording: Record<Language, GitWording>;
```

**Guarantees**
- Both `en` and `de` maps contain the same key set (parity test).
- Every key exists in the English plain catalogue; the type prevents typos.
- Every value is a non-empty short label, never an explanation.
- Values are identical in both languages and use canonical English Git words,
  as required by the spec clarification.
- Keys not listed in section 4 are absent. Explanations, hints, errors and the
  Git log are never overlaid.

## 3. Resolution and hint helpers — `src/lib/i18n/index.ts`

```ts
export function t(key: MessageKey, params?: MessageParams): string;

export type GitTerm =
  | 'commit'
  | 'fetch'
  | 'pull'
  | 'push'
  | 'stash'
  | 'stash pop';

export function operationHint(key: MessageKey, term: GitTerm): string;
```

**`t()` contract**
- Resolution order in Git mode:
  `gitWording[language][key]` (if a non-blank string) →
  `catalogues[language][key]` → `catalogues.en[key]`.
- Resolution order in plain mode: `catalogues[language][key]` →
  `catalogues.en[key]`.
- Parameters are interpolated after the template is selected.
- A missing overlay entry, a non-string overlay entry, or a blank/whitespace
  overlay entry falls back to the plain label (FR-010).
- A missing plain key returns the key and warns in development, as today.
- The function stays synchronous and dependency-free beyond the i18n modules.

**`operationHint()` contract**
- In Git mode: returns `t(key)` unchanged.
- In plain mode: returns `` `${t(key)} (git ${term})` ``.
- It never changes the base hint text, never overlays the hint key, and never
  appends a suffix in Git mode.
- `term` is one of the canonical union values above; call sites cannot pass an
  arbitrary command string.

## 4. Overlay key and value contract (47 keys per language)

All values below are identical in `gitWording.en` and `gitWording.de`.

| Key | Git value |
|---|---|
| `toolbar.saveToGit` | `Commit` |
| `toolbar.getLatest` | `Pull` |
| `toolbar.sendChanges` | `Push` |
| `toolbar.checkForUpdates` | `Fetch` |
| `toolbar.setAside` | `Stash` |
| `toolbar.bringBack` | `Stash pop` |
| `table.save` | `Commit` |
| `table.getLatestFor` | `Pull for {name}` |
| `table.sendFor` | `Push for {name}` |
| `status.changesNotSaved` | `Uncommitted changes` |
| `status.updatesAvailable` | `{count} behind` |
| `status.toSend` | `{count} ahead` |
| `filters.changesNotSaved` | `Uncommitted changes` |
| `filters.updatesAvailable` | `Behind` |
| `filters.toSend` | `Ahead` |
| `save.title` | `Commit` |
| `save.submit` | `Commit` |
| `save.savedTitle` | `Committed` |
| `help.word.save.name` | `Commit` |
| `help.word.pull.name` | `Pull` |
| `help.word.push.name` | `Push` |
| `help.word.stash.name` | `Stash` |
| `onboarding.ready.saveTitle` | `Commit` |
| `onboarding.ready.pullTitle` | `Pull` |
| `onboarding.ready.pushTitle` | `Push` |
| `action.fetch` | `Fetch` |
| `action.pull` | `Pull` |
| `action.push` | `Push` |
| `action.stash` | `Stash` |
| `action.stashPop` | `Stash pop` |
| `insight.dirty.actionOne` | `Commit` |
| `insight.dirty.actionOther` | `Commit all` |
| `toast.fetchDone` | `Fetch complete` |
| `toast.fetchFailed` | `Fetch failed` |
| `toast.fetchFailure` | `Fetch` |
| `toast.pullDone` | `Pull complete` |
| `toast.pullFailed` | `Pull failed` |
| `toast.pullFailure` | `Pull` |
| `toast.pushDone` | `Push complete` |
| `toast.pushFailed` | `Push failed` |
| `toast.pushFailure` | `Push` |
| `toast.stashDone` | `Stash complete` |
| `toast.stashFailed` | `Stash failed` |
| `toast.stashFailure` | `Stash` |
| `toast.stashPopDone` | `Stash pop complete` |
| `toast.stashPopFailed` | `Stash pop failed` |
| `toast.stashPopFailure` | `Stash pop` |

**Reused keys that must NOT be overlaid**: every `*Hint`/`*description`/`*Text`
key, `save.description`, `help.step3.*`, `help.step4.*`, `help.description`,
`save.status.*`, `review.status.*`, `save.ai.*`, all validation/error keys,
`gitLog.*`, `toolbar.whatGitDid*`, `group.saveChanges`, and
`toast.groupSaved`/`toast.saveGroupFailed`.

## 5. `WordingSwitcher.vue` component contract

- Renders one compact menu trigger next to `TdLanguageSwitcher`; it does not
  replace or modify the kit component.
- Trigger shows the short label for the active wording.
- Trigger `aria-label`/`title` = localized “Change wording” + `": "` + the full
  active label (for example `Change wording: Plain language`).
- Trigger has `aria-haspopup="menu"` and `aria-expanded`.
- Menu contains exactly two options in the current language:
  “Plain language” and “Git terminology”.
- The active option shows a visible check; a visually hidden localized
  “current” word is included for assistive technology.
- Selecting an option calls `setWording()` and closes the menu. It never reloads
  the app and never writes the language key.
- Keyboard: the trigger is reachable by Tab; the menu opens with Enter/Space;
  options are reachable with arrow keys/Enter via the existing `TdMenu`; Escape
  closes.
- New copy keys (both languages):
  `sidebar.changeWording`, `wording.plain`, `wording.git`,
  `wording.plainShort`, `wording.gitShort`, `wording.current`.

## 6. `OperationHint.vue` component contract

- Wraps exactly one focusable control via its default slot.
- Slot props: `describedBy` (string id) and `title` (string hint).
- The wrapped control must set `:aria-describedby="describedBy"` and
  `:title="title"` (or equivalent).
- Tooltip text is the value passed by the caller, normally
  `operationHint(key, term)`.
- The tooltip is visible on pointer hover and on keyboard focus
  (`focusin`/`focus-within`), and is hidden on leave/focusout/Escape.
- An always-present visually hidden description element carries the id
  referenced by `aria-describedby`, so assistive technology can resolve the
  description at the moment focus lands, even before the visual panel opens.
- The visible tooltip is a separate, `aria-hidden="true"` presentation
  teleported (via the kit's exported `usePopover`) so table and toolbar overflow
  do not clip it. It is shown on the same hover/focus events.
- In Git mode the tooltip still shows the base hint but no `(git …)` suffix.
- Applied to: toolbar Save/Get latest/Send changes buttons and table row
  Save/Get latest/Send changes buttons. Menu items use their existing
  description slot (`operationHint(...)`) instead.

## 7. Hint suffix table

| Control | Hint key | `GitTerm` |
|---|---|---|
| Toolbar Save | `toolbar.saveHint` | `commit` |
| Toolbar Get latest | `toolbar.getLatestHint` | `pull` |
| Toolbar Send changes | `toolbar.sendHint` | `push` |
| Toolbar Check for updates (menu) | `toolbar.checkForUpdatesHint` | `fetch` |
| Toolbar Set aside changes (menu) | `toolbar.setAsideHint` | `stash` |
| Toolbar Bring back changes (menu) | `toolbar.bringBackHint` | `stash pop` |
| Table Save | `table.saveHint` | `commit` |
| Table Get latest | `table.getLatestForHint` | `pull` |
| Table Send changes | `table.sendForHint` | `push` |

Example plain value:
`Save changes in this project to Git (git commit)`.
Same key in Git mode: `Save changes in this project to Git` (unchanged).

## 8. Non-goals / contract boundaries

- No change to `src-tauri/`, `AppConfig`, axum routes, typed events, or
  `backend.ts`.
- No change to `@topdata-software-gmbh/ui-kit-web-vue3` or its components.
- No change to Git output, commit contents/messages, branch names, file names,
  or data sent to repositories.
- No wording-aware toast history: a toast already on screen keeps the text it
  was created with; the next operation uses the active wording.
- No automatic detection of the user's experience level and no persona/verbosity
  setting.
