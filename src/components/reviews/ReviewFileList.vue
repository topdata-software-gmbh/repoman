<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { TdBadge } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t, tPlural } from '../../lib/i18n';
import { statusLabelKey, statusTone } from '../../lib/review';
import type { DiffFileEntry } from '../../types/domain';

const props = defineProps<{
  files: DiffFileEntry[];
  selectedPath: string | null;
}>();

const emit = defineEmits<{ select: [path: string] }>();

const activeIndex = ref(0);
const optionRefs = ref<Array<HTMLElement | null>>([]);

const total = computed(() => props.files.length);

watch(
  () => props.selectedPath,
  (path) => {
    const index = props.files.findIndex((file) => file.path === path);
    if (index >= 0) activeIndex.value = index;
  },
  { immediate: true },
);

watch(
  () => props.files,
  () => {
    if (activeIndex.value >= props.files.length) activeIndex.value = 0;
  },
  { deep: true },
);

function setOptionRef(element: unknown, index: number): void {
  optionRefs.value[index] =
    element instanceof HTMLElement ? element : null;
}

function focusOption(index: number): void {
  activeIndex.value = Math.min(Math.max(index, 0), Math.max(props.files.length - 1, 0));
  optionRefs.value[activeIndex.value]?.focus();
}

function choose(index: number): void {
  const file = props.files[index];
  if (!file) return;
  activeIndex.value = index;
  emit('select', file.path);
}

function onKeydown(event: KeyboardEvent): void {
  if (props.files.length === 0) return;
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault();
      choose(Math.min(activeIndex.value + 1, props.files.length - 1));
      focusOption(activeIndex.value);
      break;
    case 'ArrowUp':
      event.preventDefault();
      choose(Math.max(activeIndex.value - 1, 0));
      focusOption(activeIndex.value);
      break;
    case 'Home':
      event.preventDefault();
      choose(0);
      focusOption(0);
      break;
    case 'End':
      event.preventDefault();
      choose(props.files.length - 1);
      focusOption(props.files.length - 1);
      break;
    case 'Enter':
    case ' ':
      event.preventDefault();
      choose(activeIndex.value);
      break;
    default:
      break;
  }
}

function lineCountLabel(count: number): string {
  return String(count);
}

function addedLabel(count: number): string {
  return t('review.summary.added', { count: lineCountLabel(count) });
}

function removedLabel(count: number): string {
  return t('review.summary.removed', { count: lineCountLabel(count) });
}

const fileWord = computed(() => tPlural('review.file', total.value));
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <p class="mb-2 flex items-baseline justify-between text-xs font-medium text-fg-muted">
      <span>{{ t('review.files.heading') }}</span>
      <span>{{ total }} {{ fileWord }}</span>
    </p>
    <ul
      role="listbox"
      :aria-label="t('review.files.label')"
      class="min-h-0 flex-1 divide-y divide-border overflow-auto rounded-lg border border-border bg-surface"
      @keydown="onKeydown"
    >
      <li
        v-for="(file, index) in props.files"
        :key="file.path"
        :ref="(element) => setOptionRef(element, index)"
        role="option"
        :aria-selected="props.selectedPath === file.path"
        :tabindex="index === activeIndex ? 0 : -1"
        class="cursor-pointer px-3 py-2 outline-none transition-colors hover:bg-surface-hover focus-visible:bg-surface-hover"
        :class="{ 'bg-surface-hover': props.selectedPath === file.path }"
        @click="choose(index)"
        @focus="activeIndex = index"
      >
        <span class="flex items-center justify-between gap-2">
          <span class="min-w-0 flex-1 truncate font-mono text-xs text-fg" :title="file.path">
            {{ file.path }}
          </span>
          <TdBadge :variant="statusTone(file.status)" size="sm">
            {{ t(statusLabelKey(file.status)) }}
          </TdBadge>
        </span>
        <span class="mt-1 flex flex-wrap items-center gap-2 text-[10px] text-fg-muted">
          <span v-if="file.oldPath" class="truncate font-mono" :title="file.oldPath">
            ← {{ file.oldPath }}
          </span>
          <span v-if="file.kind === 'image'">{{ t('review.kind.image') }}</span>
          <span v-else-if="file.kind === 'binary'">{{ t('review.kind.binary') }}</span>
          <span v-if="file.kind === 'text'" class="text-success-fg">+{{ file.addedLines }}</span>
          <span v-if="file.kind === 'text'" class="text-error-fg">−{{ file.removedLines }}</span>
          <span v-if="file.whitespaceOnly">{{ t('review.whitespace') }}</span>
        </span>
        <span class="sr-only">
          {{ addedLabel(file.addedLines) }}, {{ removedLabel(file.removedLines) }}
        </span>
      </li>
    </ul>
  </div>
</template>
