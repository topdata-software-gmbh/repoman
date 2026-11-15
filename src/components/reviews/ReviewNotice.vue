<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    message?: string;
    tone?: 'info' | 'warning' | 'error';
  }>(),
  { tone: 'info' },
);
</script>

<template>
  <div
    class="rounded-lg border border-border bg-surface px-4 py-6 text-center"
    :class="{
      'text-fg-muted': tone === 'info',
      'text-warning-fg': tone === 'warning',
      'text-error-fg': tone === 'error',
    }"
    role="status"
  >
    <p class="text-sm font-medium text-fg">{{ title }}</p>
    <p v-if="message" class="mt-1 text-xs text-fg-muted">{{ message }}</p>
    <slot />
  </div>
</template>
