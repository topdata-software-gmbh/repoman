import { language, t, tPlural, type MessageKey } from './i18n';
import type {
  DiffFileStatus,
  DiffFold,
  DiffLine,
  DisplayMode,
  ImagePreviewState,
  PreviewNotice,
  TextPreview,
} from '../types/domain';

export type UnifiedKind = 'same' | 'added' | 'removed' | 'fold';

export interface UnifiedRow {
  kind: UnifiedKind;
  oldNumber: number | null;
  newNumber: number | null;
  text: string;
  fold?: DiffFold;
}

export function statusLabelKey(status: DiffFileStatus): MessageKey {
  switch (status) {
    case 'new':
      return 'review.status.new';
    case 'deleted':
      return 'review.status.deleted';
    case 'renamed':
      return 'review.status.renamed';
    case 'conflict':
      return 'review.status.conflict';
    default:
      return 'review.status.modified';
  }
}

export function noticeLabelKey(notice: PreviewNotice | null): MessageKey | null {
  switch (notice) {
    case 'too-large':
      return 'review.notice.tooLarge';
    case 'unsupported':
      return 'review.notice.unsupported';
    case 'corrupt':
      return 'review.notice.corrupt';
    case 'missing':
      return 'review.notice.missing';
    default:
      return null;
  }
}

export function imageStateLabelKey(state: ImagePreviewState): MessageKey | null {
  switch (state) {
    case 'too-large':
      return 'review.image.tooLarge';
    case 'corrupt':
      return 'review.image.corrupt';
    case 'unsupported':
      return 'review.image.unsupported';
    case 'missing':
      return 'review.image.missing';
    default:
      return null;
  }
}

export function errorLabelKey(code: string | null | undefined): MessageKey {
  switch (code) {
    case 'unknown-file':
      return 'review.error.unknownFile';
    case 'save-not-found':
      return 'review.error.saveNotFound';
    case 'head-not-found':
      return 'review.error.headNotFound';
    case 'not-a-repository':
      return 'review.error.notRepository';
    default:
      return 'review.error.generic';
  }
}

export function displayModeLabelKey(mode: DisplayMode): MessageKey {
  switch (mode) {
    case 'unified':
      return 'review.display.unified';
    case 'raw':
      return 'review.display.raw';
    default:
      return 'review.display.sideBySide';
  }
}

export function foldLabel(count: number): string {
  return t('review.foldLine.one', { count });
}

export function foldLinesLabel(count: number): string {
  return tPlural('review.foldLine', count);
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return '0 B';
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const rounded = value >= 10 ? Math.round(value).toString() : value.toFixed(1);
  return `${rounded} ${units[unit]}`;
}

export function formatSavedAt(timestamp: number): string {
  const locale = language.value === 'de' ? 'de-DE' : 'en-GB';
  return new Date(timestamp * 1000).toLocaleString(locale, {
    dateStyle: 'medium',
    timeStyle: 'short',
  });
}

export function truncationLabelKey(reason: 'bytes' | 'lines' | null): MessageKey | null {
  if (reason === 'bytes') return 'review.truncated.bytes';
  if (reason === 'lines') return 'review.truncated.lines';
  return null;
}

/** Derives one column from the same core-produced rows, in document order. */
export function toUnifiedRows(rows: DiffLine[]): UnifiedRow[] {
  const unified: UnifiedRow[] = [];
  for (const row of rows) {
    switch (row.kind) {
      case 'same':
        unified.push({
          kind: 'same',
          oldNumber: row.oldNumber ?? null,
          newNumber: row.newNumber ?? null,
          text: row.oldText ?? '',
        });
        break;
      case 'changed':
        unified.push({
          kind: 'removed',
          oldNumber: row.oldNumber ?? null,
          newNumber: null,
          text: row.oldText ?? '',
        });
        unified.push({
          kind: 'added',
          oldNumber: null,
          newNumber: row.newNumber ?? null,
          text: row.newText ?? '',
        });
        break;
      case 'removed':
        unified.push({
          kind: 'removed',
          oldNumber: row.oldNumber ?? null,
          newNumber: null,
          text: row.oldText ?? '',
        });
        break;
      case 'added':
        unified.push({
          kind: 'added',
          oldNumber: null,
          newNumber: row.newNumber ?? null,
          text: row.newText ?? '',
        });
        break;
      case 'fold':
        unified.push({
          kind: 'fold',
          oldNumber: row.oldStart,
          newNumber: row.newStart,
          text: '',
          fold: row,
        });
        break;
    }
  }
  return unified;
}

export function rawPresentation(preview: TextPreview): string | null {
  return preview.raw && !preview.truncated ? preview.raw : null;
}

const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'javascript',
  ts: 'typescript',
  tsx: 'typescript',
  vue: 'xml',
  html: 'xml',
  htm: 'xml',
  xml: 'xml',
  svg: 'xml',
  css: 'css',
  scss: 'scss',
  json: 'json',
  php: 'php',
  rs: 'rust',
  sh: 'bash',
  bash: 'bash',
  zsh: 'bash',
  yml: 'yaml',
  yaml: 'yaml',
  md: 'markdown',
  markdown: 'markdown',
};

export function languageForPath(path: string): string {
  const extension = path.split('.').pop()?.toLowerCase() ?? '';
  return LANGUAGE_BY_EXTENSION[extension] ?? 'plaintext';
}

export function statusTone(status: DiffFileStatus): 'success' | 'error' | 'info' | 'warning' | 'neutral' {
  switch (status) {
    case 'new':
      return 'success';
    case 'deleted':
    case 'conflict':
      return 'error';
    case 'renamed':
      return 'info';
    case 'modified':
      return 'warning';
    default:
      return 'neutral';
  }
}
