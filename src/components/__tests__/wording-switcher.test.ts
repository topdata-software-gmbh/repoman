import { mount } from '@vue/test-utils';
import { defineComponent, h, nextTick } from 'vue';
import { beforeEach, describe, expect, it } from 'vitest';
import WordingSwitcher from '../WordingSwitcher.vue';
import { setLanguage, t } from '../../lib/i18n';
import { setWording, wording } from '../../lib/i18n/wording';

const Harness = defineComponent({
  setup() {
    return () =>
      h('div', [
        h(WordingSwitcher),
        h('span', { class: 'live-label' }, t('toolbar.saveToGit')),
      ]);
  },
});

function triggerButton(): HTMLButtonElement {
  const button = document.querySelector<HTMLButtonElement>('button[aria-haspopup="menu"]');
  if (!button) throw new Error('wording trigger not found');
  return button;
}

function menuItems(): HTMLButtonElement[] {
  return Array.from(document.querySelectorAll<HTMLButtonElement>('[role="menuitem"]'));
}

async function openMenu(): Promise<void> {
  triggerButton().click();
  await nextTick();
  await nextTick();
}

beforeEach(() => {
  setLanguage('en');
  setWording('plain');
  document.body.innerHTML = '';
});

describe('WordingSwitcher', () => {
  it('names the trigger with the localized label and the active option', async () => {
    const wrapper = mount(Harness, { attachTo: document.body });

    expect(triggerButton().getAttribute('aria-label')).toBe('Change wording: Plain language');
    expect(triggerButton().getAttribute('aria-expanded')).toBe('false');

    setWording('git');
    await nextTick();

    expect(triggerButton().getAttribute('aria-label')).toBe('Change wording: Git terminology');
    wrapper.unmount();
  });

  it('offers exactly the two wording options', async () => {
    const wrapper = mount(Harness, { attachTo: document.body });
    await openMenu();

    const labels = Array.from(document.querySelectorAll('.wording-option-label')).map(
      (item) => item.textContent?.trim(),
    );
    expect(labels).toEqual(['Plain language', 'Git terminology']);
    expect(menuItems()).toHaveLength(2);
    wrapper.unmount();
  });

  it('selects Git wording and closes the menu', async () => {
    const wrapper = mount(Harness, { attachTo: document.body });
    await openMenu();

    const gitOption = menuItems().find((item) => item.textContent?.includes('Git terminology'));
    gitOption?.click();
    await nextTick();
    await nextTick();

    expect(wording.value).toBe('git');
    expect(menuItems()).toHaveLength(0);
    expect(triggerButton().getAttribute('aria-expanded')).toBe('false');
    wrapper.unmount();
  });

  it('updates a label rendered from t() without remounting', async () => {
    const wrapper = mount(Harness, { attachTo: document.body });
    expect(wrapper.get('.live-label').text()).toBe('Save to Git');

    await openMenu();
    menuItems()
      .find((item) => item.textContent?.includes('Git terminology'))
      ?.click();
    await nextTick();
    await nextTick();

    expect(wrapper.get('.live-label').text()).toBe('Commit');
    wrapper.unmount();
  });
});
