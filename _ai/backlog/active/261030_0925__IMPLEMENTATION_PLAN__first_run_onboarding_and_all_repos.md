---
filename: "_ai/backlog/active/261030_0925__IMPLEMENTATION_PLAN__first_run_onboarding_and_all_repos.md"
title: "First-Run Onboarding Wizard and Filterable All Repos Page"
createdAt: 2026-10-30 09:25
updatedAt: 2026-10-30 09:25
status: draft
priority: high
tags: [onboarding, ux, discovery, groups, filtering, open-source]
estimatedComplexity: complex
documentRevision: 1
sourceSpec: ""
sourceHandoff: ""
sourceAdrs:
  - _ai/technical_decisions/ADR__261028-1__shared-service-tauri-desktop-and-axum-web.md
  - _ai/technical_decisions/ADR__261030-1__generic-resumable-first-run-onboarding.md
  - _ai/technical_decisions/ADR__261030-2__virtual-all-repos-view-and-curated-groups.md
documentType: IMPLEMENTATION_PLAN
---

# First-Run Onboarding Wizard and Filterable All Repos Page

**Sources used:** the approved design discussion in this session (no feature spec
or design handoff file exists), plus ADR 261028-1, ADR 261030-1, and ADR 261030-2.

## Problem

The application currently starts with a guided empty state that asks the user to
add a project folder, then shows one table whose contents change when a group is
selected. That works for small setups, but it has four gaps for the target user
and for a public open-source audience:

1. Nested clones such as
   `<root>/<shop>/vol/www/custom/plugins/<repo>` sit deeper than the default scan
   depth of three, so a first scan can find nothing.
2. Setup choices are not persisted; closing the app during setup loses the
   folder, owner names, and progress.
3. The all-repositories view is the same table used for curated groups, including
   the group toolbar's bulk actions, and it has no status, group, or type filters.
4. Any organization-specific defaults would be wrong for a public repository.
   Owner names, scan depth, and grouping behavior must be local configuration.

## Executive Summary

Add a full-screen, resumable four-step onboarding flow backed by a
backward-compatible `onboarding` block in the JSON configuration. The flow asks
for one project folder, optional remote owner names, and a scan depth, then shows
every discovered repository for explicit acceptance. Matching owner names
pre-select owned repositories. It then proposes editable groups derived from the
folder layout, creates only the groups the user keeps, and ends on a ready screen
that explains **Save to Git**, **Get latest**, and **Send changes** before opening
a group.

Separately, promote the existing all-projects selection into a virtual **All
repos** page. It is always all discovered repositories, cannot be renamed or
deleted, and combines status, group, repository-kind, and search filters. It
offers row-level actions only; bulk actions remain inside curated groups.

The backend gains onboarding persistence, a repository-kind field, and matching
Tauri/HTTP endpoints. Pure frontend modules handle owner parsing, group
suggestions, filtering, and status text so both list views share behavior. Tests
cover the pure logic, backend persistence, kind detection, and filter/status
combinations.

## Design Decisions & Rejected Alternatives

| Decision | Rationale | Rejected alternative |
| --- | --- | --- |
| Store onboarding progress in the local config with `#[serde(default)]` (ADR 261030-1) | Works in desktop and web mode, survives restart, and does not break existing config files | Browser-only local storage; forcing every existing user through onboarding |
| Treat existing non-empty configs as complete | Upgrades must not interrupt current users | Showing the wizard whenever the onboarding block is missing |
| One folder during setup, more later | Keeps the first run focused; the existing scan-path flow already supports multiple roots | Multi-folder management inside the wizard |
| Optional, local remote-owner names | Generic for any public user and useful for pre-selection | Hardcoded organization allowlist; selecting every discovered repo |
| Deep default scan (8 levels) with advanced override | Finds nested clones; advanced users can tune it | Keeping the shallow default of three; progressive auto-scanning |
| Group suggestions from the first folder level, fallback to root name | Matches cloned-shop layouts without assuming a vendor | Hardcoded shop naming; one group per repository |
| Wizard suggestions are renamed/skipped; no repo moves inside the wizard | Keeps the flow light and reuses the existing group editor | A second full group builder |
| All repos is a virtual view, not a persisted group (ADR 261030-2) | Always current; cannot go stale; distinguishes browsing from acting | A real "All repositories" group; automatic membership updates |
| All repos filters combine status, group, kind, and search; row actions only | Safe browsing across 100+ repos | Bulk actions on the filtered set |
| Repository kind from local metadata only | Useful theme/plugin filter without remote or vendor assumptions | Path-name guessing; remote-based type detection |

## Project Environment

- Project root: `/topdata/repo-manager`.
- Desktop: Tauri v2, Rust 2021 (`src-tauri`), Vue 3 + TypeScript + Pinia (`src`).
- Styling: Tailwind utilities plus `@topdata-software-gmbh/ui` and
  `@topdata-software-gmbh/design-tokens`; existing components already use
  `TdButton`, `TdInput`, `TdModal`, and `TdFormField`.
- Tests: Rust unit tests beside modules; Vitest for frontend pure logic and
  component smoke tests.
- Required commands:
  - `cargo fmt --manifest-path src-tauri/Cargo.toml --check`
  - `cargo check --manifest-path src-tauri/Cargo.toml`
  - `cargo test --manifest-path src-tauri/Cargo.toml`
  - `pnpm typecheck`
  - `pnpm test`
  - `pnpm build`
- No database and no migration framework. Configuration is a single atomic JSON
  file (`config.json`) managed by `ConfigManager`.
- The working tree currently contains unrelated modifications in
  `.ctx/ruleman.yaml` and `_ai/conventions.md`. Leave them alone and do not include
  them in this work.
- Work on the current checkout. Do not create feature branches or pull requests.
  Do not commit unless the user explicitly asks.
- The implementation must keep the Tauri and axum transports in sync; every new
  service function needs both wrappers and a typed frontend call.

## Phase 1 — Backend Onboarding Schema and Persistence

**Goal:** Make onboarding state a first-class, backward-compatible configuration
value with validation and both transports.

### Files

- [MODIFY] `src-tauri/src/models.rs`
- [MODIFY] `src-tauri/src/config.rs`
- [MODIFY] `src-tauri/src/service.rs`
- [MODIFY] `src-tauri/src/commands/config_commands.rs`
- [MODIFY] `src-tauri/src/web.rs`
- [MODIFY] `src-tauri/src/main.rs`

### 1.1 Add models

[MODIFY] `src-tauri/src/models.rs`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum GroupingStyle {
    #[default]
    TopLevel,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub started: bool,
    #[serde(default)]
    pub current_step: u8,
    #[serde(default)]
    pub scan_path: Option<String>,
    #[serde(default)]
    pub owner_names: Vec<String>,
    #[serde(default)]
    pub grouping_style: GroupingStyle,
    #[serde(default)]
    pub selected_repo_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub scan_paths: Vec<ScanPath>,
    pub groups: Vec<RepoGroup>,
    #[serde(default)]
    pub onboarding: OnboardingState,
}
```

Keep the existing `AppConfig` defaults for `scan_paths` and `groups`; only the
new field needs a serde default so old config files keep loading.

### 1.2 Persist and validate onboarding

[MODIFY] `src-tauri/src/service.rs`

```rust
use crate::models::{
    AppConfig, BranchInfo, FileChange, OnboardingState, OperationResult, RepoGroup, Repository,
    ScanPath,
};

pub fn save_onboarding_state(
    config: &ConfigManager,
    mut onboarding: OnboardingState,
) -> Result<AppConfig, String> {
    if onboarding.current_step > 3 {
        return Err("Onboarding step must be between 0 and 3.".to_string());
    }

    onboarding.owner_names = onboarding
        .owner_names
        .into_iter()
        .map(|owner| owner.trim().trim_start_matches('@').to_lowercase())
        .filter(|owner| !owner.is_empty())
        .collect();
    onboarding.owner_names.sort();
    onboarding.owner_names.dedup();

    if let Some(path) = onboarding.scan_path.as_mut() {
        *path = expand_home(path);
    }

    {
        let mut cfg = config.config.lock().map_err(|error| error.to_string())?;
        cfg.onboarding = onboarding;
    }

    config.save()?;
    config.snapshot()
}
```

Validation rules:

- `currentStep` must be `0..=3`.
- Owner names are trimmed, a leading `@` is removed, lowercased, deduplicated,
  and empty values are dropped.
- A scan path uses the existing `expand_home` behavior.
- No GitHub/GitLab/Bitbucket host assumptions.

### 1.3 Tests

[MODIFY] `src-tauri/src/config.rs`

Add a test proving an old JSON payload without `onboarding` still loads:

```rust
#[test]
fn loads_legacy_config_without_onboarding() {
    let temp = tempfile::tempdir().unwrap();
    let config_path = temp.path().join("config.json");
    fs::write(&config_path, r#"{"scanPaths":[],"groups":[]}"#).unwrap();

    let manager = ConfigManager::from_path(config_path);
    let snapshot = manager.snapshot().unwrap();

    assert!(!snapshot.onboarding.started);
    assert!(!snapshot.onboarding.completed);
    assert_eq!(snapshot.onboarding.current_step, 0);
    assert_eq!(snapshot.onboarding.owner_names, Vec::<String>::new());
}
```

[MODIFY] `src-tauri/src/service.rs`

Add a `#[cfg(test)]` module or extend the existing one with:

```rust
#[test]
fn normalizes_and_persists_onboarding_state() {
    let temp = tempfile::tempdir().unwrap();
    let manager = ConfigManager::from_path(temp.path().join("config.json"));

    let saved = save_onboarding_state(
        &manager,
        OnboardingState {
            started: true,
            current_step: 2,
            scan_path: Some("/tmp/projects".to_string()),
            owner_names: vec![
                " @Acme ".to_string(),
                "acme".to_string(),
                String::new(),
            ],
            selected_repo_paths: vec!["/tmp/projects/theme".to_string()],
            ..OnboardingState::default()
        },
    )
    .unwrap();

    assert_eq!(saved.onboarding.owner_names, vec!["acme".to_string()]);
    assert_eq!(saved.onboarding.current_step, 2);
}

#[test]
fn rejects_out_of_range_onboarding_step() {
    let temp = tempfile::tempdir().unwrap();
    let manager = ConfigManager::from_path(temp.path().join("config.json"));
    let state = OnboardingState {
        current_step: 4,
        ..OnboardingState::default()
    };

    assert!(save_onboarding_state(&manager, state).is_err());
}
```

### 1.4 Tauri command, HTTP route, and handler

[MODIFY] `src-tauri/src/commands/config_commands.rs`

```rust
use crate::models::{AppConfig, OnboardingState, RepoGroup, ScanPath};

#[tauri::command]
pub fn save_onboarding_state(
    onboarding: OnboardingState,
    state: State<'_, ConfigManager>,
) -> Result<AppConfig, String> {
    service::save_onboarding_state(state.inner(), onboarding)
}
```

[MODIFY] `src-tauri/src/main.rs`

Add `save_onboarding_state` to `tauri::generate_handler!` after
`delete_repo_group`.

[MODIFY] `src-tauri/src/web.rs`

```rust
use crate::models::{
    AppConfig, BranchInfo, FileChange, OnboardingState, OperationResult, RepoGroup, Repository,
    ScanPath,
};

async fn save_onboarding_state(
    State(state): State<Arc<WebState>>,
    Json(onboarding): Json<OnboardingState>,
) -> ApiResult<AppConfig> {
    service::save_onboarding_state(state.config.as_ref(), onboarding)
        .map(Json)
        .map_err(api_error)
}
```

Register the route before the token middleware:

```rust
.route("/api/onboarding", post(save_onboarding_state))
```

### Validation

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

## Phase 2 — Repository Kind Detection

**Goal:** Provide the theme/plugin/other filter without organization-specific
rules or network calls.

### Files

- [MODIFY] `src-tauri/src/models.rs`
- [MODIFY] `src-tauri/src/scanner.rs`

### 2.1 Add the DTO field

[MODIFY] `src-tauri/src/models.rs`

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RepoKind {
    Theme,
    Plugin,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_accessible: bool,
    pub state: GitRepoState,
    pub group_ids: Vec<String>,
    #[serde(default)]
    pub kind: RepoKind,
}
```

### 2.2 Detect kind locally

[MODIFY] `src-tauri/src/scanner.rs`

```rust
use crate::models::{GitRepoState, RepoKind, Repository};

fn detect_repository_kind(repo_path: &Path) -> RepoKind {
    if repo_path.join("src/Resources/theme.json").is_file() {
        return RepoKind::Theme;
    }

    let composer_path = repo_path.join("composer.json");
    if let Ok(raw) = std::fs::read_to_string(composer_path) {
        if let Ok(composer) = serde_json::from_str::<serde_json::Value>(&raw) {
            if composer.get("type").and_then(|value| value.as_str())
                == Some("shopware-platform-plugin")
            {
                return RepoKind::Plugin;
            }
        }
    }

    RepoKind::Other
}
```

Use it in both `Repository` construction paths in `inspect_repository`:

```rust
kind: if repo_path.is_dir() {
    detect_repository_kind(repo_path)
} else {
    RepoKind::Other
},
```

This is a local, replaceable heuristic. It does not read remotes, call APIs, or
contain vendor names.

### 2.3 Tests

[MODIFY] `src-tauri/src/scanner.rs`

Add a synchronous test for the detector and an async test that inspects a real
temporary repository:

```rust
#[test]
fn detects_theme_plugin_and_other_repositories() {
    let temp = tempfile::tempdir().unwrap();

    let theme = temp.path().join("theme");
    fs::create_dir_all(theme.join("src/Resources")).unwrap();
    fs::write(theme.join("src/Resources/theme.json"), "{}").unwrap();

    let plugin = temp.path().join("plugin");
    fs::create_dir_all(&plugin).unwrap();
    fs::write(
        plugin.join("composer.json"),
        r#"{"type":"shopware-platform-plugin"}"#,
    )
    .unwrap();

    let other = temp.path().join("other");
    fs::create_dir_all(&other).unwrap();

    assert_eq!(detect_repository_kind(&theme), RepoKind::Theme);
    assert_eq!(detect_repository_kind(&plugin), RepoKind::Plugin);
    assert_eq!(detect_repository_kind(&other), RepoKind::Other);
}
```

Update existing scanner test expectations and any `Repository` literal
construction in Rust tests to include `kind`.

### Validation

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```

## Phase 3 — Frontend Domain and Pure Logic

**Goal:** Add typed domain fields and isolate all nontrivial matching/grouping
logic in testable modules before touching UI components.

### Files

- [MODIFY] `src/types/domain.ts`
- [MODIFY] `src/lib/backend.ts`
- [NEW FILE] `src/lib/onboarding.ts`
- [NEW FILE] `src/lib/repoFilters.ts`
- [NEW FILE] `src/lib/repoStatus.ts`
- [NEW FILE] `src/lib/groupDefaults.ts`
- [NEW FILE] `src/lib/__tests__/onboarding.test.ts`
- [NEW FILE] `src/lib/__tests__/repoFilters.test.ts`

### 3.1 Extend the TypeScript domain

[MODIFY] `src/types/domain.ts`

```ts
export type RepoKind = 'theme' | 'plugin' | 'other';
export type GroupingStyle = 'topLevel' | 'single';

export interface OnboardingState {
  completed: boolean;
  started: boolean;
  currentStep: number;
  scanPath: string | null;
  ownerNames: string[];
  groupingStyle: GroupingStyle;
  selectedRepoPaths: string[];
}

export interface AppConfig {
  scanPaths: ScanPath[];
  groups: RepoGroup[];
  onboarding: OnboardingState;
}

export interface Repository {
  id: string;
  name: string;
  path: string;
  isAccessible: boolean;
  state: GitRepoState;
  groupIds: string[];
  kind: RepoKind;
}
```

Add the backend call:

[MODIFY] `src/lib/backend.ts`

```ts
saveOnboardingState(onboarding: OnboardingState): Promise<AppConfig> {
  return request<AppConfig>(
    'save_onboarding_state',
    { onboarding },
    'POST',
    '/api/onboarding',
    onboarding,
  );
},
```

Import `OnboardingState` in `backend.ts`.

### 3.2 Onboarding helpers

[NEW FILE] `src/lib/onboarding.ts`

```ts
import type {
  AppConfig,
  GroupingStyle,
  OnboardingState,
  Repository,
} from '../types/domain';

export interface SuggestedGroup {
  name: string;
  repoPaths: string[];
}

export function defaultOnboardingState(): OnboardingState {
  return {
    completed: false,
    started: false,
    currentStep: 0,
    scanPath: null,
    ownerNames: [],
    groupingStyle: 'topLevel',
    selectedRepoPaths: [],
  };
}

export function shouldShowOnboarding(config: AppConfig): boolean {
  const onboarding = config.onboarding ?? defaultOnboardingState();
  if (onboarding.completed) return false;
  if (onboarding.started) return true;
  return config.scanPaths.length === 0 && config.groups.length === 0;
}

export function normalizeOwnerNames(raw: string): string[] {
  return [...new Set(
    raw
      .split(/[,\n]/)
      .map((owner) => owner.trim().replace(/^@/, '').toLowerCase())
      .filter(Boolean),
  )].sort();
}

export function parseRemoteOwner(remoteUrl: string): string | null {
  const trimmed = remoteUrl.trim();
  if (!trimmed) return null;

  const scpLike = trimmed.match(/^[^@\s]+@[^:\s]+:(.+)$/);
  if (scpLike) {
    return firstPathSegment(scpLike[1]);
  }

  if (!trimmed.includes('://')) return null;

  try {
    const url = new URL(trimmed);
    if (url.protocol === 'file:') return null;
    return firstPathSegment(url.pathname);
  } catch {
    return null;
  }
}

function firstPathSegment(path: string): string | null {
  const segments = path.replace(/^\/+/, '').split('/').filter(Boolean);
  return segments.length >= 2 ? segments[0].toLowerCase() : null;
}

export function ownedRepoPaths(
  repos: Repository[],
  ownerNames: string[],
): string[] {
  const owners = new Set(ownerNames.map((owner) => owner.toLowerCase()));
  if (owners.size === 0) return [];

  return repos
    .filter((repo) =>
      repo.state.remotes.some((remote) => {
        const owner = parseRemoteOwner(remote.url);
        return owner !== null && owners.has(owner);
      }),
    )
    .map((repo) => repo.path);
}

export function suggestGroups(
  repoPaths: string[],
  scanRoot: string,
  style: GroupingStyle,
): SuggestedGroup[] {
  const root = normalizePath(scanRoot);
  const rootName = baseName(root) || 'Projects';

  if (style === 'single') {
    return repoPaths.length > 0
      ? [{ name: rootName, repoPaths: [...repoPaths].sort() }]
      : [];
  }

  const entries = repoPaths.map((path) => ({
    path,
    segments: relativeSegments(path, root),
  }));
  const deepKeys = new Set(
    entries.filter((entry) => entry.segments.length > 1).map((entry) => entry.segments[0]),
  );
  const useTopLevel = deepKeys.size > 1;
  const buckets = new Map<string, string[]>();

  for (const entry of entries) {
    const key =
      useTopLevel && entry.segments.length > 1 ? entry.segments[0] : rootName;
    const bucket = buckets.get(key) ?? [];
    bucket.push(entry.path);
    buckets.set(key, bucket);
  }

  return [...buckets.entries()]
    .map(([name, paths]) => ({ name, repoPaths: paths.sort() }))
    .sort((a, b) => a.name.localeCompare(b.name));
}

function normalizePath(path: string): string {
  return path.replace(/\\/g, '/').replace(/\/+$/, '');
}

function relativeSegments(path: string, root: string): string[] {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath.toLowerCase().startsWith(`${root.toLowerCase()}/`)) {
    return [];
  }
  return normalizedPath.slice(root.length + 1).split('/').filter(Boolean);
}

function baseName(path: string): string {
  const parts = normalizePath(path).split('/').filter(Boolean);
  return parts.at(-1) ?? '';
}
```

Behavior covered by tests:

- Nested clones with two shop folders produce two groups.
- A root with only one nested container produces one group named after the root,
  avoiding a meaningless `vol` group.
- Repositories directly under the root produce one group named after the root.
- `single` grouping always produces one group.
- Git SCP remotes, HTTPS remotes, and non-remote paths are handled distinctly.

### 3.3 Filtering helpers

[NEW FILE] `src/lib/repoFilters.ts`

```ts
import type { RepoGroup, RepoKind, Repository } from '../types/domain';

export type RepoStatusFilter =
  | 'needsAttention'
  | 'changesNotSaved'
  | 'updatesAvailable'
  | 'toSend'
  | 'upToDate'
  | 'localOnly';

export interface AllReposFilters {
  statuses: RepoStatusFilter[];
  groupId: string;
  kind: RepoKind | 'all';
  search: string;
}

export const emptyAllReposFilters = (): AllReposFilters => ({
  statuses: [],
  groupId: 'all',
  kind: 'all',
  search: '',
});

export function matchesStatus(
  repo: Repository,
  status: RepoStatusFilter,
): boolean {
  switch (status) {
    case 'needsAttention':
      return (
        !repo.isAccessible ||
        repo.state.hasConflicts ||
        repo.state.operationState !== 'idle'
      );
    case 'changesNotSaved':
      return !repo.state.isClean;
    case 'updatesAvailable':
      return repo.isAccessible && repo.state.behindCount > 0;
    case 'toSend':
      return repo.isAccessible && repo.state.aheadCount > 0;
    case 'upToDate':
      return (
        repo.isAccessible &&
        repo.state.isClean &&
        repo.state.aheadCount === 0 &&
        repo.state.behindCount === 0 &&
        repo.state.remotes.length > 0 &&
        !repo.state.hasConflicts &&
        repo.state.operationState === 'idle'
      );
    case 'localOnly':
      return repo.isAccessible && repo.state.remotes.length === 0;
  }
}

export function filterAllRepos(
  repos: Repository[],
  filters: AllReposFilters,
  groups: RepoGroup[],
): Repository[] {
  const search = filters.search.trim().toLowerCase();
  const group =
    filters.groupId === 'all' || filters.groupId === 'ungrouped'
      ? null
      : groups.find((candidate) => candidate.id === filters.groupId) ?? null;

  return repos.filter((repo) => {
    if (
      filters.statuses.length > 0 &&
      !filters.statuses.some((status) => matchesStatus(repo, status))
    ) {
      return false;
    }

    if (filters.groupId === 'ungrouped') {
      const grouped = groups.some((candidate) =>
        candidate.repoPaths.includes(repo.path),
      );
      if (grouped) return false;
    } else if (group && !group.repoPaths.includes(repo.path)) {
      return false;
    }

    if (filters.kind !== 'all' && repo.kind !== filters.kind) return false;

    if (search) {
      const haystack =
        `${repo.name} ${repo.path} ${repo.state.currentBranch}`.toLowerCase();
      if (!haystack.includes(search)) return false;
    }

    return true;
  });
}

export function countByStatus(
  repos: Repository[],
  status: RepoStatusFilter,
): number {
  return repos.filter((repo) => matchesStatus(repo, status)).length;
}
```

### 3.4 Shared status copy and group defaults

[NEW FILE] `src/lib/repoStatus.ts`

Move `statusBadges`, `operationLabel`, and `branchLabel` out of
`RepoTable.vue` without changing their current labels or colors. This keeps the
group table and All repos page consistent.

[NEW FILE] `src/lib/groupDefaults.ts`

```ts
import type { RepoGroup } from '../types/domain';

export const GROUP_COLORS = [
  '#38bdf8',
  '#22c55e',
  '#f59e0b',
  '#f43f5e',
  '#a855f7',
  '#14b8a6',
] as const;

export function createGroup(
  name: string,
  repoPaths: string[],
  colorIndex: number,
): RepoGroup {
  return {
    id: `grp_${typeof crypto.randomUUID === 'function' ? crypto.randomUUID() : Date.now()}`,
    name: name.trim(),
    color: GROUP_COLORS[colorIndex % GROUP_COLORS.length],
    repoPaths: [...repoPaths],
    settings: {
      syncBranching: true,
      autoStashOnSwitch: true,
      defaultBaseBranch: 'main',
    },
  };
}
```

Update `GroupEditorModal.vue` to import `GROUP_COLORS` instead of duplicating the
palette. Do not change its behavior.

### 3.5 Tests

[NEW FILE] `src/lib/__tests__/onboarding.test.ts`

Cover:

- `shouldShowOnboarding`: fresh default, existing scan path, started, completed.
- `normalizeOwnerNames`: whitespace, leading `@`, case, duplicates.
- `parseRemoteOwner`: `git@github.com:Acme/repo.git`,
  `https://gitlab.com/Acme/repo.git`, local path, empty.
- `ownedRepoPaths`: matches any remote owner, case-insensitively.
- `suggestGroups`: two nested shops, one nested shop, side-by-side repos, and
  `single` style.

[NEW FILE] `src/lib/__tests__/repoFilters.test.ts`

Cover:

- Each status matcher, including repositories that match more than one status.
- Statuses combine with OR.
- Group filter: any, ungrouped, specific group.
- Kind filter.
- Search by name, path, and branch.
- Combined filters intersect.
- `countByStatus`.

### Validation

```bash
pnpm typecheck
pnpm test
```

## Phase 4 — Store Integration

**Goal:** Make onboarding state, wizard actions, all-repos filters, and the new
backend call available to components through the existing Pinia store.

### Files

- [MODIFY] `src/stores/repoStore.ts`
- [NEW FILE] `src/stores/__tests__/repoStore.test.ts` (only if the existing test
  setup supports Pinia without a DOM; otherwise cover the same behavior through
  pure modules and skip store internals)

### 4.1 State additions

[MODIFY] `src/stores/repoStore.ts`

```ts
import {
  defaultOnboardingState,
  normalizeOwnerNames,
  ownedRepoPaths,
  shouldShowOnboarding as configNeedsOnboarding,
  suggestGroups,
  type SuggestedGroup,
} from '../lib/onboarding';
import {
  countByStatus,
  emptyAllReposFilters,
  filterAllRepos,
  type AllReposFilters,
  type RepoStatusFilter,
} from '../lib/repoFilters';
import type {
  AppConfig,
  GroupingStyle,
  OnboardingState,
  RepoGroup,
  Repository,
} from '../types/domain';

const configLoaded = ref(false);
const onboardingOpen = ref(false);
const onboardingReplay = ref(false);
const onboardingStep = ref(0);
const onboardingError = ref('');
const onboardingSearch = ref('');
const onboardingDraft = ref({
  scanPath: '',
  ownerNames: '',
  groupingStyle: 'topLevel' as GroupingStyle,
  maxDepth: 8,
});
const onboardingSelectedPaths = ref<string[]>([]);
const onboardingGroups = ref<Array<SuggestedGroup & { included: boolean }>>([]);
const onboardingPendingGroups = ref<RepoGroup[]>([]);
const allReposFilters = ref<AllReposFilters>(emptyAllReposFilters());
```

Change the initial config value from the two-field object to:

```ts
const config = ref<AppConfig>({
  scanPaths: [],
  groups: [],
  onboarding: defaultOnboardingState(),
});
```

### 4.2 Computed state

```ts
const shouldShowOnboarding = computed(
  () => configLoaded.value && configNeedsOnboarding(config.value),
);

const onboardingSelectedRepositories = computed(() =>
  repositories.value.filter((repo) =>
    onboardingSelectedPaths.value.includes(repo.path),
  ),
);

const allReposFiltered = computed(() =>
  filterAllRepos(repositories.value, allReposFilters.value, config.value.groups),
);

const allReposBase = computed(() =>
  filterAllRepos(
    repositories.value,
    { ...allReposFilters.value, statuses: [] },
    config.value.groups,
  ),
);

const allReposStatusCounts = computed(() => ({
  needsAttention: countByStatus(allReposBase.value, 'needsAttention'),
  changesNotSaved: countByStatus(allReposBase.value, 'changesNotSaved'),
  updatesAvailable: countByStatus(allReposBase.value, 'updatesAvailable'),
  toSend: countByStatus(allReposBase.value, 'toSend'),
  upToDate: countByStatus(allReposBase.value, 'upToDate'),
  localOnly: countByStatus(allReposBase.value, 'localOnly'),
}));
```

### 4.3 Initialization and onboarding actions

```ts
async function initialize(skipOnboarding = false): Promise<void> {
  await loadConfig();
  await scanRepositories();
  if (!skipOnboarding && shouldShowOnboarding.value) {
    await openOnboarding(false);
  }
}

async function loadConfig(): Promise<void> {
  try {
    config.value = await backend.getConfig();
    configLoaded.value = true;
  } catch (error) {
    configLoaded.value = false;
    showToast('error', 'Could not load configuration', String(error));
  }
}

async function openOnboarding(replay: boolean): Promise<void> {
  const onboarding = config.value.onboarding ?? defaultOnboardingState();
  onboardingReplay.value = replay || onboarding.completed;
  onboardingStep.value = replay ? 0 : Math.min(onboarding.currentStep, 2);
  onboardingDraft.value = {
    scanPath: onboarding.scanPath ?? config.value.scanPaths[0]?.path ?? '',
    ownerNames: onboarding.ownerNames.join(', '),
    groupingStyle: onboarding.groupingStyle ?? 'topLevel',
    maxDepth: config.value.scanPaths[0]?.maxDepth ?? 8,
  };
  onboardingSelectedPaths.value = onboarding.selectedRepoPaths.filter((path) =>
    repositories.value.some((repo) => repo.path === path),
  );
  if (onboardingSelectedPaths.value.length === 0) {
    onboardingStep.value = Math.min(onboardingStep.value, 1);
  }
  onboardingGroups.value =
    onboardingStep.value >= 2
      ? suggestGroups(
          onboardingSelectedPaths.value,
          onboardingDraft.value.scanPath,
          onboardingDraft.value.groupingStyle,
        ).map((group) => ({ ...group, included: true }))
      : [];
  onboardingPendingGroups.value = [];
  onboardingError.value = '';
  onboardingOpen.value = true;
  await persistOnboardingProgress();
}

async function persistOnboardingProgress(completed = false): Promise<void> {
  const current = config.value.onboarding ?? defaultOnboardingState();
  const state: OnboardingState = {
    completed: completed || (onboardingReplay.value && current.completed),
    started: true,
    currentStep: onboardingStep.value,
    scanPath: onboardingDraft.value.scanPath.trim() || null,
    ownerNames: normalizeOwnerNames(onboardingDraft.value.ownerNames),
    groupingStyle: onboardingDraft.value.groupingStyle,
    selectedRepoPaths: [...onboardingSelectedPaths.value],
  };

  try {
    config.value = await backend.saveOnboardingState(state);
  } catch (error) {
    showToast('error', 'Could not save setup progress', String(error));
    throw error;
  }
}

async function scanOnboardingFolder(): Promise<boolean> {
  const path = onboardingDraft.value.scanPath.trim();
  if (!path) {
    onboardingError.value = 'Choose the folder where your projects live.';
    return false;
  }

  onboardingError.value = '';
  const added = await addScanPath({
    id: 'onboarding-main',
    path,
    maxDepth: onboardingDraft.value.maxDepth,
    enabled: true,
    excludePatterns: [],
  });
  if (!added) {
    onboardingError.value = 'That folder could not be scanned. Check the path.';
    return false;
  }

  const savedScanPath = config.value.scanPaths.find(
    (scanPath) => scanPath.id === 'onboarding-main',
  );
  if (savedScanPath) {
    onboardingDraft.value.scanPath = savedScanPath.path;
  }

  onboardingSelectedPaths.value = ownedRepoPaths(
    repositories.value,
    normalizeOwnerNames(onboardingDraft.value.ownerNames),
  );
  await persistOnboardingProgress();
  return true;
}
```

`addScanPath` already validates the folder, expands `~`, persists the scan path,
rescans, and shows a toast. The fixed id `onboarding-main` lets the wizard replace
its own folder choice instead of accumulating scan paths.

Continue with selection and group actions:

```ts
function toggleOnboardingRepo(path: string): void {
  onboardingSelectedPaths.value = onboardingSelectedPaths.value.includes(path)
    ? onboardingSelectedPaths.value.filter((item) => item !== path)
    : [...onboardingSelectedPaths.value, path];
}

function selectAllOnboardingRepos(paths: string[]): void {
  onboardingSelectedPaths.value = [...new Set(paths)];
}

function clearOnboardingSelection(): void {
  onboardingSelectedPaths.value = [];
}

async function goToOnboardingStep(step: number): Promise<void> {
  if (step === 2 && onboardingSelectedPaths.value.length === 0) {
    onboardingError.value = 'Choose at least one project to continue.';
    return;
  }

  if (step === 2) {
    onboardingGroups.value = suggestGroups(
      onboardingSelectedPaths.value,
      onboardingDraft.value.scanPath,
      onboardingDraft.value.groupingStyle,
    ).map((group) => ({ ...group, included: true }));
  }

  onboardingError.value = '';
  onboardingStep.value = step;
  await persistOnboardingProgress();
}

function updateOnboardingGroupName(index: number, name: string): void {
  const group = onboardingGroups.value[index];
  if (group) group.name = name;
}

function toggleOnboardingGroup(index: number): void {
  const group = onboardingGroups.value[index];
  if (group) group.included = !group.included;
}

async function prepareOnboardingGroups(
  groupsToCreate: RepoGroup[],
): Promise<boolean> {
  const names = groupsToCreate.map((group) => group.name.trim().toLowerCase());
  if (names.length === 0) {
    onboardingError.value = 'Keep at least one group to continue.';
    return false;
  }
  if (names.some((name) => name.length === 0)) {
    onboardingError.value = 'Give every included group a name.';
    return false;
  }
  if (new Set(names).size !== names.length) {
    onboardingError.value = 'Give each group a different name.';
    return false;
  }

  onboardingPendingGroups.value = groupsToCreate;
  onboardingError.value = '';
  onboardingStep.value = 3;
  await persistOnboardingProgress();
  return true;
}

async function completeOnboarding(openGroupId: string | null): Promise<void> {
  const groupsToCreate = onboardingPendingGroups.value;
  if (groupsToCreate.length === 0) {
    onboardingError.value = 'Confirm at least one group before finishing.';
    onboardingStep.value = 2;
    return;
  }

  try {
    for (const group of groupsToCreate) {
      config.value = await backend.saveRepoGroup(group);
    }
    await persistOnboardingProgress(true);
    onboardingOpen.value = false;
    await scanRepositories();
    const first = openGroupId ?? groupsToCreate[0]?.id ?? null;
    if (first) {
      selectGroup(first);
    } else {
      selectedGroupId.value = 'all';
    }
    const count = groupsToCreate.length;
    showToast(
      'success',
      'Setup complete',
      `${count} ${count === 1 ? 'group' : 'groups'} ready.`,
    );
    onboardingPendingGroups.value = [];
  } catch (error) {
    onboardingError.value = String(error);
    showToast('error', 'Could not finish setup', String(error));
  }
}

async function closeOnboarding(): Promise<void> {
  try {
    await persistOnboardingProgress();
  } finally {
    onboardingOpen.value = false;
  }
}
```

`prepareOnboardingGroups` is called by the groups step with only included groups
mapped to `createGroup(...)`. It validates names, stores the pending groups, and
advances to the ready step without writing groups yet. `completeOnboarding` is
called from the ready step, writes the pending groups, marks setup complete,
closes the wizard, rescans, and opens the requested group or the virtual All repos
view. If setup is reopened after an interrupted ready step, resuming returns to
the groups step so no group is created twice. Keep duplicate-name validation in
the UI too, but the store remains the authority for save failures.

### 4.4 All-repos filter actions

```ts
function toggleAllReposStatus(status: RepoStatusFilter): void {
  const statuses = allReposFilters.value.statuses;
  allReposFilters.value.statuses = statuses.includes(status)
    ? statuses.filter((item) => item !== status)
    : [...statuses, status];
}

function clearAllReposFilters(): void {
  allReposFilters.value = emptyAllReposFilters();
}

async function runRepoAction(
  path: string,
  action: 'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop',
): Promise<void> {
  isLoading.value = true;
  try {
    const results = await backend.executeGroupAction([path], action, 1);
    await refreshSingleRepo(path);
    await loadGroupBranches();
    summarizeResults(results, ACTION_LABELS[action]);
  } catch (error) {
    showToast('error', `${ACTION_LABELS[action]} failed`, String(error));
  } finally {
    isLoading.value = false;
  }
}
```

Extract the existing `actionLabel` record from `executeGroupAction` into a module
constant `ACTION_LABELS` and reuse it in both places. This gives the All repos page
a safe single-repository path for Get latest and Send changes without invoking
group semantics.

When deleting a group, reset the group filter if it points at the deleted group:

```ts
if (allReposFilters.value.groupId === groupId) {
  allReposFilters.value.groupId = 'all';
}
```

Export every new ref, computed value, and action from the store. Keep `initialize`
as the single entry point used by `App.vue`.

### 4.5 Tests

Prefer pure-module tests from Phase 3. If a store test is added, mock
`../../lib/backend` and assert:

- A fresh config opens onboarding; a config with scan paths does not.
- `scanOnboardingFolder` preselects only owner matches.
- `goToOnboardingStep(2)` rejects an empty selection.
- `prepareOnboardingGroups` validates names and advances to step 3 without writing
  groups.
- `completeOnboarding` writes the pending groups, persists onboarding as
  completed, and selects the requested group.
- `clearAllReposFilters` restores the default filter object.

### Validation

```bash
pnpm typecheck
pnpm test
```

## Phase 5 — Onboarding Wizard UI

**Goal:** Deliver the full-screen four-step setup experience and wire it into the
app shell.

### Files

- [NEW FILE] `src/components/OnboardingWizard.vue`
- [NEW FILE] `src/components/onboarding/OnboardingStepFind.vue`
- [NEW FILE] `src/components/onboarding/OnboardingStepChoose.vue`
- [NEW FILE] `src/components/onboarding/OnboardingStepGroups.vue`
- [NEW FILE] `src/components/onboarding/OnboardingStepReady.vue`
- [MODIFY] `src/App.vue`
- [MODIFY] `src/components/Sidebar.vue`

### 5.1 Wizard shell

[NEW FILE] `src/components/OnboardingWizard.vue`

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { X } from 'lucide-vue-next';
import { useRepoStore } from '../stores/repoStore';
import OnboardingStepChoose from './onboarding/OnboardingStepChoose.vue';
import OnboardingStepFind from './onboarding/OnboardingStepFind.vue';
import OnboardingStepGroups from './onboarding/OnboardingStepGroups.vue';
import OnboardingStepReady from './onboarding/OnboardingStepReady.vue';

const store = useRepoStore();
const heading = ref<HTMLElement | null>(null);

const steps = ['Find projects', 'Choose repos', 'Confirm groups', 'Ready'] as const;
const currentLabel = computed(() => steps[store.onboardingStep] ?? steps[0]);

onMounted(() => heading.value?.focus());

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') void store.closeOnboarding();
}
</script>

<template>
  <div
    class="fixed inset-0 z-50 flex flex-col overflow-y-auto bg-slate-950 text-slate-100"
    role="dialog"
    aria-modal="true"
    aria-labelledby="onboarding-title"
    @keydown="onKeydown"
  >
    <header class="border-b border-slate-800 px-6 py-4">
      <div class="mx-auto flex w-full max-w-4xl items-center justify-between gap-4">
        <div class="min-w-0">
          <p class="text-[11px] font-semibold uppercase tracking-widest text-cyan-400">
            Step {{ store.onboardingStep + 1 }} of 4 · {{ currentLabel }}
          </p>
          <h1
            id="onboarding-title"
            ref="heading"
            tabindex="-1"
            class="mt-1 text-xl font-semibold outline-none"
          >
            Let's set up your workspace
          </h1>
        </div>
        <button
          type="button"
          class="rounded-lg p-2 text-slate-400 hover:bg-slate-800 hover:text-slate-200"
          aria-label="Close setup and resume later"
          @click="store.closeOnboarding"
        >
          <X class="h-5 w-5" />
        </button>
      </div>
      <ol class="mx-auto mt-4 flex w-full max-w-4xl gap-2" aria-label="Setup progress">
        <li
          v-for="(step, index) in steps"
          :key="step"
          class="h-1 flex-1 rounded-full"
          :class="index <= store.onboardingStep ? 'bg-cyan-500' : 'bg-slate-800'"
        >
          <span class="sr-only">{{ step }}</span>
        </li>
      </ol>
    </header>

    <main class="mx-auto w-full max-w-4xl flex-1 px-6 py-8">
      <OnboardingStepFind v-if="store.onboardingStep === 0" />
      <OnboardingStepChoose v-else-if="store.onboardingStep === 1" />
      <OnboardingStepGroups v-else-if="store.onboardingStep === 2" />
      <OnboardingStepReady v-else />
    </main>
  </div>
</template>
```

The wizard replaces the workspace while open, so the background cannot be
clicked. Focus starts on the heading, Escape closes and saves progress, and the
close button has an explicit accessible name.

### 5.2 Find step

[NEW FILE] `src/components/onboarding/OnboardingStepFind.vue`

Requirements:

- Label the folder input. Explain that the app looks for Git projects and skips
  dependency/build folders.
- Optional owner names field with the hint that it is used only to pre-select and
  stays on this computer.
- Advanced `<details>` with scan depth (default 8, min 1, max 12) and grouping
  style (`topLevel` or `single`).
- Submit calls `store.scanOnboardingFolder()`. On success call
  `store.goToOnboardingStep(1)`.
- Show `store.onboardingError` inline with `role="alert"`.
- Disable submit while `store.isScanning`; show a plain progress message.
- Keep the primary label "Find my projects".

Use `TdButton`, `TdInput`, and `TdFormField` where they already fit the existing
scan modal; otherwise use the same Tailwind classes as the current empty state.

### 5.3 Choose step

[NEW FILE] `src/components/onboarding/OnboardingStepChoose.vue`

Requirements:

- Show search bound to `store.onboardingSearch`, select-all-visible, and
  clear-selection controls.
- List `store.repositories` with a checkbox per row, repository name, kind,
  current branch, and path.
- Show the selected count, e.g. "12 of 48 projects selected".
- Owner-matched rows are already selected by the store; do not auto-reselect after
  the user clears them.
- Empty search results show "No projects match" with a clear-search action.
- Back calls `store.goToOnboardingStep(0)`. Continue is disabled when nothing is
  selected and calls `store.goToOnboardingStep(2)`.
- If scanning is still running, show a skeleton or progress state and disable
  continue.

### 5.4 Groups step

[NEW FILE] `src/components/onboarding/OnboardingStepGroups.vue`

Requirements:

- Render `store.onboardingGroups` as cards with an include checkbox, editable name
  input, repository count, and the first few repository names. Use
  `store.onboardingSelectedRepositories` for the summary count.
- Help text: "We suggested these from your folders. Rename them or leave one out;
  you can fine-tune membership later."
- Back returns to step 1.
- Finish collects included groups, trims names, rejects empty or duplicate names,
  maps them through `createGroup(name, repoPaths, index)`, and calls
  `store.prepareOnboardingGroups(groups)`. On success the store advances to the
  ready step; the wizard stays open. Groups are written only when the ready step
  completes.
- Disable finish while busy or when no group is included.
- Keep error text in `role="alert"`.

### 5.5 Ready step

[NEW FILE] `src/components/onboarding/OnboardingStepReady.vue`

Requirements:

- Summarize the accepted repository count and the number of groups in
  `store.onboardingPendingGroups`.
- Explain the three actions in plain language:
  - **Save to Git**: saves a checkpoint of your changed files.
  - **Get latest**: brings new work from the server.
  - **Send changes**: uploads your saved checkpoints.
- Offer "Open [first group]" as the primary action and a secondary "Open All
  repos". If there are several groups, list them as buttons. Use the actual group
  names and IDs from `store.onboardingPendingGroups`.
- Do not commit anything, do not force a checkout, and do not create a demo file.
- Close the wizard by calling `store.completeOnboarding(groupId)` or
  `store.completeOnboarding(null)` for the virtual All repos view. This is the
  point where the pending groups are written for the first time.

### 5.6 App and sidebar wiring

[MODIFY] `src/App.vue`

```vue
<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { subscribeToRepoChanges } from './lib/backend';
import { useRepoStore } from './stores/repoStore';
import AllReposPage from './components/AllReposPage.vue';
import AddScanPathModal from './components/AddScanPathModal.vue';
import GroupEditorModal from './components/GroupEditorModal.vue';
import GuidanceBanner from './components/GuidanceBanner.vue';
import HelpModal from './components/HelpModal.vue';
import OnboardingWizard from './components/OnboardingWizard.vue';
import RepoTable from './components/RepoTable.vue';
import SaveChangesModal from './components/SaveChangesModal.vue';
import Sidebar from './components/Sidebar.vue';
import ToastStack from './components/ToastStack.vue';
import Toolbar from './components/Toolbar.vue';

const store = useRepoStore();
let unsubscribe: (() => void) | null = null;

onMounted(async () => {
  await store.initialize();

  try {
    unsubscribe = await subscribeToRepoChanges((path) => {
      void store.refreshSingleRepo(path);
    });
  } catch (error) {
    store.showToast('error', 'Live file watching unavailable', String(error));
  }
});

onUnmounted(() => unsubscribe?.());
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-slate-900 font-sans text-slate-100">
    <OnboardingWizard v-if="store.onboardingOpen" />
    <template v-else>
      <Sidebar />
      <div class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
        <AllReposPage v-if="store.selectedGroupId === 'all'" />
        <template v-else>
          <Toolbar />
          <GuidanceBanner />
          <RepoTable />
        </template>
      </div>

      <GroupEditorModal />
      <AddScanPathModal />
      <SaveChangesModal />
      <HelpModal />
    </template>
    <ToastStack />
  </div>
</template>
```

[MODIFY] `src/components/Sidebar.vue`

- Rename "All projects" to "All repos"; use a distinct icon from the group icon.
- Keep the virtual entry first and unchanged in behavior.
- Add a **Setup** button near the bottom, above the add-folder button, calling
  `store.openOnboarding(true)`. Use a wand/sparkles icon and an accessible label.
- Do not add edit/delete controls to All repos.

### Validation

```bash
pnpm typecheck
pnpm test
pnpm build
```

Manual smoke: fresh config with no scan paths opens the wizard; closing and
reopening the app resumes; completing setup creates groups and selects one.

## Phase 6 — All Repos Page and Shared Table

**Goal:** Deliver the virtual, filterable all-repositories page without
duplicating row/status logic or exposing bulk actions.

### Files

- [NEW FILE] `src/components/RepoListTable.vue`
- [NEW FILE] `src/components/AllReposPage.vue`
- [MODIFY] `src/components/RepoTable.vue`
- [MODIFY] `src/components/Toolbar.vue` (copy fallback only)
- [MODIFY] `src/components/GroupEditorModal.vue` (palette import from Phase 3)
- [NEW FILE] `src/components/__tests__/allReposFilters.smoke.test.ts` (optional UI
  smoke only if the store mock is straightforward; pure tests remain mandatory)

### 6.1 Extract the shared table

[NEW FILE] `src/components/RepoListTable.vue`

Move the current table markup and row actions out of `RepoTable.vue`. Props:

```ts
defineProps<{
  repos: Repository[];
  actions?: 'group' | 'browse';
}>();
```

The component owns the temporary per-row refresh spinner and calls
`store.refreshSingleRepo(path)`. It imports `statusBadges`, `branchLabel`, and
`operationLabel` from `src/lib/repoStatus.ts`.

- In `group` mode, keep the current **Save/Review** and refresh controls. The group
  toolbar owns bulk actions.
- In `browse` mode, add per-row **Get latest** and **Send changes** controls next to
  Save/refresh. They call `store.runRepoAction(path, 'pull')` and
  `store.runRepoAction(path, 'push')`. Disable them while that row is busy or the
  repository is inaccessible. Use accessible names such as "Get latest for
  <repo>", not icon-only tooltips.
- Never render a bulk action inside this table.

### 6.2 Slim the group table

[MODIFY] `src/components/RepoTable.vue`

- Keep the search field, "x of y shown" count, and group-specific empty states.
- Replace the table body with `<RepoListTable :repos="rows" actions="group" />`.
- Delete the duplicated status functions and refresh state.
- Leave the group toolbar and guidance banner untouched; they remain group-only in
  `App.vue`.

### 6.3 Build the All repos page

[NEW FILE] `src/components/AllReposPage.vue`

```vue
<script setup lang="ts">
import { computed } from 'vue';
import { FolderSearch, Search, SlidersHorizontal } from 'lucide-vue-next';
import RepoListTable from './RepoListTable.vue';
import { useRepoStore } from '../stores/repoStore';
import type { RepoStatusFilter } from '../lib/repoFilters';

const store = useRepoStore();

const statusOptions: Array<{ value: RepoStatusFilter; label: string }> = [
  { value: 'needsAttention', label: 'Needs attention' },
  { value: 'changesNotSaved', label: 'Changes not saved' },
  { value: 'updatesAvailable', label: 'Updates available' },
  { value: 'toSend', label: 'To send' },
  { value: 'upToDate', label: 'Up to date' },
  { value: 'localOnly', label: 'On this computer only' },
];

const groupOptions = computed(() => [
  { value: 'all', label: 'Any group' },
  { value: 'ungrouped', label: 'Ungrouped' },
  ...store.config.groups.map((group) => ({ value: group.id, label: group.name })),
]);

const hasFilters = computed(() => {
  const filters = store.allReposFilters;
  return (
    filters.statuses.length > 0 ||
    filters.groupId !== 'all' ||
    filters.kind !== 'all' ||
    filters.search.trim() !== ''
  );
});
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col bg-slate-900">
    <header class="border-b border-slate-800/70 px-4 py-3">
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 class="text-base font-semibold text-slate-100">All repos</h1>
          <p class="mt-0.5 text-xs text-slate-400">
            Browse every project. Save, update, or send work from a single row — or
            open a group to act on several at once.
          </p>
        </div>
        <button
          class="flex items-center gap-1.5 rounded-lg bg-slate-800 px-3 py-1.5 text-xs text-slate-200 hover:bg-slate-700"
          @click="store.openAddScanPath"
        >
          <FolderSearch class="h-3.5 w-3.5" />
          Add project folder
        </button>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-2">
        <label class="relative min-w-56 flex-1">
          <span class="sr-only">Search all repositories</span>
          <Search class="pointer-events-none absolute left-3 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-slate-500" />
          <input
            v-model="store.allReposFilters.search"
            type="search"
            placeholder="Search name, path, or branch…"
            class="w-full rounded-lg border border-slate-800 bg-slate-950 py-1.5 pl-9 pr-3 text-xs text-slate-200 placeholder:text-slate-600 focus:border-cyan-600"
          />
        </label>

        <label class="flex items-center gap-2 text-[11px] text-slate-400">
          Group
          <select
            v-model="store.allReposFilters.groupId"
            class="rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-slate-200"
          >
            <option v-for="option in groupOptions" :key="option.value" :value="option.value">
              {{ option.label }}
            </option>
          </select>
        </label>

        <label class="flex items-center gap-2 text-[11px] text-slate-400">
          Type
          <select
            v-model="store.allReposFilters.kind"
            class="rounded-lg border border-slate-800 bg-slate-950 px-2 py-1.5 text-xs text-slate-200"
          >
            <option value="all">Any type</option>
            <option value="theme">Theme</option>
            <option value="plugin">Plugin</option>
            <option value="other">Other</option>
          </select>
        </label>
      </div>

      <div class="mt-3 flex flex-wrap items-center gap-1.5" aria-label="Filter by status">
        <button
          v-for="option in statusOptions"
          :key="option.value"
          type="button"
          class="rounded-full border px-2.5 py-1 text-[11px] transition-colors"
          :class="
            store.allReposFilters.statuses.includes(option.value)
              ? 'border-cyan-500/50 bg-cyan-500/15 text-cyan-200'
              : 'border-slate-800 bg-slate-900 text-slate-400 hover:text-slate-200'
          "
          :aria-pressed="store.allReposFilters.statuses.includes(option.value)"
          @click="store.toggleAllReposStatus(option.value)"
        >
          {{ option.label }}
          <span class="ml-1 font-mono text-[10px] text-slate-500">
            {{ store.allReposStatusCounts[option.value] }}
          </span>
        </button>

        <button
          v-if="hasFilters"
          type="button"
          class="ml-1 flex items-center gap-1 text-[11px] text-slate-400 hover:text-slate-200"
          @click="store.clearAllReposFilters"
        >
          <SlidersHorizontal class="h-3 w-3" />
          Clear filters
        </button>
      </div>
    </header>

    <div class="flex items-center justify-between border-b border-slate-800/70 px-4 py-2">
      <span class="font-mono text-[11px] text-slate-500">
        {{ store.allReposFiltered.length }} of {{ store.repositories.length }} repos
      </span>
    </div>

    <RepoListTable
      v-if="store.allReposFiltered.length > 0"
      :repos="store.allReposFiltered"
      actions="browse"
      class="min-h-0 flex-1"
    />

    <div v-else class="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <template v-if="store.repositories.length === 0">
        <h2 class="text-base font-semibold text-slate-100">No projects yet</h2>
        <p class="mt-1.5 max-w-md text-sm text-slate-400">
          Find the folder where your projects live. We will add every Git project
          inside it.
        </p>
        <button
          class="mt-4 rounded-lg bg-cyan-500 px-4 py-2 text-xs font-semibold text-slate-950"
          @click="store.openAddScanPath"
        >
          Add project folder
        </button>
      </template>
      <template v-else>
        <h2 class="text-base font-semibold text-slate-100">No repos match</h2>
        <p class="mt-1.5 text-sm text-slate-400">
          Try another filter or clear everything to see all projects.
        </p>
        <button
          class="mt-4 rounded-lg px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800"
          @click="store.clearAllReposFilters"
        >
          Clear filters
        </button>
      </template>
    </div>
  </section>
</template>
```

Add `AllReposPage` to `App.vue` as shown in Phase 5. Hide the group Toolbar and
GuidanceBanner when `selectedGroupId === 'all'`.

[MODIFY] `src/components/Toolbar.vue`

Change the fallback title from "All projects" to "Project group" or remove the
fallback, because All repos no longer uses the toolbar. This avoids showing an
incorrect all-projects label in the group workspace.

### 6.4 Tests and verification

- The pure filter tests from Phase 3 are the primary regression coverage.
- Add a smoke test only if the store can be mocked without importing the real
  backend; assert that All repos renders filter labels and row actions.
- Manual check: select two status chips, a group, a type, and a search term; verify
  the result count and clear action. Use the per-row Get latest and Send changes
  controls and confirm only that repository is affected. Verify no group bulk
  buttons appear.

### Validation

```bash
pnpm typecheck
pnpm test
pnpm build
```

## Phase 7 — Documentation and Housekeeping

**Goal:** Update user-facing documentation and confirm no build artifacts or
generated files need ignoring.

### Files

- [MODIFY] `README.md`
- [MODIFY] `CHANGELOG.md`
- [MODIFY] `_ai/technical_decisions/README.md` (already updated by the ADR work;
  verify links)
- No `.gitignore` change is expected: this plan introduces only source files and
  no new build output, cache, or generated directory. If implementation adds one,
  update `.gitignore` in this phase.

### README

Add to the features and everyday workflow:

- First run opens a setup wizard that scans one folder deeply, optionally uses
  remote owner names, and proposes groups.
- All repos is a browse/filter view; bulk actions live in groups.
- Setup can be reopened from the sidebar and resumes after closing.

### CHANGELOG

Under `## [Unreleased]`:

```markdown
### Added

- Resumable first-run setup wizard with deep folder scanning, optional remote
  owner pre-selection, editable group suggestions, and a ready screen for the
  Save/Get latest/Send workflow.
- Virtual All repos page with status, group, type, and search filters, plus
  row-level actions.
- Repository kind detection for themes, plugins, and other repositories.
- `save_onboarding_state` Tauri command and `POST /api/onboarding` endpoint.

### Changed

- The sidebar's "All projects" entry is now "All repos" and opens a dedicated
  browse page; group workspaces keep the bulk action toolbar.
```

Do not claim a release version; keep entries under Unreleased.

### Validation

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
pnpm typecheck
pnpm test
pnpm build
```

Optionally smoke-test web mode:

```bash
cargo build --manifest-path src-tauri/Cargo.toml --release
./src-tauri/target/release/repoman \
  --web --port 8799 --token onboarding-smoke \
  --config /tmp/repo-manager-onboarding-smoke.json --dist ./dist
```

Then verify `GET /api/config` includes `onboarding` and
`POST /api/onboarding` accepts a state object with the bearer token.

## Phase 8 — Implementation Report

**Goal:** Record what shipped, what changed, and how it was verified.

### File

- [NEW FILE] `_ai/backlog/reports/261030_0925__IMPLEMENTATION_REPORT__first_run_onboarding_and_all_repos.md`

Use this frontmatter:

```yaml
---
filename: "_ai/backlog/reports/261030_0925__IMPLEMENTATION_REPORT__first_run_onboarding_and_all_repos.md"
title: "Report: First-Run Onboarding Wizard and Filterable All Repos Page"
createdAt: 2026-10-30 09:25
updatedAt: 2026-10-30 09:25
planFile: "_ai/backlog/active/261030_0925__IMPLEMENTATION_PLAN__first_run_onboarding_and_all_repos.md"
specFile: ""
project: "repo-manager"
status: completed
filesCreated: 0
filesModified: 0
filesDeleted: 0
tags: [onboarding, ux, discovery, groups, filtering]
documentType: IMPLEMENTATION_REPORT
---
```

Report sections:

1. **Summary** — two or three sentences on the wizard, virtual All repos page, and
   configuration compatibility.
2. **Files Changed** — new files with one-line descriptions; modified files with
   the reason.
3. **Key Changes** — onboarding schema and transports, kind detection, pure
   grouping/filter logic, wizard UI, All repos UI.
4. **Deviations from Plan** — only real deviations, including any scan-depth or
   filter behavior that changed during implementation.
5. **Technical Decisions** — note any decisions made during coding that were not
   in ADR 261030-1 or ADR 261030-2.
6. **Testing Notes** — exact commands run and results.
7. **Usage Examples** — setup flow, resume behavior, filters, and API examples.
8. **Documentation Updates** — README, CHANGELOG, ADR index.
9. **Next Steps** — optional follow-ups such as saved filter presets or
   multi-folder onboarding, clearly marked as out of scope.

## Traceability

There is no feature spec, so traceability maps phases to the approved design
decisions and ADRs.

| Phase | Satisfies | ADR |
| --- | --- | --- |
| 1 — Backend onboarding schema and persistence | Resumable setup; local-only owner names; backward-compatible config | ADR 261030-1 |
| 2 — Repository kind detection | Theme/plugin/other filter without vendor assumptions | ADR 261030-2 |
| 3 — Frontend domain and pure logic | Generic owner matching; folder-derived groups; filter semantics; shared status copy | ADR 261030-1, ADR 261030-2 |
| 4 — Store integration | Wizard state, first-run detection, resumable actions, all-repos filters | ADR 261030-1, ADR 261030-2 |
| 5 — Onboarding UI | Four-step full-screen flow; accept repos; rename/skip groups; ready screen | ADR 261030-1 |
| 6 — All repos page | Virtual view; filters; row-level actions only; no bulk actions | ADR 261030-2 |
| 7 — Documentation and housekeeping | User-facing setup and browse documentation; no stale release claims | ADR 261030-1, ADR 261030-2 |
| 8 — Implementation report | Verification record and deviations | All |

## Risks and Mitigations

- **Group suggestions are imperfect for unusual folder layouts.** Mitigate with
  editable names, skip controls, and the existing group editor; cover the three
  common layouts in pure tests.
- **A deep scan can be slow on very large trees.** Keep the existing ignore rules
  and dependency pruning; expose the advanced depth control; show explicit scan
  progress and allow closing/resuming.
- **Checkbox selection can be lost if the folder is rescanned.** Preserve selected
  paths when they still exist; if they do not, leave them out rather than failing.
- **Existing configs may be read by an older binary.** The new onboarding field is
  additive; older code ignores it. There is no destructive schema migration.
- **Bulk-action confusion.** Do not render the group toolbar or guidance banner on
  All repos; keep only row actions there.

## Open Questions

None. The approved conversation and the two new ADRs cover the decisions needed
for implementation.
