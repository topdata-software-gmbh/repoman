import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useRepoStore } from '../../stores/repoStore';
import type { ScanProgressEvent } from '../../types/domain';

const RUN = '11111111-1111-4111-8111-111111111111';
const OTHER_RUN = '22222222-2222-4222-8222-222222222222';

function started(overrides: Partial<ScanProgressEvent> = {}): ScanProgressEvent {
  return {
    phase: 'started',
    runId: RUN,
    roots: ['/tmp/scan-demo'],
    startedAt: 1_000,
    ...overrides,
  };
}

function found(path: string, name: string, root = '/tmp/scan-demo'): ScanProgressEvent {
  return { phase: 'found', runId: RUN, root, path, name };
}

describe('scanActivity reducer', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it('resets the whole slice on every started (FR-001, FR-009)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/old', 'old'));
    expect(store.scanActivity.entries.size).toBe(1);

    store.applyScanProgress(started({ runId: OTHER_RUN, roots: ['/other'] }));

    expect(store.scanActivity.runId).toBe(OTHER_RUN);
    expect(store.scanActivity.roots).toEqual(['/other']);
    expect(store.scanActivity.running).toBe(true);
    expect(store.scanActivity.finished).toBeNull();
    expect(store.scanActivity.currentDir).toBeNull();
    expect(store.scanActivity.entries.size).toBe(0);
  });

  it('inserts each found path once (SC-003: no repository appears twice)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/a', 'a'));
    store.applyScanProgress(found('/tmp/scan-demo/a', 'a'));
    store.applyScanProgress(found('/tmp/scan-demo/b', 'b'));

    expect(store.scanActivity.entries.size).toBe(2);
    expect(store.scanActivity.entries.get('/tmp/scan-demo/a')).toEqual({
      name: 'a',
      root: '/tmp/scan-demo',
      kind: null,
      inspected: false,
    });
  });

  it('tick sets the current directory (FR-001)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress({ phase: 'tick', runId: RUN, root: '/tmp/scan-demo', currentDir: '/tmp/scan-demo/deep/a' });

    expect(store.scanActivity.currentDir).toBe('/tmp/scan-demo/deep/a');
  });

  it('inspected updates the existing entry in place and never appends (FR-005)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/theme', 'theme'));
    store.applyScanProgress({ phase: 'inspected', runId: RUN, path: '/tmp/scan-demo/theme', kind: 'theme' });
    store.applyScanProgress({ phase: 'inspected', runId: RUN, path: '/never/found', kind: 'plugin' });

    expect(store.scanActivity.entries.size).toBe(1);
    expect(store.scanActivity.entries.get('/tmp/scan-demo/theme')).toEqual({
      name: 'theme',
      root: '/tmp/scan-demo',
      kind: 'theme',
      inspected: true,
    });
  });

  it('done finalizes the run and reconciles the count with foundCount (FR-002, R5)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/a', 'a'));
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 9, elapsedMs: 2500 });

    expect(store.scanActivity.running).toBe(false);
    expect(store.scanActivity.finished).toEqual({ count: 9, elapsedMs: 2500, error: null });
    expect(store.scanActivity.summaryVisible).toBe(true);
    // Header count = max(entries.size, foundCount) even though events were lost.
    expect(store.scanActivityCount).toBe(9);
  });

  it('ignores events from another run id (FR-009)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/stale', 'stale', '/tmp/scan-demo'));
    store.applyScanProgress({ ...found('/other/a', 'a'), runId: OTHER_RUN });
    store.applyScanProgress({ phase: 'tick', runId: OTHER_RUN, currentDir: '/other' });
    store.applyScanProgress({ phase: 'done', runId: OTHER_RUN, foundCount: 3 });

    expect(store.scanActivity.entries.size).toBe(1);
    expect(store.scanActivity.currentDir).toBeNull();
    expect(store.scanActivity.running).toBe(true);
    expect(store.scanActivity.finished).toBeNull();
  });

  it('never reverts a terminal state on a late event of the same run', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/a', 'a'));
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 1, elapsedMs: 10 });
    store.applyScanProgress({ phase: 'tick', runId: RUN, currentDir: '/tmp/scan-demo/late' });
    store.applyScanProgress(found('/tmp/scan-demo/late', 'late'));

    expect(store.scanActivity.running).toBe(false);
    expect(store.scanActivity.currentDir).toBeNull();
    expect(store.scanActivity.entries.size).toBe(1);
    expect(store.scanActivity.finished?.count).toBe(1);
  });

  it('a second started clears a displayed summary (FR-009, US3-AC4)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 0, elapsedMs: 10 });
    expect(store.scanActivity.summaryVisible).toBe(true);

    store.applyScanProgress(started({ runId: OTHER_RUN }));

    expect(store.scanActivity.summaryVisible).toBe(false);
    expect(store.scanActivity.finished).toBeNull();
    expect(store.scanActivity.running).toBe(true);
    expect(store.scanActivity.runId).toBe(OTHER_RUN);
  });

  it('failScanActivity ends a running scan when the promise rejects (FR-008, SC-006)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress(found('/tmp/scan-demo/a', 'a'));
    store.failScanActivity('network down');

    expect(store.scanActivity.running).toBe(false);
    expect(store.scanActivity.finished?.error).toBe('network down');
    expect(store.scanActivityCount).toBe(1);
  });

  it('failScanActivity keeps a completed run untouched', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 2, elapsedMs: 10 });
    store.failScanActivity('late rejection');

    expect(store.scanActivity.finished).toEqual({ count: 2, elapsedMs: 10, error: null });
  });

  it('runs the elapsed ticker only while running (R7)', () => {
    vi.useFakeTimers();
    try {
      const store = useRepoStore();
      store.applyScanProgress(started({ startedAt: Date.now() - 5_000 }));
      vi.advanceTimersByTime(1_000);
      expect(store.scanActivity.elapsedMs).toBeGreaterThanOrEqual(6_000);

      store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 0, elapsedMs: 5_000 });
      const afterDone = store.scanActivity.elapsedMs;
      vi.advanceTimersByTime(5_000);
      expect(store.scanActivity.elapsedMs).toBe(afterDone);
    } finally {
      vi.useRealTimers();
    }
  });

  it('opening the modal shows the run and hides the chip (FR-003, FR-015)', () => {
    const store = useRepoStore();
    store.applyScanProgress(started());
    store.openScanModal();
    expect(store.scanActivity.modalOpen).toBe(true);

    store.applyScanProgress({ phase: 'done', runId: RUN, foundCount: 0, elapsedMs: 1 });
    store.closeScanModal();
    expect(store.scanActivity.modalOpen).toBe(false);
  });
});
