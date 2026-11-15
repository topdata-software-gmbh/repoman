import { flushPromises } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useRepoStore } from '../repoStore';
import { setLanguage } from '../../lib/i18n';
import type {
  DiffFileEntry,
  FilePreview,
  ProjectComparison,
  Repository,
} from '../../types/domain';

const { reviewProject, reviewFile, commitRepoChanges, refreshRepository, listGroupBranches } =
  vi.hoisted(() => ({
    reviewProject: vi.fn(),
    reviewFile: vi.fn(),
    commitRepoChanges: vi.fn(),
    refreshRepository: vi.fn(),
    listGroupBranches: vi.fn(),
  }));

vi.mock('../../lib/backend', () => ({
  backend: {
    reviewProject,
    reviewFile,
    commitRepoChanges,
    refreshRepository,
    listGroupBranches,
  },
  isTauri: false,
  backendMode: 'web',
}));

const PATH_A = '/tmp/repos/alpha';
const PATH_B = '/tmp/repos/beta';

function entry(path: string, status: DiffFileEntry['status'] = 'modified'): DiffFileEntry {
  return {
    path,
    oldPath: null,
    status,
    kind: 'text',
    whitespaceOnly: false,
    addedLines: 1,
    removedLines: 1,
  };
}

function comparison(path: string, files: DiffFileEntry[], state: ProjectComparison['state'] = 'ready'): ProjectComparison {
  return {
    path,
    name: path.split('/').pop() ?? path,
    mode: 'unsaved',
    state,
    files,
    summary: { fileCount: files.length, addedLines: 1, removedLines: 1 },
    saved: null,
    error: null,
  };
}

function preview(path: string): FilePreview {
  return {
    path,
    oldPath: null,
    status: 'modified',
    kind: 'text',
    whitespaceOnly: false,
    addedLines: 1,
    removedLines: 1,
    notice: null,
    text: {
      rows: [{ kind: 'same', oldNumber: 1, newNumber: 1, oldText: 'a', newText: 'a' }],
      truncated: false,
      truncationReason: null,
      raw: null,
      language: null,
    },
  };
}

function makeRepo(path: string, name: string): Repository {
  return {
    id: path,
    name,
    path,
    isAccessible: true,
    groupIds: [],
    kind: 'other',
    state: {
      currentBranch: 'main',
      isDetachedHead: false,
      headCommitHash: '',
      headCommitMessage: '',
      isClean: false,
      aheadCount: 0,
      behindCount: 0,
      modifiedFiles: 1,
      untrackedFiles: 0,
      stagedFiles: 0,
      hasConflicts: false,
      operationState: 'idle',
      remotes: [],
    },
  };
}

describe('review session slice', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    reviewProject.mockReset();
    reviewFile.mockReset();
    commitRepoChanges.mockReset();
    refreshRepository.mockReset().mockImplementation((path: string) =>
      Promise.resolve(makeRepo(path, path.split('/').pop() ?? path)),
    );
    listGroupBranches.mockReset().mockResolvedValue([]);

    store = useRepoStore();
    store.repositories = [makeRepo(PATH_A, 'alpha'), makeRepo(PATH_B, 'beta')];
  });

  it('opens from a row, loads one summary and caches one preview per file', async () => {
    reviewProject.mockResolvedValue(comparison(PATH_A, [entry('a.txt'), entry('b.txt')]));
    reviewFile.mockImplementation((request: { filePath: string }) =>
      Promise.resolve(preview(request.filePath)),
    );

    store.openReviewFromRow(PATH_A, 'alpha');
    await flushPromises();

    expect(store.reviewOpen).toBe(true);
    expect(store.reviewMode).toBe('unsaved');
    expect(store.reviewOrigin).toBe('row');
    expect(store.reviewProjects).toEqual([{ path: PATH_A, name: 'alpha' }]);
    expect(reviewProject).toHaveBeenCalledTimes(1);
    expect(store.reviewActiveSelectedFile).toBe('a.txt');
    expect(store.reviewActivePreview?.path).toBe('a.txt');

    store.selectReviewFile('b.txt');
    await flushPromises();
    expect(reviewFile).toHaveBeenCalledTimes(2);

    store.selectReviewFile('a.txt');
    await flushPromises();
    expect(reviewFile).toHaveBeenCalledTimes(2);
  });

  it('keeps the empty-project state', async () => {
    reviewProject.mockResolvedValue(comparison(PATH_A, [], 'empty'));
    store.openReviewFromRow(PATH_A);
    await flushPromises();

    expect(store.reviewActiveComparison?.state).toBe('empty');
    expect(store.reviewFileEntries).toEqual([]);
    expect(store.reviewActiveSelectedFile).toBeNull();
    expect(reviewFile).not.toHaveBeenCalled();
  });

  it('closes the session but keeps captured save references', async () => {
    reviewProject.mockResolvedValue(comparison(PATH_A, [entry('a.txt')]));
    reviewFile.mockResolvedValue(preview('a.txt'));
    store.reviewSaveRefs = new Map([
      [
        'run-1',
        {
          runId: 'run-1',
          repoPath: PATH_A,
          commitHash: 'abc123',
          savedAt: '2026-11-15T00:00:00.000Z',
        },
      ],
    ]);

    store.openReviewFromRow(PATH_A);
    await flushPromises();
    store.closeReview();

    expect(store.reviewOpen).toBe(false);
    expect(store.reviewComparisons.size).toBe(0);
    expect(store.reviewPreviews.size).toBe(0);
    expect(store.reviewSelectedFileByProject.size).toBe(0);
    expect(store.reviewSaveRefs.get('run-1')?.commitHash).toBe('abc123');
  });

  it('orders and deduplicates save-changes projects and hops one index at a time', async () => {
    reviewProject.mockImplementation((request: { path: string }) =>
      Promise.resolve(comparison(request.path, [entry(`${request.path}.txt`)])),
    );
    reviewFile.mockImplementation((request: { filePath: string }) =>
      Promise.resolve(preview(request.filePath)),
    );

    store.openReviewFromSaveChanges([PATH_A, PATH_B, PATH_A]);
    await flushPromises();

    expect(store.reviewProjects.map((project) => project.path)).toEqual([PATH_A, PATH_B]);

    store.nextReviewProject();
    await flushPromises();
    expect(store.reviewActiveIndex).toBe(1);

    store.prevReviewProject();
    await flushPromises();
    expect(store.reviewActiveIndex).toBe(0);
  });

  it('remembers the selected file per project', async () => {
    reviewProject.mockImplementation((request: { path: string }) =>
      Promise.resolve(
        comparison(request.path, [entry('one.txt'), entry('two.txt')]),
      ),
    );
    reviewFile.mockImplementation((request: { filePath: string }) =>
      Promise.resolve(preview(request.filePath)),
    );

    store.openReviewFromSaveChanges([PATH_A, PATH_B]);
    await flushPromises();
    store.setReviewSelectedFile(PATH_A, 'two.txt');
    await flushPromises();

    store.nextReviewProject();
    await flushPromises();
    store.prevReviewProject();
    await flushPromises();

    expect(store.reviewActiveSelectedFile).toBe('two.txt');
  });

  it('isolates one unreadable project without failing the others', async () => {
    reviewProject.mockImplementation((request: { path: string }) => {
      if (request.path === PATH_B) return Promise.reject(new Error('not-a-repository'));
      return Promise.resolve(comparison(request.path, [entry('one.txt')]));
    });
    reviewFile.mockResolvedValue(preview('one.txt'));

    store.openReviewFromSaveChanges([PATH_A, PATH_B]);
    await flushPromises();
    store.nextReviewProject();
    await flushPromises();

    expect(store.reviewActiveComparison?.state).toBe('unreadable');
    expect(store.reviewActiveComparison?.error?.code).toContain('not-a-repository');

    store.prevReviewProject();
    await flushPromises();
    expect(store.reviewActiveComparison?.state).toBe('ready');
  });

  it('marks only projects that are part of the open session stale', async () => {
    reviewProject.mockResolvedValue(comparison(PATH_A, [entry('a.txt')]));
    reviewFile.mockResolvedValue(preview('a.txt'));

    store.openReviewFromRow(PATH_A);
    await flushPromises();

    store.markReviewStale(PATH_B);
    expect(store.reviewStaleProjects.has(PATH_B)).toBe(false);

    store.markReviewStale(PATH_A);
    expect(store.reviewStaleProjects.has(PATH_A)).toBe(true);
    expect(store.reviewIsStale).toBe(true);
  });

  it('captures a save reference only from successful commit results with both fields', async () => {
    const base = { repoPath: PATH_A, output: '', error: null };
    commitRepoChanges
      .mockResolvedValueOnce({
        ...base,
        success: true,
        runId: 'run-ok',
        commitHash: 'deadbeef',
      })
      .mockResolvedValueOnce({
        ...base,
        success: false,
        runId: 'run-failed',
        commitHash: null,
      });

    await store.commitRepositories({ [PATH_A]: 'first' });
    expect(store.reviewSaveRefs.get('run-ok')).toMatchObject({
      repoPath: PATH_A,
      commitHash: 'deadbeef',
    });

    store.reviewSaveRefs.delete('run-ok');
    await store.commitRepositories({ [PATH_A]: 'second' });
    expect(store.reviewSaveRefs.size).toBe(0);
  });

  it('opens saved-state review with the current HEAD fallback when no hash is known', async () => {
    reviewProject.mockResolvedValue(comparison(PATH_A, [], 'empty'));
    store.openReviewSaved(PATH_A, null, 'alpha');
    await flushPromises();

    expect(store.reviewMode).toBe('save');
    expect(store.reviewOrigin).toBe('activity');
    expect(reviewProject).toHaveBeenCalledWith({ path: PATH_A, mode: 'save' });
  });
});
