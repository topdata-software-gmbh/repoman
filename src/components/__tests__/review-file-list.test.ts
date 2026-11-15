import { mount } from '@vue/test-utils';
import { beforeEach, describe, expect, it } from 'vitest';
import ReviewFileList from '../reviews/ReviewFileList.vue';
import { setLanguage } from '../../lib/i18n';
import type { DiffFileEntry } from '../../types/domain';

function lastSelect(wrapper: ReturnType<typeof mount>): unknown[] | undefined {
  const events = wrapper.emitted('select');
  return events?.[events.length - 1];
}

function entry(path: string, status: DiffFileEntry['status']): DiffFileEntry {
  return {
    path,
    oldPath: null,
    status,
    kind: 'text',
    whitespaceOnly: false,
    addedLines: 2,
    removedLines: 1,
  };
}

const files = [
  entry('src/new.ts', 'new'),
  entry('src/old.ts', 'deleted'),
  entry('src/changed.ts', 'modified'),
];

describe('ReviewFileList', () => {
  beforeEach(() => {
    setLanguage('en');
  });

  it('renders plain-language labels and listbox roles', () => {
    const wrapper = mount(ReviewFileList, {
      props: { files, selectedPath: 'src/new.ts' },
    });

    const listbox = wrapper.find('[role="listbox"]');
    expect(listbox.exists()).toBe(true);
    expect(listbox.attributes('aria-label')).toBe('Changed files');

    const options = wrapper.findAll('[role="option"]');
    expect(options).toHaveLength(3);
    const text = wrapper.text();
    expect(text).toContain('New');
    expect(text).toContain('Deleted');
    expect(text).toContain('Changed');
    expect(text).not.toMatch(/\bM\b/);
    expect(text).not.toMatch(/\bA\b/);
    expect(text).not.toMatch(/\bD\b/);
  });

  it('uses roving tabindex and marks the selected option', () => {
    const wrapper = mount(ReviewFileList, {
      props: { files, selectedPath: 'src/old.ts' },
    });
    const options = wrapper.findAll('[role="option"]');

    expect(options[0].attributes('tabindex')).toBe('-1');
    expect(options[1].attributes('tabindex')).toBe('0');
    expect(options[2].attributes('tabindex')).toBe('-1');
    expect(options[1].attributes('aria-selected')).toBe('true');
    expect(options[0].attributes('aria-selected')).toBe('false');
    expect(options[1].classes()).toContain('focus-visible:bg-surface-hover');
  });

  it('handles Arrow, Home, End, Enter and Space', async () => {
    const wrapper = mount(ReviewFileList, {
      props: { files, selectedPath: 'src/new.ts' },
    });
    const listbox = wrapper.find('[role="listbox"]');

    await listbox.trigger('keydown', { key: 'ArrowDown' });
    expect(lastSelect(wrapper)).toEqual(['src/old.ts']);

    await listbox.trigger('keydown', { key: 'End' });
    expect(lastSelect(wrapper)).toEqual(['src/changed.ts']);

    await listbox.trigger('keydown', { key: 'Home' });
    expect(lastSelect(wrapper)).toEqual(['src/new.ts']);

    await listbox.trigger('keydown', { key: 'ArrowUp' });
    expect(lastSelect(wrapper)).toEqual(['src/new.ts']);

    await listbox.trigger('keydown', { key: 'Enter' });
    expect(lastSelect(wrapper)).toEqual(['src/new.ts']);

    await listbox.trigger('keydown', { key: ' ' });
    expect(lastSelect(wrapper)).toEqual(['src/new.ts']);
  });

  it('selects a row on click', async () => {
    const wrapper = mount(ReviewFileList, {
      props: { files, selectedPath: null },
    });
    await wrapper.findAll('[role="option"]')[2].trigger('click');
    expect(lastSelect(wrapper)).toEqual(['src/changed.ts']);
  });
});
