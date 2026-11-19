import { beforeEach, describe, expect, it } from 'vitest';
import { language, setLanguage, t, tPlural } from '../i18n';
import { gitWording } from '../i18n/gitWording';
import { en } from '../i18n/messages.en';
import { de } from '../i18n/messages.de';
import { setWording } from '../i18n/wording';

beforeEach(() => {
  setLanguage('de');
  setWording('plain');
});

describe('catalogue', () => {
  it('has the same keys in English and German', () => {
    expect(Object.keys(de).sort()).toEqual(Object.keys(en).sort());
  });
});

describe('t', () => {
  it('translates a static key in the active language', () => {
    expect(t('toolbar.saveToGit')).toBe('In Git speichern');
    setLanguage('en');
    expect(t('toolbar.saveToGit')).toBe('Save to Git');
  });

  it('interpolates parameters and leaves unknown placeholders intact', () => {
    setLanguage('de');
    expect(t('toolbar.upToDateCount', { count: 3 })).toBe('3 aktuell');
    expect(t('common.of', { current: 2 })).toBe('2 von {total}');
  });

  it('uses the plural base key by count', () => {
    setLanguage('de');
    expect(tPlural('common.project', 1)).toBe('Projekt');
    expect(tPlural('common.project', 2)).toBe('Projekte');
    expect(tPlural('common.project', 0)).toBe('Projekte');
  });

  it('reacts to language changes', () => {
    const before = t('sidebar.groups');
    setLanguage('en');
    const after = t('sidebar.groups');

    expect(before).not.toBe(after);
    expect(language.value).toBe('en');
  });
});

describe('Git wording overlay', () => {
  it('has identical key sets in English and German', () => {
    expect(Object.keys(gitWording.en).sort()).toEqual(Object.keys(gitWording.de).sort());
  });

  it('only overlays keys that exist in both plain catalogues', () => {
    for (const key of Object.keys(gitWording.en)) {
      expect(en).toHaveProperty(key);
      expect(de).toHaveProperty(key);
    }
  });

  it('uses non-empty string values in both languages', () => {
    for (const key of Object.keys(gitWording.en) as Array<keyof typeof gitWording.en>) {
      expect(typeof gitWording.en[key]).toBe('string');
      expect(gitWording.en[key]?.trim()).not.toBe('');
      expect(gitWording.de[key]).toBe(gitWording.en[key]);
    }
  });

  it('returns the overlay in Git mode and the plain catalogue in plain mode', () => {
    setLanguage('en');
    expect(t('toolbar.saveToGit')).toBe('Save to Git');
    setWording('git');
    expect(t('toolbar.saveToGit')).toBe('Commit');
    expect(t('toolbar.getLatest')).toBe('Pull');
    expect(t('toolbar.sendChanges')).toBe('Push');

    setWording('plain');
    expect(t('toolbar.saveToGit')).toBe('Save to Git');
  });

  it('interpolates parameters after selecting the overlay template', () => {
    setLanguage('en');
    setWording('git');
    expect(t('table.getLatestFor', { name: 'alpha' })).toBe('Pull for alpha');
    expect(t('status.updatesAvailable', { count: 3 })).toBe('3 behind');
  });

  it('falls back to the plain label for a missing overlay entry', () => {
    setLanguage('en');
    setWording('git');
    expect(t('table.review')).toBe(en['table.review']);
    expect(t('status.folderNotFound')).toBe(en['status.folderNotFound']);
    expect(t('toolbar.saveHint')).toBe(en['toolbar.saveHint']);
  });

  it('falls back to the plain label for a blank overlay value', () => {
    setLanguage('en');
    setWording('git');
    const overlays = gitWording.en as Record<string, string | undefined>;
    const previous = overlays['table.save'];
    overlays['table.save'] = '   ';
    try {
      expect(t('table.save')).toBe(en['table.save']);
    } finally {
      overlays['table.save'] = previous;
    }
  });

  it('resolves German plain and German Git wording independently', () => {
    setLanguage('de');

    expect(t('toolbar.saveToGit')).toBe('In Git speichern');
    setWording('git');
    expect(t('toolbar.saveToGit')).toBe('Commit');
    expect(t('toolbar.saveHint')).toBe(de['toolbar.saveHint']);
    expect(t('toolbar.saveHint')).toContain('Git');

    setWording('plain');
    expect(t('toolbar.saveToGit')).toBe('In Git speichern');
    setLanguage('en');
    expect(t('toolbar.saveToGit')).toBe('Save to Git');
  });
});
