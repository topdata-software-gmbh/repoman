import { beforeEach, describe, expect, it } from 'vitest';
import { setLanguage } from '../i18n';
import { setWording } from '../i18n/wording';
import { statusBadges } from '../repoStatus';
import type { GitRepoState, Repository } from '../../types/domain';

beforeEach(() => {
  setLanguage('en');
  setWording('plain');
});

function repo(overrides: Partial<GitRepoState> = {}): Repository {
  return {
    id: 'repo',
    name: 'repo',
    path: '/repo',
    kind: 'other',
    isAccessible: true,
    groupIds: [],
    state: {
      currentBranch: 'main',
      isDetachedHead: false,
      headCommitHash: 'abc',
      headCommitMessage: 'msg',
      isClean: true,
      aheadCount: 0,
      behindCount: 0,
      modifiedFiles: 0,
      untrackedFiles: 0,
      stagedFiles: 0,
      hasConflicts: false,
      operationState: 'idle',
      remotes: [],
      ...overrides,
    },
  };
}

describe('statusBadges', () => {
  it('uses error for inaccessible repos', () => {
    const value = { ...repo(), isAccessible: false } as Repository;
    expect(statusBadges(value)[0]?.variant).toBe('error');
  });

  it('uses semantic variants for cleanliness and sync state', () => {
    expect(statusBadges(repo())[0]).toMatchObject({ label: 'On this computer only', variant: 'neutral' });
    expect(
      statusBadges(repo({ remotes: [{ name: 'origin', url: 'git@example.com' }] }))[0],
    ).toMatchObject({
      label: 'Up to date',
      variant: 'success',
    });
    expect(statusBadges(repo({ isClean: false }))[0]).toMatchObject({
      label: 'Changes not saved',
      variant: 'warning',
    });
    expect(statusBadges(repo({ behindCount: 2 }))[0]).toMatchObject({
      label: '2 updates available',
      variant: 'info',
    });
  });

  it('translates badges after a language switch', () => {
    setLanguage('de');
    expect(statusBadges(repo())[0]).toMatchObject({
      label: 'Nur auf diesem Computer',
      variant: 'neutral',
    });
  });

  it('uses Git badge labels in Git mode and restores plain labels', () => {
    setWording('git');
    expect(statusBadges(repo({ isClean: false }))[0]).toMatchObject({
      label: 'Uncommitted changes',
      variant: 'warning',
    });
    expect(statusBadges(repo({ behindCount: 2 }))[0]).toMatchObject({
      label: '2 behind',
      variant: 'info',
    });

    const ahead = statusBadges(
      repo({
        isClean: true,
        aheadCount: 2,
        remotes: [{ name: 'origin', url: 'git@example.com' }],
      }),
    );
    expect(ahead[0]).toMatchObject({ label: '2 ahead', variant: 'success' });

    setWording('plain');
    expect(statusBadges(repo({ isClean: false }))[0]).toMatchObject({
      label: 'Changes not saved',
    });
    expect(statusBadges(repo({ behindCount: 2 }))[0]).toMatchObject({
      label: '2 updates available',
    });
  });
});
