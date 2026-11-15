# Goals

## What RepoMan is for

RepoMan is a small development utility for **looking at what changed in your
repositories**. It replaces the heavyweight JetBrains tooling that many of us
used to keep open all day, at a time when we no longer write much code — we
mostly review Git diffs, check repository state, and save or send changes.

The primary job is the one JetBrains did well: open a project, see the changed
files, and read a clear before/after comparison. Everything else in RepoMan
exists to get you to that screen quickly and safely.

## Primary goals

1. **Make reviewing changes the fastest path.** Open any discovered repository,
   see its uncommitted files in plain language (*New*, *Changed*, *Deleted*,
   *Renamed*, *Conflict*), and compare the previous and current version side by
   side with syntax coloring and image previews.
2. **Replace the IDE for the view-only workflow.** No editor, no terminal, no
   debugger — just repository discovery, status, diffs, and a few guided Git
   actions (save, fetch, pull, push, stash).
3. **Stay simple to use.** Plain-language labels instead of Git jargon, guided
   flows, friendly errors that name the next action, and no destructive controls
   in the viewer. Looking at a change never modifies a file.
4. **Work the same on macOS as on Linux.** RepoMan runs as a desktop app and as
   a browser app, so a Mac user can review repositories that live on another
   machine without installing a toolchain there.

## Who it is for

- Web designers, shop operators, and developers who *review* changes more than
  they write them.
- People who find Git intimidating and want guidance instead of commands.
- macOS users who want the same read-only diff workflow while the repositories
  sit on a build or server machine.
- Teams that want a shared, lightweight alternative to a full IDE.

## Non-goals

- **Not an editor or IDE.** No code writing, refactoring, terminal, or debugger.
- **Not a Git power tool.** No rebase surgery, history rewriting, or advanced
  branch management.
- **Not a hosted service.** RepoMan runs locally and keeps configuration in a
  plain JSON file — no database, no cloud account.
- **Not a replacement for the Git CLI.** It drives the real `git` you already
  have, preserving your SSH agent, credential helpers, and signing setup.

## Simplicity is a requirement

- One window, one obvious action at each step.
- Every Git concept shown in plain words; the **What Git did** drawer exposes
  the real commands only when you ask for them.
- First run is a short setup wizard; later runs resume where you left off.
- Previews are bounded (first 2 MB or 5,000 diff lines per file) so large
  changes open quickly instead of freezing the app.

## macOS support

- **Browser mode** serves the same Vue UI from `repoman server`; open it in
  Safari or Chrome on a Mac while the repositories stay on another machine.
- **Desktop mode** builds for macOS with the Xcode Command Line Tools.
- Install and start stay short: `./scripts/install.sh`, then `pnpm start`, or
  `./scripts/dev-up.sh` for browser development.

## How we judge success

- A reviewer can see a project's pending changes in seconds, without opening an
  IDE.
- The diff viewer is understandable to someone who does not know Git.
- Nothing in the review path can accidentally change, stage, or revert a file.
- The same workflow is available on Linux and macOS, desktop and browser.
- The app stays a single lightweight binary plus a JSON config.
