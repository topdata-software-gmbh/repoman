import { describe, expect, it } from 'vitest';
import {
  errorLabelKey,
  formatBytes,
  languageForPath,
  noticeLabelKey,
  rawPresentation,
  statusLabelKey,
  toUnifiedRows,
  truncationLabelKey,
} from '../review';
import type { DiffLine, TextPreview } from '../../types/domain';

function preview(overrides: Partial<TextPreview> = {}): TextPreview {
  return {
    rows: [],
    truncated: false,
    truncationReason: null,
    raw: null,
    language: null,
    ...overrides,
  };
}

describe('review display helpers', () => {
  it('maps statuses to plain-language keys', () => {
    expect(statusLabelKey('new')).toBe('review.status.new');
    expect(statusLabelKey('modified')).toBe('review.status.modified');
    expect(statusLabelKey('deleted')).toBe('review.status.deleted');
    expect(statusLabelKey('renamed')).toBe('review.status.renamed');
    expect(statusLabelKey('conflict')).toBe('review.status.conflict');
  });

  it('maps notices, image states, errors and truncation reasons', () => {
    expect(noticeLabelKey('too-large')).toBe('review.notice.tooLarge');
    expect(noticeLabelKey(null)).toBeNull();
    expect(errorLabelKey('unknown-file')).toBe('review.error.unknownFile');
    expect(errorLabelKey('save-not-found')).toBe('review.error.saveNotFound');
    expect(errorLabelKey('head-not-found')).toBe('review.error.headNotFound');
    expect(errorLabelKey('something-else')).toBe('review.error.generic');
    expect(truncationLabelKey('bytes')).toBe('review.truncated.bytes');
    expect(truncationLabelKey('lines')).toBe('review.truncated.lines');
    expect(truncationLabelKey(null)).toBeNull();
  });

  it('formats byte sizes', () => {
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(10 * 1024 * 1024)).toBe('10 MB');
  });
});

describe('unified rows', () => {
  it('derives one column from the same rows in document order', () => {
    const rows: DiffLine[] = [
      { kind: 'fold', oldStart: 1, newStart: 1, count: 1, rows: [] },
      { kind: 'changed', oldNumber: 2, newNumber: 2, oldText: 'a', newText: 'A' },
      { kind: 'added', newNumber: 3, newText: 'new' },
      { kind: 'removed', oldNumber: 4, oldText: 'gone' },
    ];

    const unified = toUnifiedRows(rows);
    expect(unified.map((row) => row.kind)).toEqual([
      'fold',
      'removed',
      'added',
      'added',
      'removed',
    ]);
    expect(unified[1]).toMatchObject({ oldNumber: 2, text: 'a' });
    expect(unified[2]).toMatchObject({ newNumber: 2, text: 'A' });
  });
});

describe('raw presentation selection', () => {
  it('uses raw text when available and not truncated', () => {
    expect(rawPresentation(preview({ raw: '@@ -1 +1 @@' }))).toBe('@@ -1 +1 @@');
  });

  it('falls back when truncated', () => {
    expect(
      rawPresentation(preview({ raw: '@@ -1 +1 @@', truncated: true })),
    ).toBeNull();
  });
});

describe('language selection', () => {
  it('maps extensions to highlight.js ids with a plaintext fallback', () => {
    expect(languageForPath('src/app.ts')).toBe('typescript');
    expect(languageForPath('src/App.vue')).toBe('xml');
    expect(languageForPath('composer.json')).toBe('json');
    expect(languageForPath('script.unknown')).toBe('plaintext');
  });
});
