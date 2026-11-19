<script setup lang="ts">
import { computed } from 'vue';
import { TdButton, TdMenu, TdMenuItem } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../lib/i18n';
import { setWording, wording, type Wording } from '../lib/i18n/wording';

const options = computed<
  Array<{ value: Wording; label: string; short: string }>
>(() => [
  { value: 'plain', label: t('wording.plain'), short: t('wording.plainShort') },
  { value: 'git', label: t('wording.git'), short: t('wording.gitShort') },
]);

const active = computed(
  () => options.value.find((option) => option.value === wording.value) ?? options.value[0],
);

const triggerLabel = computed(
  () => `${t('sidebar.changeWording')}: ${active.value.label}`,
);

function select(value: Wording): void {
  if (value !== wording.value) setWording(value);
}
</script>

<template>
  <TdMenu align="end" :aria-label="t('sidebar.changeWording')">
    <template #trigger="{ open }">
      <TdButton
        variant="ghost"
        size="sm"
        class="td-lang-switcher"
        :aria-label="triggerLabel"
        :title="triggerLabel"
        aria-haspopup="menu"
        :aria-expanded="open"
      >
        <template #icon-left>
          <span class="td-lang-switcher__code">{{ active.short }}</span>
        </template>
      </TdButton>
    </template>
    <TdMenuItem
      v-for="option in options"
      :key="option.value"
      @select="select(option.value)"
    >
      <span class="wording-option-label">{{ option.label }}</span>
      <span v-if="option.value === wording" aria-hidden="true"> ✓</span>
      <span v-if="option.value === wording" class="sr-only">
        ({{ t('wording.current') }})
      </span>
    </TdMenuItem>
  </TdMenu>
</template>
