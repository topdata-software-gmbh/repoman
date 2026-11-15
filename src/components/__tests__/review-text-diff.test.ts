import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it } from 'vitest';
import ReviewTextDiff from '../reviews/ReviewTextDiff.vue';
import { setLanguage } from '../../lib/i18n';
import type { TextPreview } from '../../types/domain';

function preview(overrides: Partial<TextPreview> = {}): TextPreview {
  return {
    rows: [
      {
        kind: 'fold',
        oldStart: 1,
        newStart: 1,
        count: 2,
        rows: [
          { kind: 'same', oldNumber: 1, newNumber: 1, oldText: 'line one', newText: 'line one' },
          { kind: 'same', oldNumber: 2, newNumber: 2, oldText: 'line two', newText: 'line two' },
        ],
      },
      { kind: 'changed', oldNumber: 3, newNumber: 3, oldText: 'old value', newText: 'new value' },
      { kind: 'added', newNumber: 4, newText: 'brand new' },
      { kind: 'removed', oldNumber: 4, oldText: 'gone away' },
    ],
    truncated: false,
    truncationReason: null,
    raw: '@@ -3,1 +3,1 @@\n-old value\n+new value\n',
    language: 'typescript',
    ...overrides,
  };
}

function lastMode(wrapper: ReturnType<typeof mount>): unknown[] | undefined {
  const events = wrapper.emitted('update:displayMode');
  return events?.[events.length - 1];
}

function foldButton(wrapper: ReturnType<typeof mount>) {
  return wrapper
    .findAll('button')
    .find((button) => button.text().includes('unchanged lines'));
}

describe('ReviewTextDiff', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('renders side-by-side rows with tint markers and collapsed folds', () => {
    const wrapper = mount(ReviewTextDiff, {
      props: { preview: preview(), path: 'src/app.ts' },
    });

    expect(wrapper.text()).toContain('Before');
    expect(wrapper.text()).toContain('After');
    expect(wrapper.text()).toContain('old value');
    expect(wrapper.text()).toContain('new value');
    expect(wrapper.text()).toContain('brand new');
    expect(wrapper.text()).toContain('gone away');
    expect(wrapper.text()).toContain('+');
    expect(wrapper.text()).toContain('−');
    expect(wrapper.text()).not.toContain('line one');

    const fold = foldButton(wrapper);
    expect(fold?.attributes('aria-expanded')).toBe('false');
  });

  it('expands a fold to reveal exactly its unchanged rows', async () => {
    const wrapper = mount(ReviewTextDiff, {
      props: { preview: preview(), path: 'src/app.ts' },
    });
    const fold = foldButton(wrapper);
    await fold?.trigger('click');

    expect(fold?.attributes('aria-expanded')).toBe('true');
    expect(wrapper.text()).toContain('line one');
    expect(wrapper.text()).toContain('line two');

    await fold?.trigger('click');
    expect(wrapper.text()).not.toContain('line one');
  });

  it('renders unified rows in document order', () => {
    const wrapper = mount(ReviewTextDiff, {
      props: { preview: preview(), path: 'src/app.ts', displayMode: 'unified' },
    });

    expect(wrapper.text()).not.toContain('Before');
    expect(wrapper.text()).toContain('old value');
    expect(wrapper.text()).toContain('new value');
    expect(wrapper.text()).toContain('brand new');
  });

  it('renders raw technical text when available and exposes an accessible toggle', async () => {
    const wrapper = mount(ReviewTextDiff, {
      props: { preview: preview(), path: 'src/app.ts', displayMode: 'raw' },
    });

    expect(wrapper.find('pre').text()).toContain('@@ -3,1 +3,1 @@');

    const unifiedButton = wrapper
      .findAll('button')
      .find((button) => button.text() === 'Unified');
    expect(unifiedButton?.attributes('aria-pressed')).toBe('false');
    await unifiedButton?.trigger('click');
    expect(lastMode(wrapper)).toEqual(['unified']);
  });

  it('falls back to rows when raw text was omitted by truncation', () => {
    const wrapper = mount(ReviewTextDiff, {
      props: {
        preview: preview({ raw: null, truncated: true, truncationReason: 'lines' }),
        path: 'src/app.ts',
        displayMode: 'raw',
      },
    });

    expect(wrapper.find('pre').exists()).toBe(false);
    expect(wrapper.text()).toContain('old value');
    expect(wrapper.text()).toContain('Only the first');
  });
});
