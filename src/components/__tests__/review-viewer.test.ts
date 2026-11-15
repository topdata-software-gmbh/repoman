import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import ReviewViewer from '../ReviewViewer.vue';
import { useRepoStore } from '../../stores/repoStore';
import { setLanguage } from '../../lib/i18n';
import type {
  DiffFileEntry,
  FilePreview,
  ProjectComparison,
  ProjectComparisonState,
} from '../../types/domain';

const { reviewProject, reviewFile } = vi.hoisted(() => ({
  reviewProject: vi.fn(),
  reviewFile: vi.fn(),
}));

vi.mock('../../lib/backend', () => ({
  backend: { reviewProject, reviewFile },
  isTauri: false,
  backendMode: 'web',
}));

const PATH_A = '/tmp/repos/alpha';
const PATH_B = '/tmp/repos/beta';

function entry(path: string, overrides: Partial<DiffFileEntry> = {}): DiffFileEntry {
  return {
    path,
    oldPath: null,
    status: 'modified',
    kind: 'text',
    whitespaceOnly: false,
    addedLines: 2,
    removedLines: 1,
    ...overrides,
  };
}

function comparison(
  path: string,
  files: DiffFileEntry[],
  state: ProjectComparisonState = 'ready',
  code?: string,
): ProjectComparison {
  return {
    path,
    name: path.split('/').pop() ?? path,
    mode: 'unsaved',
    state,
    files: state === 'ready' ? files : [],
    summary: { fileCount: files.length, addedLines: 2, removedLines: 1 },
    saved: null,
    error: code ? { code } : null,
  };
}

function textPreview(path: string): FilePreview {
  return {
    path,
    oldPath: null,
    status: 'modified',
    kind: 'text',
    whitespaceOnly: false,
    addedLines: 2,
    removedLines: 1,
    notice: null,
    text: {
      rows: [{ kind: 'changed', oldNumber: 1, newNumber: 1, oldText: 'old', newText: 'new' }],
      truncated: false,
      truncationReason: null,
      raw: null,
      language: 'typescript',
    },
  };
}

function binaryPreview(path: string): FilePreview {
  return {
    path,
    oldPath: null,
    status: 'modified',
    kind: 'binary',
    whitespaceOnly: false,
    addedLines: 0,
    removedLines: 0,
    notice: 'unsupported',
    binary: { oldBytes: 1024, newBytes: 2048 },
  };
}

function key(project: string, file: string): string {
  return `${project}\u0000${file}`;
}

function bodyText(): string {
  return document.body.textContent ?? '';
}

function buttonByLabel(label: string): HTMLButtonElement | undefined {
  return Array.from(document.querySelectorAll('button')).find(
    (button) => button.getAttribute('aria-label') === label,
  );
}

describe('ReviewViewer', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    document.body.innerHTML = '';
    reviewProject.mockReset();
    reviewFile.mockReset();
    reviewFile.mockImplementation((request: { filePath: string }) =>
      Promise.resolve(textPreview(request.filePath)),
    );

    store = useRepoStore();
    store.reviewOpen = true;
    store.reviewProjects = [
      { path: PATH_A, name: 'alpha' },
      { path: PATH_B, name: 'beta' },
    ];
    store.reviewActiveIndex = 0;
    store.reviewComparisons = new Map([[PATH_A, comparison(PATH_A, [entry('a.ts')])]]);
    store.reviewSelectedFileByProject = new Map([[PATH_A, 'a.ts']]);
    store.reviewPreviews = new Map([[key(PATH_A, 'a.ts'), textPreview('a.ts')]]);
  });

  it('announces the project position in a polite status region and hops one project', async () => {
    store.reviewComparisons = new Map([
      [PATH_A, comparison(PATH_A, [entry('a.ts')])],
      [PATH_B, comparison(PATH_B, [entry('b.ts')])],
    ]);
    mount(ReviewViewer, { attachTo: document.body });
    await flushPromises();

    expect(bodyText()).toContain('Project 1 of 2');
    const status = document.querySelector('[role="status"][aria-live="polite"]');
    expect(status?.textContent).toContain('Project 1 of 2');

    buttonByLabel('Next project')?.click();
    await flushPromises();
    expect(store.reviewActiveIndex).toBe(1);
    expect(bodyText()).toContain('Project 2 of 2');

    buttonByLabel('Previous project')?.click();
    await flushPromises();
    expect(store.reviewActiveIndex).toBe(0);
  });

  it('shows an isolated error for the unreadable project only', async () => {
    store.reviewActiveIndex = 1;
    store.reviewComparisons = new Map([
      [PATH_A, comparison(PATH_A, [entry('a.ts')])],
      [PATH_B, comparison(PATH_B, [], 'unreadable', 'not-a-repository')],
    ]);
    mount(ReviewViewer, { attachTo: document.body });
    await flushPromises();

    expect(bodyText()).toContain('This project cannot be opened');
    expect(bodyText()).toContain('This folder is no longer a Git project');
  });

  it('offers a refresh prompt for stale content without replacing it automatically', async () => {
    store.reviewStaleProjects = new Set([PATH_A]);
    reviewProject.mockResolvedValue(comparison(PATH_A, [entry('a.ts')]));
    mount(ReviewViewer, { attachTo: document.body });
    await flushPromises();

    expect(bodyText()).toContain('The changes have changed');
    expect(bodyText()).toContain('a.ts');
    expect(store.reviewComparisons.get(PATH_A)?.state).toBe('ready');

    const refresh = Array.from(document.querySelectorAll('button')).find((button) =>
      button.textContent?.includes('Refresh'),
    );
    refresh?.click();
    await flushPromises();

    expect(reviewProject).toHaveBeenCalledTimes(1);
    expect(store.reviewStaleProjects.has(PATH_A)).toBe(false);
  });

  it('describes a binary change with its size change and no preview', async () => {
    store.reviewComparisons = new Map([
      [PATH_A, comparison(PATH_A, [entry('vendor/library.dll', { kind: 'binary' })])],
    ]);
    store.reviewSelectedFileByProject = new Map([[PATH_A, 'vendor/library.dll']]);
    store.reviewPreviews = new Map([
      [key(PATH_A, 'vendor/library.dll'), binaryPreview('vendor/library.dll')],
    ]);
    mount(ReviewViewer, { attachTo: document.body });
    await flushPromises();

    expect(bodyText()).toContain('Binary file changed');
    expect(bodyText()).toContain('1.0 KB');
    expect(bodyText()).toContain('2.0 KB');
    expect(document.querySelectorAll('img')).toHaveLength(0);
  });

  it('shows the friendly empty state for a readable project with no changes', async () => {
    store.reviewComparisons = new Map([[PATH_A, comparison(PATH_A, [], 'empty')]]);
    store.reviewSelectedFileByProject = new Map();
    store.reviewPreviews = new Map();
    mount(ReviewViewer, { attachTo: document.body });
    await flushPromises();

    expect(bodyText()).toContain('No unsaved changes');
  });
});
