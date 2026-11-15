# Quickstart Validation: Read-Only Change Viewer

Use this guide to prove the feature end-to-end on a local machine. It validates
the user scenarios, the hard bounds, the read-only guarantee, and the
keyboard/i18n contract. Implementation details live in `plan.md`, `data-model.md`,
and `contracts/`; this file is only the run/validation guide.

## Prerequisites

- Rust 1.80+ with Cargo, Node.js 20+, pnpm 9+, and Git on `PATH`.
- The linked TopData design system is built once:
  `(cd ../topdata-design-system && pnpm -r build)`.
- Install dependencies in the repository: `pnpm install`.

## Run the app

Desktop:

```bash
pnpm tauri dev
```

Browser (shared web server + Vite):

```bash
./scripts/dev-up.sh
# open http://localhost:5183
```

## Automated gates

Run these before declaring the feature validated:

```bash
pnpm typecheck
pnpm test
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all pass. The vitest run includes the token-only styling guard, so any
raw palette class or hard-coded hex colour in the new components fails the gate.

## Prepare a throwaway project

Create a small repository outside the app's real projects so the review can be
discarded safely:

```bash
mkdir -p /tmp/review-demo/theme/assets
cd /tmp/review-demo/theme
git init -q
git config user.email demo@example.com
git config user.name Demo
printf 'body { color: black; }\n' > style.css
printf 'first line\nsecond line\nthird line\n' > app.js
printf 'old\n' > notes.txt
printf 'to be deleted\n' > remove-me.txt
git add . && git commit -qm "initial"
```

Make one of each reviewable change:

```bash
printf 'body { color: rebeccapurple; }\n' > style.css          # modified
printf 'brand new\n' > new.txt                                 # untracked / new
git rm -q remove-me.txt                                        # deleted
git mv notes.txt renamed.txt                                   # renamed
printf 'first line\nsecond line changed\nthird line\n' > app.js # line change
printf 'spacing only\n' >> app.js                              # extra change for whitespace test later
```

Add it as a scan path in the app (or reuse an existing folder that contains it)
and refresh the project list.

## Scenario 1 — Review one project's unsaved changes (US1, P1)

1. On the project's row, choose **Änderungen ansehen**.
2. Expect the project name, file count, added/removed totals, and plain-language
   labels: *Geändert* (style.css, app.js), *Neu* (new.txt), *Gelöscht*
   (remove-me.txt), *Umbenannt* (renamed.txt). No `M`, `A`, `D`, `R`, `@@`,
   `HEAD`, or hash appears.
3. Select `style.css`: expect side-by-side previous/current content, changed
   lines tinted on both sides, and unchanged stretches collapsed.
4. Expand a collapsed stretch: expect its unchanged lines to appear.
5. Select `remove-me.txt`: expect only the previous side with the *Gelöscht*
   label. Select `new.txt`: expect only the current side with *Neu*.
6. Close with **Fertig** and again with **Escape** (reopen between attempts).
7. Verify nothing changed:

```bash
cd /tmp/review-demo/theme && git status --porcelain
```

Expected: the same change list as before the review; no new commit, no staging
change, no discarded file.

## Scenario 2 — Review several projects from Save Changes (US2, P1)

1. Create or select at least three projects with different changes.
2. Open **Save to Git**, select the three projects, then choose **Überprüfen**.
3. Expect the viewer to start on the first project and show *"Projekt 1 von 3"*.
4. Move next/previous: expect exactly one action per hop, the indicator to
   update, and each project's own files/summary.
5. Select a different file in project 2, move away, then return to project 2.
   Expected: the previously selected file is still selected and its preview is
   reused.
6. Make one selected project unreadable (rename its folder temporarily), reopen
   review, and move to it. Expected: only that project shows a plain-language
   error naming it; the other projects remain fully reviewable.

## Scenario 3 — Review the last saved state (US3, P2)

1. Save one project through **Save to Git** with a message.
2. Open **What Git did**, find the *Saved changes* run, and choose **Ansehen**.
3. Expected: the viewer opens in saved-state mode with exactly the files and
   differences included in that save, read-only; the drawer is still behind the
   modal and returns unchanged when the viewer closes.
4. Make a second save of the same project, then use **Ansehen** on the first
   entry. Expected: it shows the first save (the captured commit), not the second.
5. Unavailable case (throwaway repo only):

```bash
cd /tmp/review-demo/theme
git reset --hard HEAD~1
git reflog expire --expire=now --all
git gc --prune=now
```

Reopen the activity entry. Expected: a plain-language "saved state is no longer
available" notice, not an error dialog or a blank panel.

## Scenario 4 — Images and non-text changes (US4, P2)

1. Commit a small PNG, then change it. Select the PNG in the viewer.
2. Expected: previous/current images side by side with pixel dimensions and file
   sizes, both within the 10 MB / 8,000×8,000 px bounds.
3. Add a file larger than 10 MB or an image wider than 8,000 px (throwaway
   project only). Select it. Expected: a plain-language too-large notice, no
   broken image and no endless load.
4. Change a binary file (for example a `.zip` or `.dll`). Select it. Expected:
   *"Binärdatei geändert"* plus the size change and no preview.
5. Corrupt an image (truncate it) and select it. Expected: a plain description,
   not a broken preview.

## Scenario 5 — Technical view (US5, P3)

1. Open any text change. Switch to the unified view, then the raw technical view,
   then back to side-by-side.
2. Expected: the same change is represented in each form; raw text may contain
   `@@`/hashes, but the default view never does. The chosen mode is preserved
   when changing files for the rest of the session.

## Edge-case checks

- **Empty project**: clean project → review shows the *nothing to review* empty
  state, not an error.
- **Whitespace-only change**: make a change that only adds/removes spaces; the
  file is shown fully with *"Nur Leerzeichen geändert"*.
- **Conflict**: start a merge that conflicts (or fabricate an unmerged path) and
  open review during the operation. Expected: *Konflikt* label; the file remains
  viewable and the viewer never blocks. An unmerged path outside an in-progress
  merge/rebase/cherry-pick must not claim the *Konflikt* label.
- **Very large text file**: generate a >2 MB or >5,000-diff-line file (throwaway
  project only). Expected: the first bounded portion plus the rest-not-displayed
  notice; the viewer never appears to hang.
- **Changes while open**: while the viewer shows a project, modify one of its
  files from a terminal. Expected: the displayed comparison does not jump; a
  translated *"Änderungen haben sich geändert · Aktualisieren"* prompt appears.
  Choosing it refreshes on demand.
- **Many files**: a project with more than 50 changed files opens without a hard
  cap; the first file is still visible within the 2-second target on local disk,
  and each preview respects the same bounds.

## Accessibility and language

- Keyboard only: Tab reaches the project navigation, file list, fold controls,
  mode switch, refresh prompt, and *Fertig*; the file list supports Arrow/Home/End
  and Enter/Space; focus is always visible.
- Screen reader: the dialog, file list, project position (*"Projekt N von M"*),
  fold buttons (`aria-expanded`), and file status all have accessible names/roles.
- Switch the app language in the sidebar and reopen the viewer: every viewer
  string appears in both German and English, with no missing-key fallbacks.

## Read-only proof

After every scenario, run:

```bash
cd /tmp/review-demo/theme && git status --porcelain && git log -1 --format=%H
```

Expected: the working-tree change list and `HEAD` are identical to before the
review. A review of any kind performs zero writes (SC-005).
