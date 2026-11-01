# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- AI commit-message drafts in **Save to Git**: one Conventional Commit suggestion
  per selected project, streamed live (`commitgen:repo` / SSE `commitgen-repo`),
  editable per row, with isolated per-project failures and single-row retry.
  Generation only reads repositories; nothing is committed until you save.
- Optional `ai` configuration block (`baseUrl`, `model`, `apiKeyEnv`,
  `maxToolCalls`, `diffBudgetChars`). The key is read from the named environment
  variable and never persisted, logged, or sent to the browser.
- Bounded read-only inspection tools (`read_file`, `file_at_head`,
  `recent_history`, `grep`) sandboxed to each repository, with a per-run lookup
  budget.
- `generate_commit_messages` Tauri command and
  `POST /api/repositories/generate-commit-messages` (202 + `runId`), backed by
  the git-pinned `serdes-ai` framework confined to `src-tauri/src/commitgen/`.
- German UI with an English fallback and a sidebar language switcher. The choice
  is stored under `topdata.language.v1` and applied to `<html lang>`.
- App-owned typed translation catalogues (`src/lib/i18n/`) with `t()` and
  `tPlural()`.
- Shared `useLanguage` composable and `TdLanguageSwitcher` component added to
  `@topdata-software-gmbh/ui-kit-web-vue3`.
- Resumable first-run setup wizard with deep folder scanning, optional remote
  owner pre-selection, editable group suggestions, and a ready screen for the
  Save/Get latest/Send workflow.
- Virtual **All repos** page with status, group, type, and search filters, plus
  row-level actions.
- Repository kind detection for themes, plugins, and other repositories.
- `save_onboarding_state` Tauri command and `POST /api/onboarding` endpoint.
- Shared `@topdata-software-gmbh/ui-kit-web-vue3` and
  `@topdata-software-gmbh/design-tokens` dependencies, token/kit CSS imports, and
  a Vitest migration smoke test.
- "What Git did" drawer: a session-only, plain-language transcript of the commands
  behind each Git action, with gray `#` step comments, `git -C` rendering,
  skipped-step markers, and oh-my-zsh alias hints.
- Git actions are declared once in `src-tauri/macros/git-actions.yaml` and executed
  from that template, so the log wording and the behaviour share one source.
- Refresh button feedback: one success or error toast per manual refresh click,
  naming the project and summarizing its branch and status in German and English.
  Background refreshes stay silent.

### Changed

- The Save dialog now commits each project with its own message instead of one
  shared message; typing manually behaves exactly as before.
- Renamed the project and CLI from `git-repo-manager` to `repoman`: the installed
  command and Rust package are now `repoman`, the desktop product is `RepoMan`
  (`com.topdata.repoman`), configuration moved to `~/.config/repoman/config.json`,
  environment variables to `REPOMAN_*`, and browser storage keys to `repoman.*`.
  The legacy `GIT_REPO_MANAGER_*` variables are still honoured and an existing
  `git-repo-manager` config directory is copied to the new location on first launch.
- Migrated every screen, modal, onboarding step, table, and helper to the shared
  web UI kit and token-backed utilities. The app no longer ships a second
  (slate/cyan) palette.
- Added `TdSelect`, `TdTextarea`, `TdCheckbox`, `TdBadge`, `TdAlert`,
  `TdSpinner`, `TdMenu`, `TdMenuItem`, and `TdSegmentedControl` usage across the
  toolbar, filters, forms, empty states, and guidance banner.
- Group colours are now stored as semantic token names; existing hex values keep
  rendering through a legacy fallback.
- The sidebar's "All projects" entry is now "All repos" and opens a dedicated
  browse page; group workspaces keep the bulk action toolbar.
- `AddScanPathModal` and `ToastStack` now use the shared design-system
  components and `--td-*` tokens instead of hand-rolled Tailwind markup.
- Success and info toasts expire after 5 seconds and at most 3 are visible;
  error toasts persist until dismissed. Previously errors auto-dismissed after
  9 seconds and the stack was unbounded.
- Vite and the Tauri dev URL now use reserved port 5183 from `port-map` instead
  of the shared 5173 default.
- The application event bus now carries typed `AppEvent` values; repository-change
  notifications and Git step events share one SSE endpoint (`/api/events`) and one
  frontend subscription.

## [0.1.0] - 2026-10-28

### Added

- Tauri v2 desktop application with a Vue 3, TypeScript, Tailwind CSS, and Pinia UI.
- `scripts/install.sh` / `pnpm install:bin` to build and install the release
  binary on `PATH`, with `--prefix`, `--symlink`, and `--no-build` options.
- Recursive multi-repository discovery with `.gitignore` awareness, dependency
  folder pruning, configurable depth, enable/disable, and exclusion patterns.
- Repository groups with colors, repository assignment, synchronous branch
  switching, auto-stash, and a default base branch setting.
- Live Git state updates from `.git` filesystem events, including worktree and
  submodule `gitdir` support and an automatic polling fallback when the OS
  inotify watch limit is exhausted.
- Guided **Save to Git** flow with changed-file preview and one commit message per
  selected project, available from the toolbar, guidance banner, and each row.
- Tauri commands and HTTP endpoints for listing changes and committing:
  `list_repo_changes`, `commit_repo_changes`,
  `POST /api/repositories/changes`, and `POST /api/repositories/commit`.
- Plain-language UI: next-step guidance banner, help/glossary modal, friendly
  status badges, `Get latest` / `Send changes` / `Working on` labels, and guided
  empty states.
- Friendly translations for common Git failures (missing upstream, rejected push,
  authentication, conflicts, locked index).
- Pull sets aside and restores unfinished changes; push creates the upstream branch
  when one is missing; stash includes untracked files.
- Scan paths expand `~` and are validated before being saved.
- Parallel fetch, fast-forward pull, push, stash, and stash pop across a selection
  or group, bounded by a concurrency semaphore.
- Repository status matrix with branch, ahead/behind counts, clean/dirty state,
  conflicts, remotes, and merge/rebase/cherry-pick detection.
- Atomic JSON configuration with a platform config directory and `--config`
  override.
- Browser/web-server mode using axum: same UI and API over HTTP, server-sent
  events for live updates, optional bearer-token authentication, static frontend
  serving, and CLI/environment configuration.
- `git-repo-manager --web` with `--host`, `--port`, `--token`, `--config`, and
  `--dist` options; non-loopback binds without a token generate one automatically.
- Rust unit tests for configuration persistence, repository scanning, Git state,
  autostash checkout, change listing/committing, friendly error mapping, and
  worktree path resolution.
- Project documentation covering desktop development, web mode, HTTP API,
  configuration, architecture, and the everyday save/get-latest/send workflow.

### Fixed

- Removed the unused `tauri-plugin-shell` dependency that blocked release builds
  with a Rust/npm version mismatch.
- Changed the bundle identifier to avoid the macOS `.app` suffix conflict warning.
- The first line of `git status --porcelain` lost its leading status column when
  the output was trimmed, which shifted the reported path by one character and
  could hide a modified file.
