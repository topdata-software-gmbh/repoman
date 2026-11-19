# Quickstart: Personal Wording Preference — Plain Language vs Git Terminology

**Feature**: `005-plain-git-wording` | **Date**: 2026-11-19

This is a validation guide, not an implementation guide. Entity and contract
details live in [data-model.md](./data-model.md) and
[contracts/i18n-wording-contract.md](./contracts/i18n-wording-contract.md).

## Prerequisites

- Node.js and `pnpm` installed.
- Dependencies installed from the repository root:

```bash
pnpm install
```

- For desktop validation: `pnpm tauri dev` (requires the Rust/Tauri toolchain
  and one of the platform prerequisites).
- For browser validation: `./scripts/dev-up.sh`, then open the Vite URL it
  prints (normally `http://localhost:5173`).

The feature is frontend-only; no server restart or config migration is needed
after a code change beyond the normal dev-server reload.

## Automated gates

Run from the repository root:

```bash
pnpm typecheck   # vue-tsc: catalogue keys, overlay types, component props
pnpm test        # vitest: storage, fallback, parity, hints, badges, control
```

Expected: both commands pass. The tests must cover:

- default `plain` with no `topdata.wording.v1` value;
- invalid stored value and throwing storage fall back to plain;
- Git overlay values in both languages;
- a missing/blank Git value falls back to the plain label;
- English/German overlay key parity and catalogue-key existence;
- `operationHint()` adds `(git …)` only in plain mode;
- badges/filters resolve through the same overlay;
- the wording control exposes its current state and switches live.

Optional Rust gates (unchanged by this feature, but part of the repository
quality gate when the whole change is validated):

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets --all-features -- -D warnings && cargo test --all-features
```

## Scenario 1 — First run stays plain

1. Open the app in a fresh browser profile, or clear the wording key in
   DevTools → Application → Local Storage (for the app origin):

   ```js
   localStorage.removeItem('topdata.wording.v1');
   localStorage.removeItem('topdata.language.v1'); // optional: reset language too
   ```

2. Reload the app.
3. Inspect the toolbar, table rows, status badges and the Help modal.

**Expected**
- The wording control is visible in the sidebar footer beside the language
  switcher and shows the plain option.
- Toolbar operations read “Save to Git”, “Get latest”, “Send changes”.
- Row/status labels read plain (for example “Changes not saved”,
  “Updates available”, “To send”).
- No Git jargon appears as an operation label.

## Scenario 2 — Reveal the Git term in plain mode

1. Keep plain wording.
2. Hover the toolbar “Save to Git” button.
3. Tab to the same button with the keyboard.
4. Repeat for “Get latest”, “Send changes”, and the “More Git actions” menu
   entries.

**Expected**
- Hover shows the existing explanation plus the Git term, for example
  `Create a checkpoint of your changed files in Git (git commit)`.
- Keyboard focus reveals the same hint visibly (tooltip or the menu's
  description line), not only on hover.
- A screen reader announces the hint through the control's
  `aria-describedby`; the hint is not only a `title` attribute.
- In plain mode the suffix uses lowercase command spelling: `(git pull)`,
  `(git push)`, `(git fetch)`, `(git stash)`, `(git stash pop)`.

## Scenario 3 — Switch to Git terminology without reloading

1. Open the wording menu in the sidebar footer.
2. Select “Git terminology”.
3. Without reloading, inspect every surface named in the contract:
   toolbar, table row actions, save dialog, status badges/filters, Help modal
   glossary, onboarding “ready” headings, and insight action buttons.
4. Trigger an operation (for example Get latest) and observe the toast.

**Expected**
- Labels update immediately: `Commit`, `Fetch`, `Pull`, `Push`, `Stash`,
  `Stash pop`.
- The save dialog title and submit button read `Commit`.
- The save confirmation toast reads `Committed`.
- An operation toast reads `<Term> complete` / `<Term> failed`; the failure
  summary uses the bare term (for example `Pull`).
- Status badges read `Uncommitted changes`, `{count} behind`, `{count} ahead`;
  the matching filters read `Uncommitted changes`, `Behind`, `Ahead`.
- Glossary names read `Commit`, `Pull`, `Push`, `Stash`.
- Hints still show their plain explanation but no `(git …)` suffix, because the
  label already names the term.
- Explanatory text, hints, error guidance, and the “What Git did” drawer are
  unchanged and still plain.

## Scenario 4 — All four combinations are coherent

For each pair, switch language and wording in either order and inspect the same
surfaces:

| Language | Wording | Expected operation labels |
|---|---|---|
| English | Plain | Save to Git, Get latest, Send changes |
| English | Git | Commit, Pull, Push |
| German | Plain | In Git speichern, Neueste holen, Änderungen senden |
| German | Git | Commit, Pull, Push (canonical English Git words) |

**Expected**
- German plain labels stay German; German Git labels use the canonical English
  Git terms, with German explanations remaining German.
- No empty, truncated, placeholder, or mixed-language operation label appears.
- Switching language does not change the wording choice, and switching wording
  does not change the language.

## Scenario 5 — Persistence and device isolation

1. Choose Git wording.
2. Inspect Local Storage for the app origin.

**Expected**
- `topdata.wording.v1` is `git`.
- `topdata.language.v1` is unchanged by the wording switch.

3. Restart the desktop app or reload the browser tab.

**Expected**
- The app opens with Git wording and the previously selected language.

4. Open the app in a second browser profile or another device/browser.

**Expected**
- That profile has its own wording value and starts in plain if it has none.
- The first profile's choice is not overwritten or shared through project
  configuration.

5. In DevTools, set an invalid value and reload:

   ```js
   localStorage.setItem('topdata.wording.v1', 'shakespeare');
   location.reload();
   ```

**Expected**
- The app opens in plain wording, with no crash or empty label.
- Selecting a valid wording afterwards overwrites the invalid value.

## Scenario 6 — Live change while a dialog or operation is visible

1. Select plain wording.
2. Open the save dialog (“Save to Git”).
3. While the dialog is open, switch to Git terminology.
4. Close the dialog and start “Get latest”; while it runs, switch back to plain.

**Expected**
- The open dialog's title and submit label update to `Commit` immediately.
- The running operation is unaffected; it completes normally.
- The completion toast uses the wording active when it finishes.
- No reload, mixed-register control, or lost input occurs.

## Scenario 7 — Keyboard and assistive-technology pass

1. Use Tab only to reach the wording control in the sidebar footer.
2. Open it with Enter/Space, move through the two options, select one, and press
   Escape on a subsequent open.

**Expected**
- The trigger's accessible name includes “Change wording” and the current
  option (for example `Change wording: Plain language`).
- The menu exposes the two options and marks the active one; the current state
  is announced, not only shown with a check mark.
- The selection changes labels immediately.
- Operation controls expose their hint as an accessible description on focus,
  and the tooltip is visible, not hover-only.

## Scenario 8 — Scope guard

Confirm these do **not** change between plain and Git wording:

- explanatory sentences, hints' base text, error/validation messages;
- “What Git did” drawer and raw command output;
- file-status words such as “New file”, “Deleted”, “Changed”;
- group save/delete labels;
- Git output, commit contents, branch names, file names, and any data sent to
  repositories.

This guards FR-008, FR-011 and FR-014 and prevents the overlay from creeping
into explanatory copy.
