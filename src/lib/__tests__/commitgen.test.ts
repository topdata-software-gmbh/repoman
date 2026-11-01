import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { useRepoStore } from '../../stores/repoStore';
import type { CommitGenEvent } from '../../types/domain';

const { generateCommitMessages } = vi.hoisted(() => ({ generateCommitMessages: vi.fn() }));

vi.mock('../../lib/backend', () => ({
  backend: {
    generateCommitMessages,
    commitRepoChanges: vi.fn(),
  },
  isTauri: false,
  backendMode: 'web',
}));

const RUN = '11111111-1111-4111-8111-111111111111';
const PATH_A = '/tmp/repos/a';
const PATH_B = '/tmp/repos/b';

function event(overrides: Partial<CommitGenEvent>): CommitGenEvent {
  return { phase: 'started', runId: RUN, path: PATH_A, ...overrides };
}

function done(rendered: string, path = PATH_A): CommitGenEvent {
  return event({
    phase: 'done',
    path,
    draft: {
      type: 'feat',
      scope: null,
      subject: rendered,
      body: null,
      rendered,
    },
  });
}

async function startedStore(paths = [PATH_A]): Promise<ReturnType<typeof useRepoStore>> {
  const store = useRepoStore();
  generateCommitMessages.mockResolvedValueOnce(RUN);
  await store.startRun(paths);
  return store;
}

describe('commitgen slice', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    generateCommitMessages.mockReset();
  });

  it('startRun seeds one idle row per selected repo and stores the runId', async () => {
    const store = await startedStore([PATH_A, PATH_B]);

    expect(store.commitgen.activeRunId).toBe(RUN);
    expect([...store.commitgen.rows.keys()]).toEqual([PATH_A, PATH_B]);
    expect(store.commitgen.rows.get(PATH_A)).toEqual({
      path: PATH_A,
      name: 'a',
      status: 'idle',
      message: '',
      dirty: false,
      error: null,
    });
    expect(store.commitgen.canCommit).toBe(false);
  });

  it('applies started, done and error transitions per path', async () => {
    const store = await startedStore([PATH_A, PATH_B]);

    store.handleCommitGenEvent(event({ phase: 'started' }));
    expect(store.commitgen.rows.get(PATH_A)?.status).toBe('generating');
    expect(store.commitgen.canCommit).toBe(false);

    store.handleCommitGenEvent(done('feat: add a thing'));
    expect(store.commitgen.rows.get(PATH_A)).toMatchObject({
      status: 'ready',
      message: 'feat: add a thing',
    });

    store.handleCommitGenEvent(event({ phase: 'error', path: PATH_B, error: 'boom' }));
    expect(store.commitgen.rows.get(PATH_B)).toMatchObject({
      status: 'error',
      error: 'boom',
    });
  });

  it('ignores events from a superseded run (late frames after close)', async () => {
    const store = await startedStore();

    store.handleCommitGenEvent(done('chore: should not land', PATH_A));
    store.handleCommitGenEvent(event({ runId: 'stale-run', phase: 'started' }));
    store.handleCommitGenEvent(event({ runId: 'stale-run', phase: 'done', draft: undefined }));

    expect(store.commitgen.rows.get(PATH_A)?.message).toBe('chore: should not land');
    expect(store.commitgen.rows.get(PATH_A)?.status).toBe('ready');
  });

  it('lets user edits win over a late draft (US1-AC3)', async () => {
    const store = await startedStore();

    store.handleCommitGenEvent(event({ phase: 'started' }));
    store.setCommitGenMessage(PATH_A, 'fix: my own words');

    store.handleCommitGenEvent(done('feat: generated words'));

    expect(store.commitgen.rows.get(PATH_A)).toMatchObject({
      status: 'ready',
      message: 'fix: my own words',
      dirty: true,
    });
  });

  it('retry reruns only the given path under a fresh run id', async () => {
    const store = await startedStore([PATH_A, PATH_B]);
    store.handleCommitGenEvent(done('feat: keep me', PATH_A));
    store.handleCommitGenEvent(event({ phase: 'error', path: PATH_B, error: 'boom' }));

    generateCommitMessages.mockResolvedValueOnce(RUN);
    await store.retryRepo(PATH_B);

    expect(generateCommitMessages).toHaveBeenLastCalledWith([PATH_B], RUN);
    expect(store.commitgen.rows.get(PATH_B)?.status).toBe('generating');

    store.handleCommitGenEvent(event({ runId: RUN, path: PATH_B, phase: 'started' }));
    store.handleCommitGenEvent(done('fix: retried', PATH_B));

    expect(store.commitgen.rows.get(PATH_B)).toMatchObject({
      status: 'ready',
      message: 'fix: retried',
    });
    expect(store.commitgen.rows.get(PATH_A)).toMatchObject({
      status: 'ready',
      message: 'feat: keep me',
    });
  });

  it('buffers events that beat the start acknowledgement', async () => {
    const store = useRepoStore();
    store.ensureCommitGenRows([PATH_A]);
    store.handleCommitGenEvent(event({ phase: 'started' }));

    generateCommitMessages.mockResolvedValueOnce(RUN);
    await store.startRun([PATH_A]);
    store.handleCommitGenEvent(done('feat: raced in'));

    expect(store.commitgen.rows.get(PATH_A)).toMatchObject({
      status: 'ready',
      message: 'feat: raced in',
    });
  });

  it('canCommit needs every row to have text and none generating (FR-006)', async () => {
    const store = await startedStore([PATH_A, PATH_B]);

    store.setCommitGenMessage(PATH_A, 'feat: a');
    expect(store.commitgen.canCommit).toBe(false);

    store.setCommitGenMessage(PATH_B, 'fix: b');
    expect(store.commitgen.canCommit).toBe(true);

    store.handleCommitGenEvent(event({ phase: 'started', path: PATH_B }));
    expect(store.commitgen.canCommit).toBe(false);
  });

  it('typed text in an error row makes it committable (US3-AC3)', async () => {
    const store = await startedStore();

    store.handleCommitGenEvent(event({ phase: 'error', error: 'provider down' }));
    expect(store.commitgen.canCommit).toBe(false);

    store.setCommitGenMessage(PATH_A, 'chore: manual fallback');

    expect(store.commitgen.canCommit).toBe(true);
  });

  it('updates interleaved events for different paths independently (US2-AC2)', async () => {
    const store = await startedStore([PATH_A, PATH_B]);

    store.handleCommitGenEvent(event({ phase: 'started', path: PATH_A }));
    store.handleCommitGenEvent(event({ phase: 'started', path: PATH_B }));
    store.handleCommitGenEvent(done('feat: alpha', PATH_A));

    expect(store.commitgen.rows.get(PATH_A)?.status).toBe('ready');
    expect(store.commitgen.rows.get(PATH_B)?.status).toBe('generating');
    expect(store.commitgen.canCommit).toBe(false);

    store.handleCommitGenEvent(done('fix: beta', PATH_B));
    expect(store.commitgen.canCommit).toBe(true);
  });

  it('an error for one path leaves sibling drafts and edits untouched (SC-004)', async () => {
    const store = await startedStore([PATH_A, PATH_B]);

    store.handleCommitGenEvent(done('feat: alpha draft', PATH_A));
    store.setCommitGenMessage(PATH_A, 'feat: alpha edited');

    store.handleCommitGenEvent(event({ phase: 'error', path: PATH_B, error: 'provider down' }));

    expect(store.commitgen.rows.get(PATH_A)).toMatchObject({
      status: 'ready',
      message: 'feat: alpha edited',
      dirty: true,
    });
    expect(store.commitgen.rows.get(PATH_B)).toMatchObject({
      status: 'error',
      message: '',
    });
  });

  it('a retry started clears only that row and its dirty flag (FR-015)', async () => {
    const store = await startedStore([PATH_A, PATH_B]);
    store.handleCommitGenEvent(done('feat: alpha', PATH_A));
    store.handleCommitGenEvent(event({ phase: 'error', path: PATH_B, error: 'boom' }));
    store.setCommitGenMessage(PATH_B, 'chore: manual b');

    generateCommitMessages.mockResolvedValueOnce(RUN);
    await store.retryRepo(PATH_B);
    store.handleCommitGenEvent(event({ phase: 'started', path: PATH_B }));

    expect(store.commitgen.rows.get(PATH_B)).toMatchObject({
      status: 'generating',
      dirty: false,
      error: null,
    });
    expect(store.commitgen.rows.get(PATH_A)?.message).toBe('feat: alpha');
  });

  it('de-selecting a repo drops its row and ignores in-flight results', async () => {
    const store = await startedStore([PATH_A, PATH_B]);
    store.handleCommitGenEvent(done('feat: alpha', PATH_A));

    // The modal re-seeds rows for the new selection (data-model E4 rule 3).
    store.ensureCommitGenRows([PATH_A]);
    store.handleCommitGenEvent(done('fix: beta late', PATH_B));

    expect(store.commitgen.rows.has(PATH_B)).toBe(false);
    expect(store.commitgen.rows.get(PATH_A)?.message).toBe('feat: alpha');
    expect(store.commitgen.canCommit).toBe(true);
  });

  it('closing the dialog discards the whole slice (FR-007)', async () => {
    const store = await startedStore();
    store.handleCommitGenEvent(done('feat: transient'));

    store.openSaveChanges();
    store.closeSaveChanges();

    expect(store.commitgen.activeRunId).toBeNull();
    expect(store.commitgen.rows.size).toBe(0);
    expect(store.commitgen.canCommit).toBe(false);
  });
});
