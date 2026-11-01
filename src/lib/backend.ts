import type {
  AppConfig,
  BranchInfo,
  CommitGenEvent,
  FileChange,
  GitStepEvent,
  OnboardingState,
  OperationResult,
  RepoGroup,
  Repository,
  ScanPath,
  ScanProgressEvent,
} from '../types/domain';

export const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const backendMode: 'desktop' | 'web' = isTauri ? 'desktop' : 'web';

const TOKEN_STORAGE_KEY = 'repoman.web-token';

function captureWebToken(): void {
  if (isTauri || typeof window === 'undefined') return;

  const url = new URL(window.location.href);
  const token = url.searchParams.get('token');
  if (!token) return;

  try {
    window.sessionStorage.setItem(TOKEN_STORAGE_KEY, token);
  } catch {
    // Session storage can be unavailable in hardened browser profiles.
  }

  url.searchParams.delete('token');
  window.history.replaceState({}, '', `${url.pathname}${url.search}${url.hash}`);
}

captureWebToken();

export function getWebToken(): string | null {
  try {
    return window.sessionStorage.getItem(TOKEN_STORAGE_KEY);
  } catch {
    return null;
  }
}

export function setWebToken(token: string): void {
  try {
    if (token) {
      window.sessionStorage.setItem(TOKEN_STORAGE_KEY, token);
    } else {
      window.sessionStorage.removeItem(TOKEN_STORAGE_KEY);
    }
  } catch {
    // Ignore storage failures; requests will be rejected by the server.
  }
}

async function tauriInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

async function httpRequest<T>(method: string, path: string, body?: unknown): Promise<T> {
  const token = getWebToken();
  const response = await fetch(path, {
    method,
    headers: {
      'Content-Type': 'application/json',
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  if (!response.ok) {
    const message = (await response.text()).trim();
    throw new Error(message || `${response.status} ${response.statusText}`);
  }

  return (await response.json()) as T;
}

function request<T>(
  tauriCommand: string,
  tauriArgs: Record<string, unknown>,
  method: string,
  path: string,
  body?: unknown,
): Promise<T> {
  return isTauri
    ? tauriInvoke<T>(tauriCommand, tauriArgs)
    : httpRequest<T>(method, path, body);
}

export const backend = {
  getConfig(): Promise<AppConfig> {
    return request<AppConfig>('get_config', {}, 'GET', '/api/config');
  },

  addScanPath(scanPath: ScanPath): Promise<AppConfig> {
    return request<AppConfig>(
      'add_scan_path',
      { scanPath },
      'POST',
      '/api/scan-paths',
      scanPath,
    );
  },

  removeScanPath(id: string): Promise<AppConfig> {
    return request<AppConfig>(
      'remove_scan_path',
      { id },
      'DELETE',
      `/api/scan-paths/${encodeURIComponent(id)}`,
    );
  },

  saveRepoGroup(group: RepoGroup): Promise<AppConfig> {
    return request<AppConfig>('save_repo_group', { group }, 'POST', '/api/groups', group);
  },

  deleteRepoGroup(groupId: string): Promise<AppConfig> {
    return request<AppConfig>(
      'delete_repo_group',
      { groupId },
      'DELETE',
      `/api/groups/${encodeURIComponent(groupId)}`,
    );
  },

  saveOnboardingState(onboarding: OnboardingState): Promise<AppConfig> {
    return request<AppConfig>(
      'save_onboarding_state',
      { onboarding },
      'POST',
      '/api/onboarding',
      onboarding,
    );
  },

  discoverRepositories(paths: string[], maxDepth: number): Promise<Repository[]> {
    return request<Repository[]>(
      'discover_repositories',
      { paths, maxDepth },
      'POST',
      '/api/repositories/discover',
      { paths, maxDepth },
    );
  },

  refreshRepository(path: string, groupIds: string[]): Promise<Repository> {
    return request<Repository>(
      'refresh_repository',
      { path, groupIds },
      'POST',
      '/api/repositories/refresh',
      { path, groupIds },
    );
  },

  listRepoChanges(path: string): Promise<FileChange[]> {
    return request<FileChange[]>(
      'list_repo_changes',
      { path },
      'POST',
      '/api/repositories/changes',
      { path },
    );
  },

  commitRepoChanges(path: string, message: string): Promise<OperationResult> {
    return request<OperationResult>(
      'commit_repo_changes',
      { path, message },
      'POST',
      '/api/repositories/commit',
      { path, message },
    );
  },

  executeGroupAction(
    repoPaths: string[],
    action: string,
    concurrencyLimit: number,
  ): Promise<OperationResult[]> {
    return request<OperationResult[]>(
      'execute_group_action',
      { repoPaths, action, concurrencyLimit },
      'POST',
      '/api/repositories/actions',
      { repoPaths, action, concurrencyLimit },
    );
  },

  switchGroupBranch(
    repoPaths: string[],
    branch: string,
    autoStash: boolean,
  ): Promise<OperationResult[]> {
    return request<OperationResult[]>(
      'switch_group_branch',
      { repoPaths, branch, autoStash },
      'POST',
      '/api/repositories/switch-branch',
      { repoPaths, branch, autoStash },
    );
  },

  generateCommitMessages(repoPaths: string[], runId?: string): Promise<string> {
    const payload = runId ? { repoPaths, runId } : { repoPaths };
    return request<{ runId: string }>(
      'generate_commit_messages',
      payload,
      'POST',
      '/api/repositories/generate-commit-messages',
      payload,
    ).then((response) => response.runId);
  },

  listGroupBranches(repoPaths: string[]): Promise<BranchInfo[]> {
    return request<BranchInfo[]>(
      'list_group_branches',
      { repoPaths },
      'POST',
      '/api/branches',
      { repoPaths },
    );
  },
};

export interface AppEventHandlers {
  onRepoChanged?: (path: string) => void;
  onGitStep?: (step: GitStepEvent) => void;
  onScanProgress?: (event: ScanProgressEvent) => void;
  onCommitGen?: (event: CommitGenEvent) => void;
}

export async function subscribeToEvents(handlers: AppEventHandlers): Promise<() => void> {
  if (isTauri) {
    const { listen } = await import('@tauri-apps/api/event');
    const unlisten = await Promise.all([
      listen<string>('repo:fs-changed', (event) => handlers.onRepoChanged?.(event.payload)),
      listen<GitStepEvent>('git:step', (event) => handlers.onGitStep?.(event.payload)),
      listen<ScanProgressEvent>('scan:progress', (event) =>
        handlers.onScanProgress?.(event.payload),
      ),
      listen<CommitGenEvent>('commitgen:repo', (event) => handlers.onCommitGen?.(event.payload)),
    ]);
    return () => unlisten.forEach((stop) => stop());
  }

  const token = getWebToken();
  const url = new URL('/api/events', window.location.href);
  if (token) url.searchParams.set('token', token);

  const source = new EventSource(url);
  const repoHandler = (event: Event) => {
    handlers.onRepoChanged?.((event as MessageEvent<string>).data);
  };
  const stepHandler = (event: Event) => {
    try {
      handlers.onGitStep?.(JSON.parse((event as MessageEvent<string>).data) as GitStepEvent);
    } catch {
      // A malformed frame must never break the live view.
    }
  };
  const scanHandler = (event: Event) => {
    try {
      handlers.onScanProgress?.(
        JSON.parse((event as MessageEvent<string>).data) as ScanProgressEvent,
      );
    } catch {
      // A malformed frame must never break the live view.
    }
  };
  const commitGenHandler = (event: Event) => {
    try {
      handlers.onCommitGen?.(
        JSON.parse((event as MessageEvent<string>).data) as CommitGenEvent,
      );
    } catch {
      // A malformed frame must never break the live view.
    }
  };
  source.addEventListener('repo-fs-changed', repoHandler as EventListener);
  source.addEventListener('git-step', stepHandler as EventListener);
  source.addEventListener('scan-progress', scanHandler as EventListener);
  source.addEventListener('commitgen-repo', commitGenHandler as EventListener);

  return () => source.close();
}
