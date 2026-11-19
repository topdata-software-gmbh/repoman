import { beforeEach, describe, expect, it } from 'vitest';
import { operationHint, setLanguage, t } from '../i18n';
import {
  WORDING_STORAGE_KEY,
  setWording,
  useWording,
  wording,
  type Wording,
  type WordingStorage,
} from '../i18n/wording';

beforeEach(() => {
  setLanguage('en');
  setWording('plain');
  window.localStorage?.clear();
});

describe('operationHint', () => {
  it('appends the canonical Git command in plain mode', () => {
    expect(operationHint('toolbar.saveHint', 'commit')).toBe(
      `${t('toolbar.saveHint')} (git commit)`,
    );
    expect(operationHint('toolbar.getLatestHint', 'pull')).toBe(
      `${t('toolbar.getLatestHint')} (git pull)`,
    );
    expect(operationHint('toolbar.sendHint', 'push')).toBe(
      `${t('toolbar.sendHint')} (git push)`,
    );
    expect(operationHint('toolbar.checkForUpdatesHint', 'fetch')).toBe(
      `${t('toolbar.checkForUpdatesHint')} (git fetch)`,
    );
    expect(operationHint('toolbar.setAsideHint', 'stash')).toBe(
      `${t('toolbar.setAsideHint')} (git stash)`,
    );
    expect(operationHint('toolbar.bringBackHint', 'stash pop')).toBe(
      `${t('toolbar.bringBackHint')} (git stash pop)`,
    );
  });

  it('keeps the base hint text unchanged', () => {
    expect(operationHint('table.saveHint', 'commit')).toBe(
      'Save changes in this project to Git (git commit)',
    );
    expect(operationHint('table.getLatestForHint', 'pull')).toBe(
      'Get latest for this project (git pull)',
    );
    expect(operationHint('table.sendForHint', 'push')).toBe(
      'Send changes for this project (git push)',
    );
  });

  it('returns the base hint unchanged in Git mode', () => {
    setWording('git');
    expect(operationHint('toolbar.saveHint', 'commit')).toBe(t('toolbar.saveHint'));
    expect(operationHint('toolbar.saveHint', 'commit')).not.toContain('(git');
  });

  it('never overlays the hint key itself', () => {
    setWording('git');
    expect(t('toolbar.saveHint')).toBe('Create a checkpoint of your changed files in Git');
    expect(t('table.saveHint')).toBe('Save changes in this project to Git');
  });
});

function memoryStorage(initial: Record<string, string> = {}): WordingStorage & {
  values: Map<string, string>;
} {
  const values = new Map(Object.entries(initial));
  return {
    values,
    get: (key) => values.get(key) ?? null,
    set: (key, value) => {
      values.set(key, value);
    },
  };
}

describe('useWording', () => {
  it('defaults to plain when no value is stored', () => {
    const state = useWording({ storage: memoryStorage() });
    expect(state.wording.value).toBe('plain');
  });

  it('writes only the wording key when a value is selected', () => {
    const storage = memoryStorage();
    const state = useWording({ storage });

    state.setWording('git');
    expect(state.wording.value).toBe('git');
    expect(storage.values.get(WORDING_STORAGE_KEY)).toBe('git');
    expect([...storage.values.keys()]).toEqual([WORDING_STORAGE_KEY]);

    state.setWording('plain');
    expect(storage.values.get(WORDING_STORAGE_KEY)).toBe('plain');
  });

  it('reads a stored Git value', () => {
    const state = useWording({ storage: memoryStorage({ [WORDING_STORAGE_KEY]: 'git' }) });
    expect(state.wording.value).toBe('git');
  });

  it('falls back to plain for an invalid stored value', () => {
    const state = useWording({
      storage: memoryStorage({ [WORDING_STORAGE_KEY]: 'shakespeare' }),
    });
    expect(state.wording.value).toBe('plain');
  });

  it('does not throw when storage read or write fails', () => {
    const throwing: WordingStorage = {
      get: () => {
        throw new Error('disabled');
      },
      set: () => {
        throw new Error('disabled');
      },
    };

    expect(() => {
      const state = useWording({ storage: throwing });
      expect(state.wording.value).toBe('plain');
      state.setWording('git');
      expect(state.wording.value).toBe('git');
    }).not.toThrow();
  });

  it('ignores unsupported values without overwriting storage', () => {
    const storage = memoryStorage({ [WORDING_STORAGE_KEY]: 'git' });
    const state = useWording({ storage });

    state.setWording('shakespeare' as unknown as Wording);

    expect(state.wording.value).toBe('git');
    expect(storage.values.get(WORDING_STORAGE_KEY)).toBe('git');
  });

  it('keeps two factory instances with separate storage isolated', () => {
    const first = useWording({ storage: memoryStorage() });
    const second = useWording({ storage: memoryStorage() });

    first.setWording('git');

    expect(first.wording.value).toBe('git');
    expect(second.wording.value).toBe('plain');
  });

  it('does not share state between language and wording keys', () => {
    const storage = memoryStorage();
    const state = useWording({ storage });

    setLanguage('en');
    const languageValue = window.localStorage?.getItem('topdata.language.v1');

    state.setWording('git');
    expect(window.localStorage?.getItem('topdata.language.v1')).toBe(languageValue);

    const storedWording = storage.values.get(WORDING_STORAGE_KEY);
    setLanguage('de');
    expect(storage.values.get(WORDING_STORAGE_KEY)).toBe(storedWording);
    expect(storage.values.has('topdata.language.v1')).toBe(false);
  });

  it('exposes the module singleton ref', () => {
    setWording('git');
    expect(wording.value).toBe('git');
    setWording('plain');
    expect(wording.value).toBe('plain');
  });
});
