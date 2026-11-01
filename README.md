# RepoMan

A lightweight, beginner-friendly multi-repository Git GUI (formerly
`git-repo-manager`). Discover repositories below configured folders, group them
(for example "Shop themes"), watch their Git
state, and save, fetch, pull, push, or stash across a whole group at once — with
plain-language guidance instead of Git jargon.

The core is written in Rust and shared by two frontends:

- **Desktop** — Tauri v2 window with a Vue 3 UI.
- **Web** — an axum HTTP/SSE server that serves the same Vue UI to any browser,
  so a colleague can use it from macOS while the repositories live on another
  machine.

Git itself is always executed through the system `git` CLI, preserving the
user's SSH agent, credential helpers, and GPG signing configuration.

## Features

- Recursive repository discovery with `.gitignore` awareness and dependency
  folder pruning (`node_modules`, `vendor`, `target`, caches).
- Resumable first-run setup wizard: scan one folder deeply, optionally pre-select
  projects by remote owner name, confirm discovered repositories, and keep
  editable groups suggested from the folder layout.
- Virtual **All repos** page with status, group, repository-type, and search
  filters. It always reflects every discovered repository and offers row-level
  actions only; bulk actions stay inside curated groups.
- Multiple scan paths with per-path depth, enable/disable, and exclusions.
- Logical groups with repository assignment, colors, synchronous branch
  switching, auto-stash on switch, and a configurable base branch.
- Live repository state via `.git` filesystem events (with automatic polling
  fallback when the operating system's watch limit is exhausted).
- Guided **Save to Git** flow: preview changed files, write one plain-language
  message, and commit each dirty project in the selection.
- Pull automatically sets aside unfinished changes and restores them afterwards;
  push creates the upstream branch when needed.
- Friendly error messages translate common Git failures into the next action.
- Parallel group actions bounded by a concurrency semaphore (fetch, fast-forward
  pull, push, stash, stash pop).
- Status matrix with plain-language badges, branch, ahead/behind, conflicts, and
  in-progress operation detection (merge, rebase, cherry-pick).
- Repository type detection for Shopware themes, plugins, and other projects,
  based only on local metadata.
- Built-in help modal, next-step guidance banner, and friendly empty states.
- **What Git did** drawer that explains each multi-step action as a commented
  shell transcript, with gray `#` step comments, `git -C` commands, skipped-step
  markers, and oh-my-zsh alias hints. Git actions themselves are declared in
  `src-tauri/macros/git-actions.yaml`.
- JSON configuration persisted atomically; no database.
- Browser mode with optional bearer-token protection and server-sent events.

## Everyday workflow

1. On first run, the setup wizard asks for the folder where your projects live,
   scans it deeply, and proposes groups for you to confirm.
2. Edit your files normally in your editor. Use **All repos** to browse and filter
   everything; open a group when you want to act on several projects at once.
3. Click **Save to Git** at a checkpoint and describe what changed. You can write
   the message yourself, or click **✨ Suggest with AI** to draft one message per
   project, review and edit each row, and then save.
4. Click **Get latest** before starting work and **Send changes** when finished.
5. Use **How this works** in the app for a plain-English glossary and recovery
   guidance.

Setup can be reopened from the sidebar at any time. If you close it partway
through, the current step, folder, owner names, and selection are kept on this
computer and resumed next time.

The group toolbar wires the common actions; fetch, stash, and stash pop live under
the **More** menu. The **All repos** page has no bulk actions.

### AI commit messages

**Save to Git** can draft one Conventional Commit message per selected project
from that project's own pending changes. Drafts appear as editable rows, one per
project, and nothing is committed until you click **Save to Git** — generation
only reads the repository.

The AI is optional. Without an `ai` block in the configuration the button stays
disabled and the dialog works exactly as before.

```json
"ai": {
  "baseUrl": "http://localhost:11434/v1",
  "model": "gpt-4o-mini",
  "apiKeyEnv": "OPENAI_API_KEY",
  "maxToolCalls": 12,
  "diffBudgetChars": 40000
}
```

- `baseUrl` / `model` point at any OpenAI-compatible endpoint.
- `apiKeyEnv` is the **name** of the environment variable that holds the key; the
  key itself is never written to the config, an event, or a log line. In web
  server mode the variable is read from the **repoman server process**, not the
  browser.
- `maxToolCalls` bounds read-only repository lookups per project (files, HEAD
  content, recent subjects, text search); the generator answers from the change
  detail when the budget is spent.
- `diffBudgetChars` caps the change detail sent to the model; larger change sets
  are truncated instead of failing.

Failures are per project: an unreachable endpoint, a missing key, an empty
answer, or merge conflicts only mark that row, with a retry button (except for
"nothing to save" and merge conflicts, which need a human first). Typing your own
message into a failed row always works.

### What Git did

Open the "…" menu in a group toolbar and choose **What Git did** to see the exact
commands behind the last actions. Each action is one card; expand it to read the
plain-language `#` comments and the real `git -C <project> …` commands, including
the oh-my-zsh alias you could have typed instead. Nothing is stored: the list is
kept for the current session only.

The commands themselves are declared in `src-tauri/macros/git-actions.yaml`, so
the explanation and the behaviour cannot drift apart.

## Requirements

- Rust 1.78+ and Cargo
- Node.js 20+ and pnpm 9+
- Git on `PATH`
- Linux desktop builds additionally need the Tauri system dependencies
  (`webkit2gtk-4.1`, GTK 3, and related libraries)
- macOS desktop builds need Xcode Command Line Tools

## Install and develop

```bash
pnpm install
pnpm tauri dev          # desktop window + Vite dev server
pnpm start              # launch the installed app (see below)
pnpm typecheck          # vue-tsc
pnpm build              # type-check and build dist/
```

For browser development, `./scripts/dev-up.sh` builds the linked design-system
packages and opens two mprocs panes: the Rust web API on `127.0.0.1:8787` and
Vite on `localhost:5183`. Open <http://localhost:5183>; press `q` in mprocs to
stop both. Pass `--skip-kit-build` to skip the design-system build, or `--help`
for details. It requires `pnpm`, `cargo`, and `mprocs`.

`pnpm start` (or `./scripts/start.sh`) launches the installed
`repoman` command, falling back to `~/.local/bin`. It does not build;
run `pnpm install:bin` first. Any extra arguments are forwarded to the app, so
`pnpm start -- --web --port 8788` starts web mode.

## Design system

The Vue UI uses the shared TopData design system from
`../topdata-design-system`: `@topdata-software-gmbh/ui-kit-web-vue3` for components and
`@topdata-software-gmbh/design-tokens` for the generated `--td-*` CSS variables.
`src/main.ts` imports the token CSS and the kit styles before the app stylesheet.

Every screen uses kit components (`TdButton`, `TdInput`, `TdFormField`, `TdSelect`,
`TdTextarea`, `TdCheckbox`, `TdBadge`, `TdAlert`, `TdSpinner`, `TdMenu`,
`TdSegmentedControl`, `TdModal`, `TdToastStack`) plus Tailwind utilities backed by
the token preset. App templates and helpers must not use raw palette classes
(`slate-*`, `cyan-*`, `rose-*`, …) or hard-coded hex colours; `src/lib/__tests__/token-only.test.ts`
enforces this. Group colours are stored as semantic token names, with a legacy hex
fallback for existing config files.

This checkout currently links both packages from the sibling working tree. Build
them once before starting the app:

```bash
(cd ../topdata-design-system && pnpm -r build)
pnpm dev
```

Run the Vue tests with `pnpm run test`.

Run the Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

Build a desktop installer/bundle:

```bash
pnpm tauri build
```

### Install a command on your PATH

Build a release binary and install it as `repoman`:

```bash
pnpm install:bin
```

The default target is `~/.local/bin`. To install elsewhere, or to symlink the
release binary for live rebuilds:

```bash
./scripts/install.sh --prefix /usr/local/bin
./scripts/install.sh --symlink
```

Then start the desktop app with:

```bash
repoman
```

For browser mode from this checkout, point the server at the built assets:

```bash
repoman --web --port 8787 --dist ./dist
```

On first launch the main panel shows **Let's find your projects**: add the folder
where your repositories live, then click **Find my projects**.

## Languages

The UI ships in German (default) and English. The chosen language is stored under
`topdata.language.v1` in `localStorage` and applied to `<html lang>`; switch it from
the sidebar footer next to the theme toggle. The translation runtime lives in
`src/lib/i18n/`: `messages.en.ts` is the source of truth and `messages.de.ts` is
typed against it, so a missing German key fails `pnpm typecheck`. Add user-facing
copy there and call `t('namespace.key')` rather than writing literals.

The Rust backend and the "What Git did" transcript from
`src-tauri/macros/git-actions.yaml` stay English in this pass.

## Web server mode

Build the frontend once, then start the binary in web mode:

```bash
pnpm build
cargo run --manifest-path src-tauri/Cargo.toml -- \
  --web --host 0.0.0.0 --port 8787 --token "$(openssl rand -hex 16)"
```

Or use the already-built binary:

```bash
./src-tauri/target/release/repoman --web --host 0.0.0.0 --port 8787
```

Open the printed URL, for example:

```text
http://127.0.0.1:8787/?token=<token>
```

The token is copied into `sessionStorage` and removed from the address bar. API
requests use `Authorization: Bearer <token>`; the SSE endpoint accepts the token
as a query parameter because `EventSource` cannot set headers.

If the bind address is not loopback and no `--token` is supplied, a random token
is generated and printed. Binding to `0.0.0.0` without a token is therefore safe
by default. Static UI files are public, but every `/api/*` request is protected.

### Options and environment variables

| Flag | Environment variable | Default |
| :--- | :--- | :--- |
| `--web` | `REPOMAN_WEB=1` | desktop mode |
| `--host <HOST>` | `REPOMAN_HOST` | `127.0.0.1` |
| `--port <PORT>` | `REPOMAN_PORT` | `8787` |
| `--token <TOKEN>` | `REPOMAN_TOKEN` | auto-generated for non-loopback |
| `--config <PATH>` | `REPOMAN_CONFIG` | platform config directory |
| `--dist <PATH>` | `REPOMAN_DIST` | `dist` |

Use `--help` for the full usage text.

### Accessing from another computer (macOS example)

Run the server on the machine that owns the repositories:

```bash
pnpm build
./src-tauri/target/release/repoman \
  --web --host 0.0.0.0 --port 8787 --token "long-random-token"
```

Then open `http://<server-ip>:8787/?token=long-random-token` in Safari or Chrome
on the Mac. Allow port 8787 through the server firewall and keep the token
private. For a trusted local-only setup, bind to `127.0.0.1` and forward the
port instead:

```bash
ssh -L 8787:127.0.0.1:8787 user@repo-host
# then open http://127.0.0.1:8787/
```

To run natively on the Mac, build the desktop or web binary there with the same
Rust/Node prerequisites:

```bash
pnpm install
pnpm tauri build
# or: pnpm build && cargo build --release --manifest-path src-tauri/Cargo.toml
```

## Configuration

Configuration is stored as JSON:

- Linux/macOS: `~/.config/repoman/config.json`
- Windows: `%APPDATA%\repoman\config.json`
- Override with `--config <path>` or `REPOMAN_CONFIG`.

Installs made before the rename from `git-repo-manager` keep working: the first
launch copies `~/.config/git-repo-manager/config.json` into the `repoman`
directory (the old file is left in place) and the legacy `GIT_REPO_MANAGER_*`
environment variables are still read when no `REPOMAN_*` counterpart is set.

Example:

```json
{
  "scanPaths": [
    {
      "id": "scan-1",
      "path": "/srv/projects",
      "maxDepth": 3,
      "enabled": true,
      "excludePatterns": ["legacy"]
    }
  ],
  "groups": [
    {
      "id": "group-1",
      "name": "Shop Microservices",
      "color": "#38bdf8",
      "repoPaths": ["/srv/projects/api", "/srv/projects/worker"],
      "settings": {
        "syncBranching": true,
        "autoStashOnSwitch": true,
        "defaultBaseBranch": "main"
      }
    }
  ],
  "onboarding": {
    "completed": true,
    "started": true,
    "currentStep": 3,
    "scanPath": "/srv/projects",
    "ownerNames": ["acme"],
    "groupingStyle": "topLevel",
    "selectedRepoPaths": ["/srv/projects/api", "/srv/projects/worker"]
  },
  "ai": {
    "baseUrl": "http://localhost:11434/v1",
    "model": "gpt-4o-mini",
    "apiKeyEnv": "OPENAI_API_KEY",
    "maxToolCalls": 12,
    "diffBudgetChars": 40000
  }
}
```

Older config files without the `onboarding` or `ai` blocks keep working; both
fields are additive. The `ai` block stores only the environment-variable **name**
of the key, never the key itself.

Repository state itself is never persisted; it is read from Git on demand.

## HTTP API

All routes are under `/api` and require the token when one is configured.

| Method | Path | Purpose |
| :--- | :--- | :--- |
| GET | `/api/config` | Read scan paths and groups |
| POST | `/api/scan-paths` | Add or update a scan path |
| DELETE | `/api/scan-paths/{id}` | Remove a scan path |
| POST | `/api/groups` | Add or update a group |
| DELETE | `/api/groups/{id}` | Remove a group |
| POST | `/api/onboarding` | Save resumable setup progress |
| POST | `/api/repositories/discover` | Scan paths and return repository states |
| POST | `/api/repositories/refresh` | Refresh one repository |
| POST | `/api/repositories/changes` | List changed files for one repository |
| POST | `/api/repositories/commit` | Commit all changes in one repository |
| POST | `/api/repositories/actions` | Fetch/pull/push/stash across repositories |
| POST | `/api/repositories/switch-branch` | Switch a set of repositories to a branch |
| POST | `/api/branches` | List local branches with repository counts |
| GET | `/api/events` | Server-sent events (`repo-fs-changed`, `git-step`) |

## Architecture

```text
Vue 3 UI (Pinia store, Tailwind)
        │
        ├── Tauri IPC ──────────────┐
        └── fetch + EventSource ────┤
                                    ▼
                         service.rs (shared logic)
                         ├── config.rs      JSON persistence
                         ├── scanner.rs     discovery via ignore
                         ├── git_runner.rs  git CLI + macro execution
                         ├── macros.rs      YAML action templates
                         ├── git_log.rs     typed step events
                         └── watcher.rs     notify + polling fallback
                                    │
                         EventBus (tokio broadcast, AppEvent)
                         ├── Tauri events: repo:fs-changed, git:step
                         └── SSE events:   repo-fs-changed, git-step
```

Thin command wrappers in `src-tauri/src/commands/` expose the service to Tauri;
`src-tauri/src/web.rs` exposes the same service over HTTP. The frontend chooses
the transport in `src/lib/backend.ts`.

## Project layout

```text
src/                     Vue 3 application
  components/            Toolbar, sidebar, repository table, modals, toasts
  lib/backend.ts         Tauri/web transport abstraction
  stores/repoStore.ts    Application state and actions
  types/domain.ts        Shared TypeScript models
src-tauri/src/           Rust backend
  commands/              Tauri command wrappers
  config.rs              Atomic JSON configuration
  events.rs              Typed application event payload
  git_log.rs             Git step event model and observer seam
  git_runner.rs          Git CLI execution and macro execution
  macros.rs              YAML action template parsing and validation
  scanner.rs             Repository discovery
  service.rs             Logic shared by desktop and web
  watcher.rs             Live updates and polling fallback
  web.rs                 axum server, SSE, token middleware
  main.rs                CLI and application entry point
src-tauri/macros/        Declarative Git action and alias templates
assets/                  Icon source
scripts/install.sh       Build and install the release binary
```

## Security notes

- The web server has no TLS. Use SSH port forwarding or a reverse proxy with TLS
  when exposing it beyond a trusted network.
- Tokens are compared verbatim and are not stored server-side beyond process
  memory. Prefer long random values (the auto-generated token is 64 hex chars).
- The API can run Git operations with the server user's credentials; treat the
  token like an SSH key.
- AI drafts are generated from a bounded, read-only change detail. Lookup tools
  are sandboxed to each project folder; paths outside it are refused. The model
  credential stays in the repoman process environment and never reaches the
  browser, the config file, events, or logs.
