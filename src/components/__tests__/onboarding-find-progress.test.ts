import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import OnboardingStepFind from '../onboarding/OnboardingStepFind.vue';
import { useRepoStore } from '../../stores/repoStore';
import { setLanguage } from '../../lib/i18n';
import type { ScanProgressEvent } from '../../types/domain';

const never = vi.hoisted(() => ({ resolve: null as null | (() => void) }));

vi.mock('../../lib/backend', () => ({
  isTauri: false,
  backendMode: 'web',
  getWebToken: () => null,
  setWebToken: () => {},
  subscribeToEvents: vi.fn(),
  backend: {
    addScanPath: vi.fn(
      () =>
        new Promise<void>((resolve) => {
          never.resolve = resolve;
        }),
    ),
  },
}));

const RUN = 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa';
const ROOT = '/tmp/scan-demo';

function started(): ScanProgressEvent {
  return { phase: 'started', runId: RUN, roots: [ROOT], startedAt: 1 };
}

describe('OnboardingStepFind live progress (US1)', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    never.resolve = null;
    store = useRepoStore();
    store.onboardingDraft.scanPath = ROOT;
    store.onboardingDraft.maxDepth = 6;
  });

  async function mounted() {
    const wrapper = mount(OnboardingStepFind);
    await wrapper.find('form').trigger('submit');
    await flushPromises();
    return wrapper;
  }

  it('shows the static fallback until this run started', async () => {
    const wrapper = await mounted();
    expect(store.scanActivity.runId).toBe('');
    expect(wrapper.get('[role="status"]').text()).toContain('Looking for projects');
    wrapper.unmount();
  });

  it('renders the current dir and found count and updates on tick/found (FR-001, FR-002)', async () => {
    const wrapper = await mounted();

    store.applyScanProgress(started());
    await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain(ROOT);
    expect(wrapper.get('[role="status"]').text()).toContain('0 projects found');

    store.applyScanProgress({
      phase: 'found',
      runId: RUN,
      root: ROOT,
      path: `${ROOT}/proj-1`,
      name: 'proj-1',
    });
    await flushPromises();
    expect(wrapper.get('[role="status"]').text()).toContain('1 projects found');

    store.applyScanProgress({
      phase: 'tick',
      runId: RUN,
      root: ROOT,
      currentDir: `${ROOT}/deep/a/b`,
    });
    await flushPromises();
    const line = wrapper.get('[role="status"]').text();
    expect(line).toContain(`${ROOT}/deep/a/b`);
    expect(line).toContain('1 projects found');

    wrapper.unmount();
  });
});
