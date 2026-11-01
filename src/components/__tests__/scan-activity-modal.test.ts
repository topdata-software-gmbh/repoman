import { flushPromises, mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import ScanActivityModal from '../ScanActivityModal.vue';
import { useRepoStore } from '../../stores/repoStore';
import { setLanguage } from '../../lib/i18n';
import type { ScanProgressEvent } from '../../types/domain';

const RUN = '33333333-3333-4333-8333-333333333333';
const ROOT = '/tmp/scan-demo';

function started(): ScanProgressEvent {
  return { phase: 'started', runId: RUN, roots: [ROOT], startedAt: 1 };
}

function found(path: string, name: string, root = ROOT): ScanProgressEvent {
  return { phase: 'found', runId: RUN, root, path, name };
}

// TdModal teleports to <body>, so assertions query the live document.
function rows(): Element[] {
  return Array.from(document.querySelectorAll('[data-testid="activity-row"]'));
}

function rowNames(): string[] {
  return rows().map((row) => row.textContent?.replace(/\s+/g, ' ').trim() ?? '');
}

describe('ScanActivityModal (US2)', () => {
  let store: ReturnType<typeof useRepoStore>;

  beforeEach(() => {
    setActivePinia(createPinia());
    setLanguage('en');
    document.body.innerHTML = '';
    store = useRepoStore();
    store.applyScanProgress(started());
    store.openScanModal();
  });

  async function mounted() {
    return mount(ScanActivityModal, { attachTo: document.body });
  }

  it('renders rows in insertion order without duplicates (SC-003)', async () => {
    store.applyScanProgress(found(`${ROOT}/a`, 'a'));
    store.applyScanProgress(found(`${ROOT}/b`, 'b'));
    store.applyScanProgress(found(`${ROOT}/a`, 'a'));
    store.applyScanProgress(found(`${ROOT}/c`, 'c'));

    await mounted();
    expect(rows()).toHaveLength(3);
    expect(rowNames()[0]).toContain('a');
    expect(rowNames()[1]).toContain('b');
    expect(rowNames()[2]).toContain('c');
  });

  it('updates the badge of an existing row in place when inspected (FR-005)', async () => {
    store.applyScanProgress(found(`${ROOT}/theme`, 'theme'));
    await mounted();
    const badge = document.querySelector('[data-testid="activity-badge"]');
    expect(badge?.textContent?.trim()).toBe('…');

    store.applyScanProgress({
      phase: 'inspected',
      runId: RUN,
      path: `${ROOT}/theme`,
      kind: 'theme',
    });
    await flushPromises();

    expect(rows()).toHaveLength(1);
    expect(document.querySelector('[data-testid="activity-badge"]')?.textContent?.trim()).toBe(
      'Theme',
    );
  });

  it('caps the log at 500 rows with an "and N more" row while the header count stays exact (FR-013)', async () => {
    for (let index = 0; index < 505; index += 1) {
      store.applyScanProgress(found(`${ROOT}/p${index}`, `p${index}`));
    }
    expect(store.scanActivityCount).toBe(505);

    await mounted();
    expect(rows()).toHaveLength(500);
    const overflow = document.querySelector('[data-testid="overflow-row"]');
    expect(overflow?.textContent).toContain('and 5 more');
    const header = document.querySelector('[role="status"]');
    expect(header?.textContent).toContain('505 projects found');
  });

  it('elides the middle of a long directory and keeps the full path in the title (FR-014)', async () => {
    const long = `/tmp/${'x'.repeat(40)}/deep/${'y'.repeat(40)}/leaf`;
    store.applyScanProgress({
      phase: 'tick',
      runId: RUN,
      root: ROOT,
      currentDir: long,
    });

    await mounted();
    const dir = document.querySelector('[role="status"] span[title]');
    expect(dir?.getAttribute('title')).toBe(long);
    const shown = dir?.textContent ?? '';
    expect(shown).toContain('…');
    expect(shown.length).toBeLessThan(long.length);
  });

  it('names the originating root on every row (FR-010)', async () => {
    store.applyScanProgress(found(`${ROOT}/one`, 'one', ROOT));
    store.applyScanProgress(found('/topdata/two', 'two', '/topdata'));

    await mounted();
    const roots = Array.from(document.querySelectorAll('[data-testid="activity-root"]')).map(
      (element) => element.textContent?.trim(),
    );
    expect(roots).toEqual(['scan-demo', 'topdata']);
  });

  it('after done shows the summary with elapsed and keeps the log readable (US3-AC1)', async () => {
    store.applyScanProgress(found(`${ROOT}/a`, 'a'));
    store.applyScanProgress(found(`${ROOT}/b`, 'b'));
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 2, elapsedMs: 4200 });

    await mounted();
    const header = document.querySelector('[role="status"]');
    expect(header?.textContent).toContain('Scan complete');
    expect(header?.textContent).toContain('2 projects');
    // 4200 ms -> "4s"
    expect(header?.textContent).toContain('4s');
    // The log persists after the run ends.
    expect(rows()).toHaveLength(2);
  });

  it('renders the multi-root header naming the root being processed (FR-010)', async () => {
    // Restart with two roots so the active-root badge shows.
    store.applyScanProgress({
      phase: 'started',
      runId: RUN,
      roots: [ROOT, '/topdata'],
      startedAt: 1,
    });
    store.applyScanProgress({
      phase: 'tick',
      runId: RUN,
      root: '/topdata',
      currentDir: '/topdata/x',
    });
    await mounted();
    const headerRoot = document.querySelector('[data-testid="header-root"]');
    expect(headerRoot?.textContent?.trim()).toBe('topdata');
  });

  it('an error done renders the error state and the chip clears after the modal opens (US3-AC2)', async () => {
    store.applyScanProgress({
      phase: 'done',
      runId: RUN,
      foundCount: 0,
      elapsedMs: 10,
      error: 'folder disappeared',
    });

    await mounted();
    const header = document.querySelector('[role="status"]');
    expect(header?.textContent).toContain('Scan failed');
    expect(header?.textContent).toContain('folder disappeared');

    store.openScanModal();
    await flushPromises();
    expect(store.scanActivity.summaryVisible).toBe(false);
  });

  it('a second started resets the finished summary in the view (US3-AC4)', async () => {
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 1, elapsedMs: 10 });
    await mounted();
    expect(document.querySelector('[role="status"]')?.textContent).toContain('Scan complete');

    store.applyScanProgress({
      phase: 'started',
      runId: RUN,
      roots: [ROOT],
      startedAt: 1,
    });
    await flushPromises();
    expect(document.querySelector('[role="status"]')?.textContent).toContain('projects found');
    expect(document.querySelector('[role="status"]')?.textContent).not.toContain('Scan complete');
  });
});
