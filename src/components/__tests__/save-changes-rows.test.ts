import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import SaveChangesModal from '../SaveChangesModal.vue';
import { useRepoStore } from '../../stores/repoStore';
import { setLanguage } from '../../lib/i18n';
import type { Repository } from '../../types/domain';

const {
  generateCommitMessages,
  commitRepoChanges,
  listRepoChanges,
  refreshRepository,
  listGroupBranches,
} = vi.hoisted(() => ({
  generateCommitMessages: vi.fn(),
  commitRepoChanges: vi.fn(),
  listRepoChanges: vi.fn(),
  refreshRepository: vi.fn(),
  listGroupBranches: vi.fn(),
}));

vi.mock('../../lib/backend', () => ({
  backend: {
    generateCommitMessages,
    commitRepoChanges,
    listRepoChanges,
    refreshRepository,
    listGroupBranches,
  },
  isTauri: false,
  backendMode: 'web',
}));

const RUN = '44444444-4444-4444-8444-444444444444';
const PATH_A = '/tmp/repos/alpha';
const PATH_B = '/tmp/repos/beta';

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

function buttons(): HTMLButtonElement[] {
  return Array.from(document.querySelectorAll('button'));
}

function buttonByText(text: string): HTMLButtonElement | undefined {
  return buttons().find((button) => button.textContent?.includes(text));
}

function textareas(): HTMLTextAreaElement[] {
  return Array.from(document.querySelectorAll('textarea'));
}

async function typeInto(area: HTMLTextAreaElement, value: string): Promise<void> {
  area.value = value;
  area.dispatchEvent(new Event('input'));
  await flushPromises();
}

describe('SaveChangesModal per-repo rows (US1)', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    document.body.innerHTML = '';
    generateCommitMessages.mockReset().mockResolvedValue(RUN);
    commitRepoChanges
      .mockReset()
      .mockImplementation((path: string) =>
        Promise.resolve({ repoPath: path, success: true, output: '', error: null }),
      );
    listRepoChanges.mockReset().mockResolvedValue([]);
    refreshRepository
      .mockReset()
      .mockImplementation((path: string) =>
        Promise.resolve(makeRepo(path, path.split('/').pop() ?? path)),
      );
    listGroupBranches.mockReset().mockResolvedValue([]);

    store = useRepoStore();
    store.config = {
      scanPaths: [],
      groups: [],
      onboarding: {
        completed: true,
        started: true,
        currentStep: 4,
        scanPath: null,
        ownerNames: [],
        groupingStyle: 'topLevel',
        selectedRepoPaths: [],
      },
      ai: {
        baseUrl: 'http://localhost:11434/v1',
        model: 'test-model',
        apiKeyEnv: 'OPENAI_API_KEY',
        maxToolCalls: 12,
        diffBudgetChars: 40000,
      },
    };
    store.repositories = [makeRepo(PATH_A, 'alpha'), makeRepo(PATH_B, 'beta')];
  });

  async function mounted(): Promise<void> {
    mount(SaveChangesModal, { attachTo: document.body });
    store.openSaveChanges();
    await flushPromises();
  }

  it('renders one row per selected repo and generates for all of them', async () => {
    await mounted();

    expect(textareas()).toHaveLength(2);

    await buttonByText('Suggest with AI')?.click();
    await flushPromises();

    expect(generateCommitMessages).toHaveBeenCalledWith([PATH_A, PATH_B]);
  });

  it('keeps commit blocked while drafting, then commits each row text', async () => {
    await mounted();

    await buttonByText('Suggest with AI')?.click();
    await flushPromises();

    store.handleCommitGenEvent({
      phase: 'started',
      runId: RUN,
      path: PATH_A,
    });
    store.handleCommitGenEvent({
      phase: 'started',
      runId: RUN,
      path: PATH_B,
    });
    await flushPromises();

    expect(commitRepoChanges).not.toHaveBeenCalled();
    expect(buttonByText('Save to Git')?.disabled).toBe(true);

    store.handleCommitGenEvent({
      phase: 'done',
      runId: RUN,
      path: PATH_A,
      draft: {
        type: 'feat',
        scope: null,
        subject: 'add alpha',
        body: null,
        rendered: 'feat: add alpha',
      },
    });
    store.handleCommitGenEvent({
      phase: 'error',
      runId: RUN,
      path: PATH_B,
      error: 'provider down',
    });
    await flushPromises();

    const areas = textareas();
    expect(areas[0]?.value).toBe('feat: add alpha');
    expect(buttonByText('Save to Git')?.disabled).toBe(true);

    await typeInto(areas[0]!, 'feat: alpha edited');
    await typeInto(areas[1]!, 'chore: manual beta');
    expect(buttonByText('Save to Git')?.disabled).toBe(false);

    await buttonByText('Save to Git')?.click();
    await flushPromises();

    expect(commitRepoChanges).toHaveBeenCalledWith(PATH_A, 'feat: alpha edited');
    expect(commitRepoChanges).toHaveBeenCalledWith(PATH_B, 'chore: manual beta');
    expect(commitRepoChanges).toHaveBeenCalledTimes(2);
    expect(store.saveModalOpen).toBe(false);
  });

  it('generation alone never commits (US1-AC4)', async () => {
    await mounted();

    await buttonByText('Suggest with AI')?.click();
    await flushPromises();

    store.handleCommitGenEvent({
      phase: 'done',
      runId: RUN,
      path: PATH_A,
      draft: {
        type: 'fix',
        scope: null,
        subject: 'repair alpha',
        body: null,
        rendered: 'fix: repair alpha',
      },
    });
    await flushPromises();

    expect(commitRepoChanges).not.toHaveBeenCalled();
  });
});
