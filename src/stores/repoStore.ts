import { computed, ref } from 'vue';
import { defineStore } from 'pinia';
import { backend } from '../lib/backend';
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
  BranchInfo,
  GitStepEvent,
  GroupingStyle,
  OnboardingState,
  OperationResult,
  RepoGroup,
  Repository,
  ScanPath,
} from '../types/domain';

export type ToastKind = 'success' | 'error' | 'info';

export interface Toast {
  id: number;
  kind: ToastKind;
  title: string;
  message?: string;
}

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
const REPOSITORY_CACHE_KEY = 'git-repo-manager.repositories.v1';

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

const ACTION_LABELS: Record<
  'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop',
  string
> = {
  fetch: 'Check for updates',
  pull: 'Get latest',
  push: 'Send changes',
  stash: 'Set aside changes',
  stash_pop: 'Bring back set-aside changes',
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
    const projectWord = (value: number): string => (value === 1 ? 'project' : 'projects');

    const missing = count((repo) => !repo.isAccessible);
    if (missing > 0) {
      return {
        tone: 'danger',
        title: `${missing} ${projectWord(missing)} cannot be found`,
        message:
          'The folder may have been moved or renamed. Update the project folder or remove it from the scan list.',
        action: null,
      };
    }

    const conflicts = count((repo) => repo.state.hasConflicts);
    if (conflicts > 0) {
      return {
        tone: 'danger',
        title: `${conflicts} ${projectWord(conflicts)} need attention`,
        message:
          'Some files have conflicts. Open the project in your editor, fix the marked files, then click "Save to Git".',
        action: 'save',
        actionLabel: 'Review changes',
      };
    }

    const busy = count((repo) => repo.state.operationState !== 'idle');
    if (busy > 0) {
      return {
        tone: 'warning',
        title: 'A Git operation is still in progress',
        message:
          'Finish or cancel the merge, rebase, or cherry-pick in your editor before using the buttons here.',
        action: null,
      };
    }

    const detached = count((repo) => repo.state.isDetachedHead);
    if (detached > 0) {
      return {
        tone: 'warning',
        title: 'You are not on a branch',
        message:
          'Switch to a branch below before saving changes. Otherwise new work can be hard to find later.',
        action: null,
      };
    }

    const dirty = count((repo) => !repo.state.isClean);
    if (dirty > 0) {
      return {
        tone: 'warning',
        title:
          dirty === 1
            ? 'You have changes that are not saved to Git'
            : `${dirty} projects have changes that are not saved to Git`,
        message:
          'Save them to Git to create a checkpoint. Your files stay where they are, and you can keep working afterwards.',
        action: 'save',
        actionLabel: dirty === 1 ? 'Save to Git' : 'Save all to Git',
      };
    }

    const behind = count((repo) => repo.state.behindCount > 0);
    if (behind > 0) {
      return {
        tone: 'info',
        title: behind === 1 ? 'An update is available' : 'Updates are available',
        message: `${behind} ${projectWord(
          behind,
        )} can receive new work from the server. Get the latest changes before you start editing.`,
        action: 'pull',
        actionLabel: 'Get latest',
      };
    }

    const ahead = count((repo) => repo.state.aheadCount > 0);
    if (ahead > 0) {
      return {
        tone: 'info',
        title: 'You have work ready to send',
        message: `${ahead} ${projectWord(
          ahead,
        )} have saved changes that are not on the server yet. Send them when you are ready.`,
        action: 'push',
        actionLabel: 'Send changes',
      };
    }

    const withoutRemote = count((repo) => repo.state.remotes.length === 0);
    if (withoutRemote === list.length) {
      return {
        tone: 'success',
        title: 'Your work is saved on this computer',
        message:
          'This project is not connected to a server, so there is nothing to send or receive.',
        action: null,
      };
    }

    return {
      tone: 'success',
      title: 'Everything is up to date',
      message: 'No unsaved changes and no updates waiting.',
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

  function summarizeResults(results: OperationResult[], verb: string): void {
    if (results.length === 0) return;

    const failures = results.filter((result) => !result.success);
    if (failures.length === 0) {
      showToast(
        'success',
        `${verb} complete`,
        `${results.length} ${results.length === 1 ? 'project' : 'projects'} updated.`,
      );
      return;
    }

    const details = failures
      .slice(0, 4)
      .map((failure) => {
        const firstLine = (failure.error ?? 'Unknown error').trim().split('\n')[0];
        return `${repoLabel(failure.repoPath)}: ${firstLine}`;
      })
      .join('\n');
    const suffix = failures.length > 4 ? `\n…and ${failures.length - 4} more.` : '';

    showToast(
      'error',
      `${verb} failed in ${failures.length}/${results.length}`,
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
      showToast('error', 'Could not load configuration', String(error));
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
      showToast('error', 'Could not look for projects', String(error));
    } finally {
      isScanning.value = false;
    }
  }

  async function refreshSingleRepo(path: string): Promise<void> {
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
    } catch {
      // A repository can disappear between a filesystem event and the refresh.
    }
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
        showToast('error', 'Could not list branches', String(error));
      }
    } finally {
      if (requestId === branchRequestId) {
        isBranchListLoading.value = false;
      }
    }
  }

  async function executeGroupAction(
    action: 'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop',
  ): Promise<void> {
    const paths = filteredRepositories.value.map((repo) => repo.path);
    if (paths.length === 0) return;

    isLoading.value = true;
    try {
      const results = await backend.executeGroupAction(paths, action, 5);
      await Promise.all(paths.map((path) => refreshSingleRepo(path)));
      await loadGroupBranches();
      summarizeResults(results, ACTION_LABELS[action]);
    } catch (error) {
      showToast('error', `${ACTION_LABELS[action]} failed`, String(error));
    } finally {
      isLoading.value = false;
    }
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

  async function commitRepositories(
    paths: string[],
    message: string,
  ): Promise<OperationResult[]> {
    const results: OperationResult[] = [];

    for (const path of paths) {
      try {
        results.push(await backend.commitRepoChanges(path, message));
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

  async function switchGroupBranch(branch: string, autoStash = true): Promise<void> {
    const paths = filteredRepositories.value.map((repo) => repo.path);
    if (paths.length === 0) return;

    isLoading.value = true;
    try {
      const results = await backend.switchGroupBranch(paths, branch, autoStash);
      await Promise.all(paths.map((path) => refreshSingleRepo(path)));
      await loadGroupBranches();
      summarizeResults(results, `Switch to ${branch}`);
    } catch (error) {
      showToast('error', `Switch to ${branch} failed`, String(error));
    } finally {
      isLoading.value = false;
    }
  }

  async function saveGroup(group: RepoGroup): Promise<void> {
    try {
      config.value = await backend.saveRepoGroup(group);
      selectedGroupId.value = group.id;
      await loadGroupBranches();
      showToast('success', 'Group saved', group.name);
    } catch (error) {
      showToast('error', 'Could not save group', String(error));
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
      showToast('success', 'Group deleted');
    } catch (error) {
      showToast('error', 'Could not delete group', String(error));
    }
  }

  async function addScanPath(scanPath: ScanPath): Promise<boolean> {
    try {
      config.value = await backend.addScanPath(scanPath);
      showToast('success', 'Project folder added', scanPath.path);
      await scanRepositories();
      return true;
    } catch (error) {
      showToast('error', 'Could not add project folder', String(error));
      return false;
    }
  }

  async function removeScanPath(id: string): Promise<void> {
    try {
      config.value = await backend.removeScanPath(id);
      await scanRepositories();
      showToast('success', 'Project folder removed');
    } catch (error) {
      showToast('error', 'Could not remove project folder', String(error));
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
      if (openGroupId) {
        selectGroup(openGroupId);
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
    initialize,
    loadConfig,
    scanRepositories,
    refreshSingleRepo,
    loadGroupBranches,
    executeGroupAction,
    runRepoAction,
    commitRepositories,
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
  };
});
