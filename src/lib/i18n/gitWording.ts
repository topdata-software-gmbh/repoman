import type { Language } from './index';
import type { MessageKey } from './messages.en';

/**
 * Short Git-term overlay for the plain-language catalogues.
 *
 * Values are canonical English Git words in every language, as decided by the
 * spec clarification. Only labels that name an operation belong here;
 * explanations, hints, errors and the Git log stay plain.
 */
export type GitWording = Partial<Record<MessageKey, string>>;

const gitLabels: GitWording = {
  'toolbar.saveToGit': 'Commit',
  'toolbar.getLatest': 'Pull',
  'toolbar.sendChanges': 'Push',
  'toolbar.checkForUpdates': 'Fetch',
  'toolbar.setAside': 'Stash',
  'toolbar.bringBack': 'Stash pop',
  'table.save': 'Commit',
  'table.getLatestFor': 'Pull for {name}',
  'table.sendFor': 'Push for {name}',
  'status.changesNotSaved': 'Uncommitted changes',
  'status.updatesAvailable': '{count} behind',
  'status.toSend': '{count} ahead',
  'filters.changesNotSaved': 'Uncommitted changes',
  'filters.updatesAvailable': 'Behind',
  'filters.toSend': 'Ahead',
  'save.title': 'Commit',
  'save.submit': 'Commit',
  'save.savedTitle': 'Committed',
  'help.word.save.name': 'Commit',
  'help.word.pull.name': 'Pull',
  'help.word.push.name': 'Push',
  'help.word.stash.name': 'Stash',
  'onboarding.ready.saveTitle': 'Commit',
  'onboarding.ready.pullTitle': 'Pull',
  'onboarding.ready.pushTitle': 'Push',
  'action.fetch': 'Fetch',
  'action.pull': 'Pull',
  'action.push': 'Push',
  'action.stash': 'Stash',
  'action.stashPop': 'Stash pop',
  'insight.dirty.actionOne': 'Commit',
  'insight.dirty.actionOther': 'Commit all',
  'toast.fetchDone': 'Fetch complete',
  'toast.fetchFailed': 'Fetch failed',
  'toast.fetchFailure': 'Fetch',
  'toast.pullDone': 'Pull complete',
  'toast.pullFailed': 'Pull failed',
  'toast.pullFailure': 'Pull',
  'toast.pushDone': 'Push complete',
  'toast.pushFailed': 'Push failed',
  'toast.pushFailure': 'Push',
  'toast.stashDone': 'Stash complete',
  'toast.stashFailed': 'Stash failed',
  'toast.stashFailure': 'Stash',
  'toast.stashPopDone': 'Stash pop complete',
  'toast.stashPopFailed': 'Stash pop failed',
  'toast.stashPopFailure': 'Stash pop',
};

export const gitWording: Record<Language, GitWording> = {
  en: { ...gitLabels },
  de: { ...gitLabels },
};
