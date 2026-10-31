import { t, tPlural } from './i18n';
import type { Repository } from '../types/domain';

export type StatusBadgeVariant = 'neutral' | 'success' | 'info' | 'warning' | 'error';
export type StatusBadgeKind = 'attention' | 'operation';

export interface StatusBadge {
  label: string;
  variant: StatusBadgeVariant;
  kind?: StatusBadgeKind;
}

export function statusBadges(repo: Repository): StatusBadge[] {
  if (!repo.isAccessible) {
    return [{ label: t('status.folderNotFound'), variant: 'error' }];
  }

  const badges: StatusBadge[] = [];

  if (repo.state.hasConflicts) {
    badges.push({ label: t('status.needsAttention'), variant: 'error', kind: 'attention' });
  }

  if (repo.state.operationState !== 'idle') {
    badges.push({
      label: operationLabel(repo.state.operationState),
      variant: 'warning',
      kind: 'operation',
    });
  }

  if (!repo.state.isClean) {
    badges.push({ label: t('status.changesNotSaved'), variant: 'warning' });
  }

  if (repo.state.behindCount > 0) {
    badges.push({
      label: t('status.updatesAvailable', {
        count: repo.state.behindCount,
        updateWord: tPlural('common.update', repo.state.behindCount),
      }),
      variant: 'info',
    });
  }

  if (repo.state.aheadCount > 0) {
    badges.push({
      label: t('status.toSend', { count: repo.state.aheadCount }),
      variant: 'success',
    });
  }

  if (repo.state.remotes.length === 0) {
    badges.push({ label: t('status.localOnly'), variant: 'neutral' });
  } else if (repo.state.isClean && repo.state.aheadCount === 0 && repo.state.behindCount === 0) {
    badges.push({ label: t('status.upToDate'), variant: 'success' });
  }

  return badges;
}

export function operationLabel(operation: string): string {
  switch (operation) {
    case 'merging':
      return t('status.mergeInProgress');
    case 'rebasing':
      return t('status.rebaseInProgress');
    case 'cherry-picking':
      return t('status.cherryPickInProgress');
    default:
      return t('status.operationInProgress');
  }
}

export function branchLabel(repo: Repository): string {
  return repo.state.isDetachedHead
    ? t('status.notOnBranch')
    : repo.state.currentBranch || t('status.unknownBranch');
}
