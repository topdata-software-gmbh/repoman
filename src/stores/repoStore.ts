import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { backend } from '../lib/backend';
import { t, tPlural, type MessageKey } from '../lib/i18n';
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
import { branchLabel, statusBadges } from '../lib/repoStatus';
import type {
  AppConfig,
  BranchInfo,
  CommitGenEvent,
  GitStepEvent,
  GroupingStyle,
  OnboardingState,
  OperationResult,
  RepoDraftRow,
  RepoGroup,
  Repository,
  ScanActivityState,
  ScanPath,
  ScanProgressEvent,
} from '../types/domain';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  message?: string;
}

export type RefreshResult =
  | { ok: true; repo: Repository }
  | { ok: false; error: string };

export type InsightAction = 'save' | 'pull' | 'push';

export interface GroupInsight {
  tone: 'success' | 'info' | 'warning' | 'danger';
  title: string;
  message: string;
  action: InsightAction | null;
  actionLabel?: string;
}

let nextToastId = 0;
let branchRequestId = 0;

const GIT_STEP_LIMIT = 200;
const REPOSITORY_CACHE_KEY = 'repoman.repositories.v1';
const SCAN_CHIP_LINGER_MS = 10_000;
const SCAN_ELAPSED_TICK_MS = 1_000;

function emptyScanActivity(): ScanActivityState {
  return {
    runId: '',
    roots: [],
    running: false,
    currentDir: null,
    currentRoot: null,
    startedAt: 0,
    entries: new Map(),
    finished: null,
    modalOpen: false,
    summaryVisible: false,
    elapsedMs: 0,
  };
}

function readRepositoryCache(): Repository[] {
  try {
    const raw = window.localStorage.getItem(REPOSITORY_CACHE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as Repository[]) : [];
  } catch {
    return [];
  }
}

function writeRepositoryCache(repositories: Repository[]): void {
  try {
    window.localStorage.setItem(REPOSITORY_CACHE_KEY, JSON.stringify(repositories));
  } catch {
    /* Storage can be full or unavailable; the next scan will rebuild the list. */
  }
}

type GroupAction = 'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop';

const ACTION_COPY: Record<
  GroupAction,
  { success: MessageKey; failure: MessageKey; failed: MessageKey }
> = {
  fetch: { success: 'toast.fetchDone', failure: 'toast.fetchFailure', failed: 'toast.fetchFailed' },
  pull: { success: 'toast.pullDone', failure: 'toast.pullFailure', failed: 'toast.pullFailed' },
  push: { success: 'toast.pushDone', failure: 'toast.pushFailure', failed: 'toast.pushFailed' },
  stash: { success: 'toast.stashDone', failure: 'toast.stashFailure', failed: 'toast.stashFailed' },
  stash_pop: {
    success: 'toast.stashPopDone',
    failure: 'toast.stashPopFailure',
    failed: 'toast.stashPopFailed',
  },
};

export const useRepoStore = defineStore('repo', () => {
  const config = ref<AppConfig>({
    scanPaths: [],
    groups: [],
    onboarding: defaultOnboardingState(),
  });
  const repositories = ref<Repository[]>([]);
  const selectedGroupId = ref<string>('all');
  const isLoading = ref(false);
  const isScanning = ref(false);
  const isBranchListLoading = ref(false);
  const groupBranches = ref<BranchInfo[]>([]);
  const toasts = ref<Toast[]>([]);
  const scanModalOpen = ref(false);
  const saveModalOpen = ref(false);
  const saveModalPaths = ref<string[] | null>(null);
  const helpModalOpen = ref(false);
  const groupEditorOpen = ref(false);
  const editingGroup = ref<RepoGroup | null>(null);

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
    maxDepth: 4,
  });
  const onboardingSelectedPaths = ref<string[]>([]);
  const onboardingGroups = ref<Array<SuggestedGroup & { included: boolean }>>([]);
  const onboardingPendingGroups = ref<RepoGroup[]>([]);
  const allReposFilters = ref<AllReposFilters>(emptyAllReposFilters());
  const gitSteps = ref<GitStepEvent[]>([]);
  const gitLogOpen = ref(false);

  // Session-only live scan activity (data-model E1); never persisted.
  const scanActivity = ref<ScanActivityState>(emptyScanActivity());
  let scanElapsedTimer: ReturnType<typeof setInterval> | null = null;
  let scanChipTimer: ReturnType<typeof setTimeout> | null = null;

  const scanActivityCount = computed(() => {
    const state = scanActivity.value;
    return Math.max(state.entries.size, state.finished?.count ?? 0);
  });

  function stopScanElapsedTicker(): void {
    if (scanElapsedTimer !== null) {
      clearInterval(scanElapsedTimer);
      scanElapsedTimer = null;
    }
  }

  function startScanElapsedTicker(startedAt: number): void {
    stopScanElapsedTicker();
    scanActivity.value.elapsedMs = Math.max(0, Date.now() - startedAt);
    scanElapsedTimer = setInterval(() => {
      const state = scanActivity.value;
      if (!state.running) return;
      state.elapsedMs = Math.max(0, Date.now() - state.startedAt);
    }, SCAN_ELAPSED_TICK_MS);
  }

  function scheduleScanChipHide(): void {
    if (scanChipTimer !== null) clearTimeout(scanChipTimer);
    scanChipTimer = setTimeout(() => {
      scanActivity.value.summaryVisible = false;
      scanChipTimer = null;
    }, SCAN_CHIP_LINGER_MS);
  }

  function lastPathSegment(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? path;
  }

  /**
   * Reducer for one `scan:progress` event (FR-001…FR-009, FR-015):
   * `started` replaces the whole slice; foreign run ids are ignored;
   * `found` inserts once; `inspected` updates the entry in place;
   * `done` finalizes and lingers the chip for ~10 s.
   */
  function applyScanProgress(event: ScanProgressEvent): void {
    if (event.phase === 'started') {
      if (scanChipTimer !== null) {
        clearTimeout(scanChipTimer);
        scanChipTimer = null;
      }
      const startedAt = event.startedAt ?? Date.now();
      scanActivity.value = {
        runId: event.runId,
        roots: event.roots ?? [],
        running: true,
        currentDir: null,
        currentRoot: null,
        startedAt,
        entries: new Map(),
        finished: null,
        modalOpen: scanActivity.value.modalOpen,
        summaryVisible: false,
        elapsedMs: 0,
      };
      startScanElapsedTicker(startedAt);
      return;
    }

    const state = scanActivity.value;
    if (event.runId !== state.runId) return;
    if (state.finished || !state.running) return;

    switch (event.phase) {
      case 'found': {
        if (!event.path || state.entries.has(event.path)) break;
        if (event.root) state.currentRoot = event.root;
        state.entries.set(event.path, {
          name: event.name ?? lastPathSegment(event.path),
          root: event.root ?? '',
          kind: null,
          inspected: false,
        });
        break;
      }
      case 'tick': {
        if (event.currentDir) state.currentDir = event.currentDir;
        if (event.root) state.currentRoot = event.root;
        break;
      }
      case 'inspected': {
        if (!event.path || !event.kind) break;
        const entry = state.entries.get(event.path);
        if (entry) {
          entry.kind = event.kind;
          entry.inspected = true;
        }
        break;
      }
      case 'done': {
        state.running = false;
        state.finished = {
          count: event.foundCount ?? state.entries.size,
          elapsedMs: event.elapsedMs ?? state.elapsedMs,
          error: event.error ?? null,
        };
        stopScanElapsedTicker();
        state.elapsedMs = state.finished.elapsedMs;
        state.summaryVisible = true;
        scheduleScanChipHide();
        break;
      }
    }
  }

  /** Promise rejection path (FR-008): terminate a running scan even without `done`. */
  function failScanActivity(message: string): void {
    const state = scanActivity.value;
    if (state.finished || !state.runId) return;
    state.running = false;
    state.finished = {
      count: state.entries.size,
      elapsedMs: Math.max(0, Date.now() - (state.startedAt || Date.now())),
      error: message,
    };
    stopScanElapsedTicker();
    state.elapsedMs = state.finished.elapsedMs;
    state.summaryVisible = true;
    scheduleScanChipHide();
  }

  function openScanModal(): void {
    scanActivity.value.modalOpen = true;
    scanActivity.value.summaryVisible = false;
    if (scanChipTimer !== null) {
      clearTimeout(scanChipTimer);
      scanChipTimer = null;
    }
  }

  function closeScanModal(): void {
    scanActivity.value.modalOpen = false;
  }

  const currentGroup = computed(() => {
    if (selectedGroupId.value === 'all') return null;
    return config.value.groups.find((group) => group.id === selectedGroupId.value) ?? null;
  });

  const filteredRepositories = computed(() => {
    if (selectedGroupId.value === 'all') return repositories.value;
    const group = currentGroup.value;
    if (!group) return [];
    return repositories.value.filter((repo) => group.repoPaths.includes(repo.path));
  });

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

  const commonBranches = computed(() => {
    const list = filteredRepositories.value;
    if (list.length === 0) return [];

    const counts: Record<string, number> = {};
    for (const repo of list) {
      const branch = repo.state.currentBranch;
      if (branch) counts[branch] = (counts[branch] ?? 0) + 1;
    }

    return Object.entries(counts)
      .map(([branch, count]) => ({
        branch,
        count,
        isSynchronized: count === list.length,
      }))
      .sort((a, b) => b.count - a.count || a.branch.localeCompare(b.branch));
  });

  const synchronizedBranch = computed(() => {
    const list = filteredRepositories.value;
    if (list.length === 0) return null;
    const [first] = commonBranches.value;
    if (!first || first.count !== list.length) return null;
    return first.branch;
  });

  const cleanRepositoryCount = computed(
    () => filteredRepositories.value.filter((repo) => repo.state.isClean).length,
  );

  const dirtyRepositories = computed(() =>
    filteredRepositories.value.filter((repo) => !repo.state.isClean),
  );

  const conflictedRepositories = computed(() =>
    filteredRepositories.value.filter((repo) => repo.state.hasConflicts),
  );

  const insight = computed<GroupInsight | null>(() => {
    const list = filteredRepositories.value;
    if (list.length === 0) return null;

    const count = (predicate: (repo: Repository) => boolean): number =>
      list.filter(predicate).length;

    const missing = count((repo) => !repo.isAccessible);
    if (missing > 0) {
      return {
        tone: 'danger',
        title: t(missing === 1 ? 'insight.missing.one' : 'insight.missing.other', {
          count: missing,
        }),
        message: t('insight.missing.text'),
        action: null,
      };
    }

    const conflicts = count((repo) => repo.state.hasConflicts);
    if (conflicts > 0) {
      return {
        tone: 'danger',
        title: t(conflicts === 1 ? 'insight.conflicts.one' : 'insight.conflicts.other', {
          count: conflicts,
        }),
        message: t('insight.conflicts.text'),
        action: 'save',
        actionLabel: t('insight.conflicts.action'),
      };
    }

    const busy = count((repo) => repo.state.operationState !== 'idle');
    if (busy > 0) {
      return {
        tone: 'warning',
        title: t('insight.busy.title'),
        message: t('insight.busy.text'),
        action: null,
      };
    }

    const detached = count((repo) => repo.state.isDetachedHead);
    if (detached > 0) {
      return {
        tone: 'warning',
        title: t('insight.detached.title'),
        message: t('insight.detached.text'),
        action: null,
      };
    }

    const dirty = count((repo) => !repo.state.isClean);
    if (dirty > 0) {
      return {
        tone: 'warning',
        title: t(dirty === 1 ? 'insight.dirty.one' : 'insight.dirty.other', { count: dirty }),
        message: t('insight.dirty.text'),
        action: 'save',
        actionLabel: t(dirty === 1 ? 'insight.dirty.actionOne' : 'insight.dirty.actionOther'),
      };
    }

    const behind = count((repo) => repo.state.behindCount > 0);
    if (behind > 0) {
      return {
        tone: 'info',
        title: t(behind === 1 ? 'insight.behind.one' : 'insight.behind.other'),
        message: t(behind === 1 ? 'insight.behind.text.one' : 'insight.behind.text.other', {
          count: behind,
        }),
        action: 'pull',
        actionLabel: t('action.pull'),
      };
    }

    const ahead = count((repo) => repo.state.aheadCount > 0);
    if (ahead > 0) {
      return {
        tone: 'info',
        title: t('insight.ahead.title'),
        message: t(ahead === 1 ? 'insight.ahead.text.one' : 'insight.ahead.text.other', {
          count: ahead,
        }),
        action: 'push',
        actionLabel: t('action.push'),
      };
    }

    const withoutRemote = count((repo) => repo.state.remotes.length === 0);
    if (withoutRemote === list.length) {
      return {
        tone: 'success',
        title: t('insight.localOnly.title'),
        message: t('insight.localOnly.text'),
        action: null,
      };
    }

    return {
      tone: 'success',
      title: t('insight.upToDate.title'),
      message: t('insight.upToDate.text'),
      action: null,
    };
  });

  function showToast(kind: ToastKind, title: string, message?: string): void {
    const id = ++nextToastId;
    toasts.value.push({ id, kind, title, message });
  }

  function dismissToast(id: number): void {
    toasts.value = toasts.value.filter((toast) => toast.id !== id);
  }

  function recordGitStep(step: GitStepEvent): void {
    gitSteps.value = [...gitSteps.value, step];
    while (gitSteps.value.length > GIT_STEP_LIMIT) {
      const oldestRun = gitSteps.value[0].runId;
      const next = gitSteps.value.filter((entry) => entry.runId !== oldestRun);
      if (next.length === gitSteps.value.length) {
        gitSteps.value = gitSteps.value.slice(-50);
        break;
      }
      gitSteps.value = next;
    }
  }

  function openGitLog(): void {
    gitLogOpen.value = true;
  }

  function closeGitLog(): void {
    gitLogOpen.value = false;
  }

  function repoLabel(path: string): string {
    const parts = path.split(/[\\/]/).filter(Boolean);
    return parts[parts.length - 1] ?? path;
  }

  function summarizeResults(
    results: OperationResult[],
    successTitle: string,
    failureTitle: string,
  ): void {
    if (results.length === 0) return;

    const failures = results.filter((result) => !result.success);
    if (failures.length === 0) {
      showToast(
        'success',
        successTitle,
        t('toast.resultCount', {
          count: results.length,
          projectWord: tPlural('common.project', results.length),
        }),
      );
      return;
    }

    const details = failures
      .slice(0, 4)
      .map((failure) => {
        const firstLine = (failure.error ?? t('common.unknownError')).trim().split('\n')[0];
        return `${repoLabel(failure.repoPath)}: ${firstLine}`;
      })
      .join('\n');
    const suffix =
      failures.length > 4 ? `\n${t('common.andMore', { count: failures.length - 4 })}` : '';

    showToast(
      'error',
      t('toast.failedCount', {
        title: failureTitle,
        failed: failures.length,
        total: results.length,
      }),
      details + suffix,
    );
  }

  async function initialize(skipOnboarding = false): Promise<void> {
    await loadConfig();
    if (repositories.value.length === 0) {
      repositories.value = readRepositoryCache();
    }
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
      showToast('error', t('toast.loadConfigFailed'), String(error));
    }
  }

  async function scanRepositories(): Promise<void> {
    isScanning.value = true;
    try {
      const paths = config.value.scanPaths.filter((scanPath) => scanPath.enabled);
      if (paths.length === 0) {
        repositories.value = [];
        groupBranches.value = [];
        writeRepositoryCache([]);
        return;
      }

      const maxDepth = Math.max(...paths.map((scanPath) => scanPath.maxDepth), 3);
      const found = await backend.discoverRepositories(
        paths.map((scanPath) => scanPath.path),
        maxDepth,
      );
      repositories.value = found;
      writeRepositoryCache(found);
      await loadGroupBranches();
    } catch (error) {
      failScanActivity(String(error));
      showToast('error', t('toast.scanFailed'), String(error));
    } finally {
      isScanning.value = false;
    }
  }

  async function refreshSingleRepo(path: string): Promise<RefreshResult> {
    const current = repositories.value.find((repo) => repo.path === path);
    const groupIds = current?.groupIds ?? [];

    try {
      const updated = await backend.refreshRepository(path, groupIds);
      const index = repositories.value.findIndex((repo) => repo.path === path);
      if (index !== -1) {
        repositories.value[index] = updated;
      } else {
        repositories.value.push(updated);
        repositories.value.sort((a, b) => a.path.localeCompare(b.path));
      }

      if (!updated.isAccessible) {
        return { ok: false, error: t('status.folderNotFound') };
      }
      return { ok: true, repo: updated };
    } catch (error) {
      // A repository can disappear between a filesystem event and the refresh.
      return { ok: false, error: String(error) };
    }
  }

  async function refreshRepoWithToast(path: string): Promise<void> {
    const result = await refreshSingleRepo(path);

    if (result.ok) {
      const { repo } = result;
      showToast(
        'success',
        t('toast.refreshDone', { name: repo.name || repoLabel(path) }),
        [branchLabel(repo), ...statusBadges(repo).map((badge) => badge.label)].join(' · '),
      );
      return;
    }

    const name = repositories.value.find((repo) => repo.path === path)?.name || repoLabel(path);
    showToast(
      'error',
      t('toast.refreshFailed', { name }),
      result.error.trim().split('\n')[0] || t('common.unknownError'),
    );
  }

  async function loadGroupBranches(): Promise<void> {
    const paths = filteredRepositories.value.map((repo) => repo.path);
    const requestId = ++branchRequestId;

    if (paths.length === 0) {
      groupBranches.value = [];
      return;
    }

    isBranchListLoading.value = true;
    try {
      const branches = await backend.listGroupBranches(paths);
      if (requestId === branchRequestId) {
        groupBranches.value = branches;
      }
    } catch (error) {
      if (requestId === branchRequestId) {
        groupBranches.value = [];
        showToast('error', t('toast.branchesFailed'), String(error));
      }
    } finally {
      if (requestId === branchRequestId) {
        isBranchListLoading.value = false;
      }
    }
  }

  async function executeGroupAction(action: GroupAction): Promise<void> {
    const paths = filteredRepositories.value.map((repo) => repo.path);
    if (paths.length === 0) return;

    isLoading.value = true;
    try {
      const results = await backend.executeGroupAction(paths, action, 5);
      await Promise.all(paths.map((path) => refreshSingleRepo(path)));
      await loadGroupBranches();
      summarizeResults(results, t(ACTION_COPY[action].success), t(ACTION_COPY[action].failure));
    } catch (error) {
      showToast('error', t(ACTION_COPY[action].failed), String(error));
    } finally {
      isLoading.value = false;
    }
  }

  async function runRepoAction(path: string, action: GroupAction): Promise<void> {
    isLoading.value = true;
    try {
      const results = await backend.executeGroupAction([path], action, 1);
      await refreshSingleRepo(path);
      await loadGroupBranches();
      summarizeResults(results, t(ACTION_COPY[action].success), t(ACTION_COPY[action].failure));
    } catch (error) {
      showToast('error', t(ACTION_COPY[action].failed), String(error));
    } finally {
      isLoading.value = false;
    }
  }

  async function commitRepositories(
    messages: Record<string, string>,
  ): Promise<OperationResult[]> {
    const results: OperationResult[] = [];
    const paths = Object.keys(messages);

    for (const path of paths) {
      try {
        results.push(await backend.commitRepoChanges(path, messages[path]));
      } catch (error) {
        results.push({
          repoPath: path,
          success: false,
          output: '',
          error: String(error),
        });
      }
    }

    await Promise.all(paths.map((path) => refreshSingleRepo(path)));
    await loadGroupBranches();
    return results;
  }

  // --- AI commit messages (specs/002-ai-commit-messages, data-model E4) ---

  const commitGenActiveRunId = ref<string | null>(null);
  const commitGenRetryRunIds = new Set<string>();
  const commitGenRows = ref<Map<string, RepoDraftRow>>(new Map());
  const commitGenBuffered = ref<CommitGenEvent[]>([]);

  const commitGenRowsList = computed(() => [...commitGenRows.value.values()]);

  const commitGenGenerating = computed(() =>
    commitGenRowsList.value.some((row) => row.status === 'generating'),
  );

  const commitGenCanCommit = computed(() => {
    if (commitGenRowsList.value.length === 0) return false;
    return commitGenRowsList.value.every(
      (row) => row.status !== 'generating' && row.message.trim().length > 0,
    );
  });

  /** Session-only view of the active run; discarded when the dialog closes. */
  const commitgen = computed(() => ({
    activeRunId: commitGenActiveRunId.value,
    rows: commitGenRows.value,
    list: commitGenRowsList.value,
    generating: commitGenGenerating.value,
    canCommit: commitGenCanCommit.value,
    configured: Boolean(config.value.ai),
  }));

  function repoNameFor(path: string): string {
    const repo = repositories.value.find((entry) => entry.path === path);
    return repo?.name ?? path.split(/[\\/]/).pop() ?? path;
  }

  /** Seeds one editable row per selected repo (manual commits need no AI run). */
  function ensureCommitGenRows(paths: string[]): void {
    const next = new Map<string, RepoDraftRow>();
    for (const path of paths) {
      next.set(
        path,
        commitGenRows.value.get(path) ?? {
          path,
          name: repoNameFor(path),
          status: 'idle',
          message: '',
          dirty: false,
          error: null,
        },
      );
    }
    commitGenRows.value = next;
  }

  function setCommitGenMessage(path: string, message: string): void {
    const row = commitGenRows.value.get(path);
    if (!row) return;
    row.message = message;
    row.dirty = true;
  }

  function isCurrentCommitGenRun(runId: string): boolean {
    return runId === commitGenActiveRunId.value || commitGenRetryRunIds.has(runId);
  }

  function applyCommitGenEvent(event: CommitGenEvent): void {
    const row = commitGenRows.value.get(event.path);
    if (!row) return;

    if (event.phase === 'started') {
      row.status = 'generating';
      row.dirty = false;
      row.error = null;
      return;
    }

    if (event.phase === 'done') {
      row.status = 'ready';
      row.error = null;
      // User edits win over a late draft (US1-AC3).
      if (!row.dirty && event.draft) {
        row.message = event.draft.rendered;
      }
      return;
    }

    row.status = 'error';
    row.error = event.error ?? 'unknown error';
  }

  /** Applies queued events once their runId is known (events can beat the ACK). */
  function replayCommitGenEvents(): void {
    if (commitGenActiveRunId.value === null) return;
    const queued = commitGenBuffered.value;
    commitGenBuffered.value = [];
    for (const event of queued) {
      if (isCurrentCommitGenRun(event.runId)) applyCommitGenEvent(event);
    }
  }

  function handleCommitGenEvent(event: CommitGenEvent): void {
    if (commitGenActiveRunId.value === null) {
      if (commitGenRows.value.has(event.path) && commitGenBuffered.value.length < 100) {
        commitGenBuffered.value.push(event);
      }
      return;
    }
    if (!isCurrentCommitGenRun(event.runId)) return;
    applyCommitGenEvent(event);
  }

  async function startRun(paths: string[]): Promise<void> {
    ensureCommitGenRows(paths);
    commitGenBuffered.value = [];
    const runId = await backend.generateCommitMessages(paths);
    commitGenActiveRunId.value = runId;
    commitGenRetryRunIds.clear();
    replayCommitGenEvents();
  }

  async function retryRepo(path: string): Promise<void> {
    const row = commitGenRows.value.get(path);
    if (!row) return;
    row.status = 'generating';
    row.error = null;
    try {
      // Reuse the active run so the retry events land on the same rows
      // (contract C2/C3: the request carries `runId`).
      const runId = await backend.generateCommitMessages(
        [path],
        commitGenActiveRunId.value ?? undefined,
      );
      if (commitGenActiveRunId.value === null) {
        commitGenActiveRunId.value = runId;
      }
      commitGenRetryRunIds.add(runId);
      replayCommitGenEvents();
    } catch (error) {
      row.status = 'error';
      row.error = String(error);
    }
  }

  function resetCommitGen(): void {
    commitGenRows.value = new Map();
    commitGenActiveRunId.value = null;
    commitGenRetryRunIds.clear();
    commitGenBuffered.value = [];
  }

  async function switchGroupBranch(branch: string, autoStash = true): Promise<void> {
    const paths = filteredRepositories.value.map((repo) => repo.path);
    if (paths.length === 0) return;

    isLoading.value = true;
    try {
      const results = await backend.switchGroupBranch(paths, branch, autoStash);
      await Promise.all(paths.map((path) => refreshSingleRepo(path)));
      await loadGroupBranches();
      summarizeResults(results, t('toast.branchSwitched'), t('toast.branchSwitchFailure'));
    } catch (error) {
      showToast('error', t('toast.switchToFailed', { branch }), String(error));
    } finally {
      isLoading.value = false;
    }
  }

  async function saveGroup(group: RepoGroup): Promise<void> {
    try {
      config.value = await backend.saveRepoGroup(group);
      selectedGroupId.value = group.id;
      await loadGroupBranches();
      showToast('success', t('toast.groupSaved'), group.name);
    } catch (error) {
      showToast('error', t('toast.saveGroupFailed'), String(error));
    }
  }

  async function deleteGroup(groupId: string): Promise<void> {
    try {
      config.value = await backend.deleteRepoGroup(groupId);
      if (selectedGroupId.value === groupId) {
        selectedGroupId.value = 'all';
      }
      if (allReposFilters.value.groupId === groupId) {
        allReposFilters.value.groupId = 'all';
      }
      await loadGroupBranches();
      showToast('success', t('toast.groupDeleted'));
    } catch (error) {
      showToast('error', t('toast.deleteGroupFailed'), String(error));
    }
  }

  async function addScanPath(scanPath: ScanPath): Promise<boolean> {
    try {
      config.value = await backend.addScanPath(scanPath);
      showToast('success', t('toast.scanPathAdded'), scanPath.path);
      await scanRepositories();
      return true;
    } catch (error) {
      showToast('error', t('toast.addScanPathFailed'), String(error));
      return false;
    }
  }

  async function removeScanPath(id: string): Promise<void> {
    try {
      config.value = await backend.removeScanPath(id);
      await scanRepositories();
      showToast('success', t('toast.scanPathRemoved'));
    } catch (error) {
      showToast('error', t('toast.removeScanPathFailed'), String(error));
    }
  }

  async function toggleScanPath(scanPath: ScanPath): Promise<void> {
    await addScanPath({ ...scanPath, enabled: !scanPath.enabled });
  }

  async function openOnboarding(replay: boolean): Promise<void> {
    const onboarding = config.value.onboarding ?? defaultOnboardingState();
    onboardingReplay.value = replay || onboarding.completed;
    onboardingStep.value = replay ? 0 : Math.min(onboarding.currentStep, 2);
    onboardingDraft.value = {
      scanPath: onboarding.scanPath ?? config.value.scanPaths[0]?.path ?? '',
      ownerNames: onboarding.ownerNames.join(', '),
      groupingStyle: onboarding.groupingStyle ?? 'topLevel',
      maxDepth: config.value.scanPaths[0]?.maxDepth ?? 4,
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
            t('onboarding.projectsGroup'),
          ).map((group) => ({ ...group, included: true }))
        : [];
    onboardingPendingGroups.value = [];
    onboardingError.value = '';
    onboardingSearch.value = '';
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
      showToast('error', t('toast.saveSetupFailed'), String(error));
      throw error;
    }
  }

  async function scanOnboardingFolder(): Promise<boolean> {
    const path = onboardingDraft.value.scanPath.trim();
    if (!path) {
      onboardingError.value = t('validation.chooseFolder');
      return false;
    }

    onboardingError.value = '';
    const rawDepth = Number(onboardingDraft.value.maxDepth);
    const maxDepth = Number.isFinite(rawDepth)
      ? Math.min(Math.max(Math.round(rawDepth), 1), 12)
      : 8;
    onboardingDraft.value.maxDepth = maxDepth;
    const added = await addScanPath({
      id: 'onboarding-main',
      path,
      maxDepth,
      enabled: true,
      excludePatterns: [],
    });
    if (!added) {
      onboardingError.value = t('validation.scanFolderFailed');
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
      onboardingError.value = t('validation.chooseProject');
      return;
    }

    if (step === 2) {
      onboardingGroups.value = suggestGroups(
        onboardingSelectedPaths.value,
        onboardingDraft.value.scanPath,
        onboardingDraft.value.groupingStyle,
        t('onboarding.projectsGroup'),
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
      onboardingError.value = t('validation.keepGroup');
      return false;
    }
    if (names.some((name) => name.length === 0)) {
      onboardingError.value = t('validation.nameEveryGroup');
      return false;
    }
    if (new Set(names).size !== names.length) {
      onboardingError.value = t('validation.uniqueGroupNames');
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
      onboardingError.value = t('validation.confirmGroup');
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
      if (openGroupId) {
        selectGroup(openGroupId);
      } else {
        selectedGroupId.value = 'all';
      }
      const count = groupsToCreate.length;
      showToast(
        'success',
        t('toast.setupComplete'),
        t('toast.groupsReady', { count, groupWord: tPlural('common.group', count) }),
      );
      onboardingPendingGroups.value = [];
    } catch (error) {
      onboardingError.value = String(error);
      showToast('error', t('toast.finishSetupFailed'), String(error));
    }
  }

  async function closeOnboarding(): Promise<void> {
    try {
      await persistOnboardingProgress();
    } finally {
      onboardingOpen.value = false;
    }
  }

  function toggleAllReposStatus(status: RepoStatusFilter): void {
    const statuses = allReposFilters.value.statuses;
    allReposFilters.value.statuses = statuses.includes(status)
      ? statuses.filter((item) => item !== status)
      : [...statuses, status];
  }

  function clearAllReposFilters(): void {
    allReposFilters.value = emptyAllReposFilters();
  }

  function selectGroup(groupId: string): void {
    selectedGroupId.value = groupId;
    void loadGroupBranches();
  }

  function openAddScanPath(): void {
    scanModalOpen.value = true;
  }

  function closeScanPathModal(): void {
    scanModalOpen.value = false;
  }

  function openSaveChanges(paths?: string[]): void {
    saveModalPaths.value = paths ?? null;
    saveModalOpen.value = true;
  }

  function closeSaveChanges(): void {
    saveModalOpen.value = false;
    saveModalPaths.value = null;
    resetCommitGen();
  }

  function openHelp(): void {
    helpModalOpen.value = true;
  }

  function closeHelp(): void {
    helpModalOpen.value = false;
  }

  function openGroupEditor(group: RepoGroup | null): void {
    editingGroup.value = group;
    groupEditorOpen.value = true;
  }

  function closeGroupEditor(): void {
    groupEditorOpen.value = false;
    editingGroup.value = null;
  }

  return {
    config,
    repositories,
    selectedGroupId,
    currentGroup,
    filteredRepositories,
    shouldShowOnboarding,
    onboardingSelectedRepositories,
    allReposFiltered,
    allReposBase,
    allReposStatusCounts,
    configLoaded,
    onboardingOpen,
    onboardingReplay,
    onboardingStep,
    onboardingError,
    onboardingSearch,
    onboardingDraft,
    onboardingSelectedPaths,
    onboardingGroups,
    onboardingPendingGroups,
    allReposFilters,
    commonBranches,
    synchronizedBranch,
    cleanRepositoryCount,
    dirtyRepositories,
    conflictedRepositories,
    insight,
    groupBranches,
    isLoading,
    isScanning,
    isBranchListLoading,
    toasts,
    scanModalOpen,
    saveModalOpen,
    saveModalPaths,
    helpModalOpen,
    groupEditorOpen,
    editingGroup,
    gitSteps,
    gitLogOpen,
    scanActivity,
    scanActivityCount,
    initialize,
    loadConfig,
    scanRepositories,
    refreshSingleRepo,
    refreshRepoWithToast,
    loadGroupBranches,
    executeGroupAction,
    runRepoAction,
    commitRepositories,
    commitgen,
    commitGenRowsList,
    ensureCommitGenRows,
    setCommitGenMessage,
    handleCommitGenEvent,
    startRun,
    retryRepo,
    resetCommitGen,
    switchGroupBranch,
    saveGroup,
    deleteGroup,
    addScanPath,
    removeScanPath,
    toggleScanPath,
    openOnboarding,
    persistOnboardingProgress,
    scanOnboardingFolder,
    toggleOnboardingRepo,
    selectAllOnboardingRepos,
    clearOnboardingSelection,
    goToOnboardingStep,
    updateOnboardingGroupName,
    toggleOnboardingGroup,
    prepareOnboardingGroups,
    completeOnboarding,
    closeOnboarding,
    toggleAllReposStatus,
    clearAllReposFilters,
    selectGroup,
    openAddScanPath,
    closeScanPathModal,
    openSaveChanges,
    closeSaveChanges,
    openHelp,
    closeHelp,
    openGroupEditor,
    closeGroupEditor,
    showToast,
    dismissToast,
    recordGitStep,
    openGitLog,
    closeGitLog,
    applyScanProgress,
    failScanActivity,
    openScanModal,
    closeScanModal,
  };
});
