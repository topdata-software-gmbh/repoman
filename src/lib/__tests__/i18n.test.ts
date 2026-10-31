import { beforeEach, describe, expect, it } from 'vitest';
import { language, setLanguage, t, tPlural } from '../i18n';
import { en } from '../i18n/messages.en';
import { de } from '../i18n/messages.de';

beforeEach(() => {
  setLanguage('de');
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
