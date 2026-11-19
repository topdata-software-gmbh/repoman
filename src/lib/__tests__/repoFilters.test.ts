import { describe, expect, it } from 'vitest';
import {
  countByStatus,
  emptyAllReposFilters,
  filterAllRepos,
  matchesStatus,
} from '../repoFilters';
import { t, setLanguage } from '../i18n';
import { setWording } from '../i18n/wording';
import type { RepoGroup, Repository } from '../../types/domain';

function makeRepo(overrides: Partial<Repository> = {}): Repository {
  const state = {
    currentBranch: 'main',
    isDetachedHead: false,
    headCommitHash: '',
    headCommitMessage: '',
    isClean: true,
    aheadCount: 0,
    behindCount: 0,
    modifiedFiles: 0,
    untrackedFiles: 0,
    stagedFiles: 0,
    hasConflicts: false,
    operationState: 'idle' as const,
    remotes: [{ name: 'origin', url: 'git@example.com:acme/repo.git' }],
  };

  return {
    id: overrides.id ?? overrides.path ?? 'repo',
    name: 'repo',
    path: '/projects/repo',
    isAccessible: true,
    groupIds: [],
    kind: 'other',
    ...overrides,
    state: { ...state, ...(overrides.state ?? {}) },
  };
}

function makeGroup(overrides: Partial<RepoGroup> = {}): RepoGroup {
  return {
    id: 'group-1',
    name: 'Group',
    color: '#38bdf8',
    repoPaths: [],
    settings: {
      syncBranching: true,
      autoStashOnSwitch: true,
      defaultBaseBranch: 'main',
    },
    ...overrides,
  };
}

describe('matchesStatus', () => {
  it('matches inaccessible, conflicted and busy repositories as needsAttention', () => {
    expect(matchesStatus(makeRepo({ isAccessible: false }), 'needsAttention')).toBe(true);
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, hasConflicts: true } }), 'needsAttention'),
    ).toBe(true);
    expect(
      matchesStatus(
        makeRepo({ state: { ...makeRepo().state, operationState: 'merging' } }),
        'needsAttention',
      ),
    ).toBe(true);
    expect(matchesStatus(makeRepo(), 'needsAttention')).toBe(false);
  });

  it('matches changesNotSaved', () => {
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, isClean: false } }), 'changesNotSaved'),
    ).toBe(true);
    expect(matchesStatus(makeRepo(), 'changesNotSaved')).toBe(false);
  });

  it('matches updatesAvailable and toSend', () => {
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, behindCount: 2 } }), 'updatesAvailable'),
    ).toBe(true);
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, aheadCount: 1 } }), 'toSend'),
    ).toBe(true);
  });

  it('matches upToDate only for clean tracked repositories', () => {
    expect(matchesStatus(makeRepo(), 'upToDate')).toBe(true);
    expect(
      matchesStatus(
        makeRepo({ state: { ...makeRepo().state, remotes: [] } }),
        'upToDate',
      ),
    ).toBe(false);
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, behindCount: 1 } }), 'upToDate'),
    ).toBe(false);
  });

  it('matches localOnly without remotes', () => {
    expect(
      matchesStatus(makeRepo({ state: { ...makeRepo().state, remotes: [] } }), 'localOnly'),
    ).toBe(true);
    expect(matchesStatus(makeRepo(), 'localOnly')).toBe(false);
  });

  it('allows a repository to match more than one status', () => {
    const repo = makeRepo({
      state: { ...makeRepo().state, isClean: false, behindCount: 1 },
    });
    expect(matchesStatus(repo, 'changesNotSaved')).toBe(true);
    expect(matchesStatus(repo, 'updatesAvailable')).toBe(true);
  });
});

describe('filterAllRepos', () => {
  const repos = [
    makeRepo({ path: '/projects/theme-a', name: 'theme-a', kind: 'theme' }),
    makeRepo({
      path: '/projects/plugin-a',
      name: 'plugin-a',
      kind: 'plugin',
      state: { ...makeRepo().state, currentBranch: 'feature/login' },
    }),
    makeRepo({ path: '/projects/other-a', name: 'other-a', kind: 'other' }),
  ];
  const groups = [makeGroup({ repoPaths: ['/projects/theme-a'] })];

  it('combines statuses with OR', () => {
    const filtered = filterAllRepos(
      repos,
      { ...emptyAllReposFilters(), statuses: ['changesNotSaved', 'localOnly'] },
      [],
    );
    expect(filtered).toHaveLength(0);
  });

  it('filters by group', () => {
    const filtered = filterAllRepos(
      repos,
      { ...emptyAllReposFilters(), groupId: 'group-1' },
      groups,
    );
    expect(filtered.map((repo) => repo.path)).toEqual(['/projects/theme-a']);
  });

  it('filters ungrouped repositories', () => {
    const filtered = filterAllRepos(
      repos,
      { ...emptyAllReposFilters(), groupId: 'ungrouped' },
      groups,
    );
    expect(filtered.map((repo) => repo.path)).toEqual([
      '/projects/plugin-a',
      '/projects/other-a',
    ]);
  });

  it('filters by kind', () => {
    const filtered = filterAllRepos(
      repos,
      { ...emptyAllReposFilters(), kind: 'plugin' },
      [],
    );
    expect(filtered.map((repo) => repo.path)).toEqual(['/projects/plugin-a']);
  });

  it('searches by name, path and branch', () => {
    expect(
      filterAllRepos(repos, { ...emptyAllReposFilters(), search: 'theme' }, []).length,
    ).toBe(1);
    expect(
      filterAllRepos(repos, { ...emptyAllReposFilters(), search: '/projects/other' }, []).length,
    ).toBe(1);
    expect(
      filterAllRepos(repos, { ...emptyAllReposFilters(), search: 'feature/login' }, []).length,
    ).toBe(1);
  });

  it('intersects all filters', () => {
    const filtered = filterAllRepos(
      repos,
      { ...emptyAllReposFilters(), statuses: ['upToDate'], kind: 'theme', groupId: 'group-1' },
      groups,
    );
    expect(filtered.map((repo) => repo.path)).toEqual(['/projects/theme-a']);
  });
});

describe('countByStatus', () => {
  it('counts matching repositories', () => {
    const repos = [
      makeRepo({ path: '/a' }),
      makeRepo({ path: '/b', state: { ...makeRepo().state, isClean: false } }),
      makeRepo({ path: '/c', state: { ...makeRepo().state, remotes: [] } }),
    ];

    expect(countByStatus(repos, 'upToDate')).toBe(1);
    expect(countByStatus(repos, 'changesNotSaved')).toBe(1);
    expect(countByStatus(repos, 'localOnly')).toBe(1);
  });
});

describe('status filter labels', () => {
  it('uses Git filter labels in Git mode and restores plain labels', () => {
    setLanguage('en');
    setWording('git');
    expect(t('filters.changesNotSaved')).toBe('Uncommitted changes');
    expect(t('filters.updatesAvailable')).toBe('Behind');
    expect(t('filters.toSend')).toBe('Ahead');

    setWording('plain');
    expect(t('filters.changesNotSaved')).toBe('Changes not saved');
    expect(t('filters.updatesAvailable')).toBe('Updates available');
    expect(t('filters.toSend')).toBe('To send');
  });
});
