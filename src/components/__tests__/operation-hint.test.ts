import { flushPromises, mount } from '@vue/test-utils';
import { h } from 'vue';
import { beforeEach, describe, expect, it } from 'vitest';
import OperationHint from '../OperationHint.vue';
import { setLanguage } from '../../lib/i18n';
import { setWording } from '../../lib/i18n/wording';

function mountHint(hint = 'Save changes in this project to Git (git commit)') {
  return mount(OperationHint, {
    props: { hint },
    attachTo: document.body,
    slots: {
      default: ({ describedBy, title }: { describedBy: string; title: string }) =>
        h('button', { type: 'button', 'aria-describedby': describedBy, title }, 'Save'),
    },
  });
}

beforeEach(() => {
  setLanguage('en');
  setWording('plain');
  document.body.innerHTML = '';
});

describe('OperationHint', () => {
  it('points aria-describedby at an always-present hidden description', () => {
    const wrapper = mountHint();
    const button = wrapper.get('button');
    const id = button.attributes('aria-describedby');

    expect(id).toBeTruthy();
    const description = document.getElementById(id as string);
    expect(description).not.toBeNull();
    expect(description?.textContent).toContain('(git commit)');
    expect(button.attributes('title')).toContain('(git commit)');
    wrapper.unmount();
  });

  it('shows the tooltip on focusin and hides it on Escape', async () => {
    const wrapper = mountHint();
    const button = wrapper.get('button');

    expect(document.body.querySelector('[role="tooltip"]')).toBeNull();

    await button.trigger('focusin');
    await flushPromises();
    expect(document.body.querySelector('[role="tooltip"]')?.textContent).toContain('(git commit)');

    await button.trigger('keydown', { key: 'Escape' });
    await flushPromises();
    expect(document.body.querySelector('[role="tooltip"]')).toBeNull();
    wrapper.unmount();
  });

  it('shows the tooltip on pointer enter and hides it on leave', async () => {
    const wrapper = mountHint();
    const root = wrapper.get('[data-operation-hint]');

    await root.trigger('mouseenter');
    await flushPromises();
    expect(document.body.querySelector('[role="tooltip"]')).not.toBeNull();

    await root.trigger('mouseleave');
    await flushPromises();
    expect(document.body.querySelector('[role="tooltip"]')).toBeNull();
    wrapper.unmount();
  });

  it('uses unique description ids for two instances', () => {
    const first = mountHint();
    const second = mountHint();
    const ids = [...document.querySelectorAll('button')].map((button) =>
      button.getAttribute('aria-describedby'),
    );

    expect(ids[0]).toBeTruthy();
    expect(ids[1]).toBeTruthy();
    expect(ids[0]).not.toBe(ids[1]);
    first.unmount();
    second.unmount();
  });
});
