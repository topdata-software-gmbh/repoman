import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useRepoStore } from '../repoStore';
import { setLanguage } from '../../lib/i18n';
import { backend } from '../../lib/backend';
import type { GitRepoState, Repository } from '../../types/domain';

vi.mock('../../lib/backend', () => ({
  backend: {
    refreshRepository: vi.fn(),
  },
}));

type RepoOverrides = Omit<Partial<Repository>, 'state'> & { state?: Partial<GitRepoState> };

export function makeRepo(overrides: RepoOverrides = {}): Repository {
  const { state, ...rest } = overrides;
  return {
    id: 'repo-1',
    name: 'demo-project',
    path: '/repos/demo-project',
    isAccessible: true,
    groupIds: [],
    kind: 'other',
    ...rest,
    state: {
      currentBranch: 'main',
      isDetachedHead: false,
      headCommitHash: 'abc123',
      headCommitMessage: 'Initial commit',
      isClean: true,
      aheadCount: 0,
      behindCount: 0,
      modifiedFiles: 0,
      untrackedFiles: 0,
      stagedFiles: 0,
      hasConflicts: false,
      operationState: 'idle',
      remotes: [],
      ...state,
    },
  };
}

describe('refresh repo toast', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    vi.mocked(backend.refreshRepository).mockReset();
    store = useRepoStore();
  });

  describe('US1: successful manual refresh', () => {
    it('shows one German success toast with branch and dirty status (FR-001–FR-003, SC-001)', async () => {
      setLanguage('de');
      const repo = makeRepo({
        state: {
          isClean: false,
          remotes: [{ name: 'origin', url: 'git@example.com:demo.git' }],
        },
      });
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0]).toMatchObject({
        kind: 'success',
        title: 'demo-project aktualisiert',
        message: 'main · Änderungen nicht gespeichert',
      });
    });

    it('shows the up-to-date label for a clean repository with a remote', async () => {
      const repo = makeRepo({
        state: { remotes: [{ name: 'origin', url: 'git@example.com:demo.git' }] },
      });
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0]).toMatchObject({
        kind: 'success',
        title: 'demo-project refreshed',
        message: 'main · Up to date',
      });
    });

    it('uses the English catalogue when the language is English', async () => {
      setLanguage('en');
      const repo = makeRepo({
        state: {
          isClean: false,
          remotes: [{ name: 'origin', url: 'git@example.com:demo.git' }],
        },
      });
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0]).toMatchObject({
        kind: 'success',
        title: 'demo-project refreshed',
        message: 'main · Changes not saved',
      });
    });

    it('shows the local-only label for a repository without remotes', async () => {
      setLanguage('de');
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0].message).toBe('main · Nur auf diesem Computer');
    });

    it('appends an unknown path and falls back to the folder name for the title', async () => {
      const repo = makeRepo({ id: 'repo-2', name: '', path: '/repos/new-project' });
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await store.refreshRepoWithToast(repo.path);

      expect(store.repositories).toHaveLength(1);
      expect(store.repositories[0].path).toBe('/repos/new-project');
      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0].title).toBe('new-project refreshed');
    });
  });

  describe('US2: failed manual refresh', () => {
    it('maps an inaccessible repository to the folder-not-found message and keeps the row (FR-004, FR-005, FR-009)', async () => {
      setLanguage('de');
      const repo = makeRepo();
      const inaccessible = makeRepo({ isAccessible: false });
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(inaccessible);

      await expect(store.refreshRepoWithToast(repo.path)).resolves.toBeUndefined();

      expect(store.repositories).toHaveLength(1);
      expect(store.repositories[0].isAccessible).toBe(false);
      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0]).toMatchObject({
        kind: 'error',
        title: 'demo-project konnte nicht aktualisiert werden',
        message: 'Ordner nicht gefunden',
      });
    });

    it('uses only the trimmed first line of a multi-line error (FR-006)', async () => {
      setLanguage('en');
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockRejectedValue('fatal: boom\ndetails');

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0]).toMatchObject({
        kind: 'error',
        title: 'Could not refresh demo-project',
        message: 'fatal: boom',
      });
    });

    it('falls back to the unknown-error label when the error text is blank', async () => {
      setLanguage('en');
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockRejectedValue('   ');

      await store.refreshRepoWithToast(repo.path);

      expect(store.toasts).toHaveLength(1);
      expect(store.toasts[0].message).toBe('Unknown error');
    });
  });

  describe('US3: background refresh stays quiet', () => {
    it('adds no toast for an accessible background refresh', async () => {
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(repo);

      await expect(store.refreshSingleRepo(repo.path)).resolves.toEqual({ ok: true, repo });

      expect(store.toasts).toHaveLength(0);
    });

    it('adds no toast for an inaccessible background refresh', async () => {
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockResolvedValue(
        makeRepo({ isAccessible: false }),
      );

      await expect(store.refreshSingleRepo(repo.path)).resolves.toEqual({
        ok: false,
        error: 'Folder not found',
      });

      expect(store.toasts).toHaveLength(0);
    });

    it('leaves repositories unchanged and resolves when a background refresh rejects', async () => {
      const repo = makeRepo();
      store.repositories = [repo];
      const before = JSON.stringify(store.repositories);
      vi.mocked(backend.refreshRepository).mockRejectedValue('boom');

      await expect(store.refreshSingleRepo(repo.path)).resolves.toEqual({
        ok: false,
        error: 'boom',
      });

      expect(store.toasts).toHaveLength(0);
      expect(JSON.stringify(store.repositories)).toBe(before);
    });

    it('does not produce an unhandled rejection in the fire-and-forget watcher path', async () => {
      const repo = makeRepo();
      store.repositories = [repo];
      vi.mocked(backend.refreshRepository).mockRejectedValue('watcher failure');
      const unhandled = vi.fn();
      const proc = (
        globalThis as unknown as {
          process: {
            on: (event: string, listener: () => void) => void;
            off: (event: string, listener: () => void) => void;
          };
        }
      ).process;
      proc.on('unhandledRejection', unhandled);

      try {
        void store.refreshSingleRepo(repo.path);
        await new Promise((resolve) => setTimeout(resolve, 0));
      } finally {
        proc.off('unhandledRejection', unhandled);
      }

      expect(unhandled).not.toHaveBeenCalled();
      expect(store.toasts).toHaveLength(0);
    });
  });
});
