<script setup lang="ts">
import { computed } from 'vue';
import { t } from '../../lib/i18n';
import { formatBytes, imageStateLabelKey } from '../../lib/review';
import ReviewNotice from './ReviewNotice.vue';
import type { FilePreview, ImageSide } from '../../types/domain';

const props = defineProps<{ preview: FilePreview }>();

const image = computed(() => props.preview.image ?? null);
const state = computed(() => image.value?.state ?? 'missing');
const showNotice = computed(() => state.value !== 'ready');
const sides = computed<Array<{ key: 'old' | 'new'; label: string; side: ImageSide }>>(() => {
  const value = image.value;
  if (!value) return [];
  const result: Array<{ key: 'old' | 'new'; label: string; side: ImageSide }> = [];
  if (value.old) {
    result.push({ key: 'old', label: t('review.image.before'), side: value.old });
  }
  if (value.new) {
    result.push({ key: 'new', label: t('review.image.after'), side: value.new });
  }
  return result;
});

const noticeKey = computed(() => imageStateLabelKey(state.value) ?? 'review.notice.unsupported');
const columns = computed(() => (sides.value.length === 2 ? 'grid-cols-2' : 'grid-cols-1'));
</script>

<template>
  <ReviewNotice
    v-if="showNotice"
    :title="t(noticeKey)"
    :message="t('review.files.heading')"
    tone="warning"
  />

  <div v-else class="grid gap-3" :class="columns">
    <figure
      v-for="entry in sides"
      :key="entry.key"
      class="flex min-w-0 flex-col gap-2 rounded-lg border border-border bg-surface p-3"
    >
      <figcaption class="flex flex-wrap items-center justify-between gap-2 text-xs text-fg-muted">
        <span class="font-medium text-fg">{{ entry.label }}</span>
        <span>
          {{ t('review.image.dimensions', { width: entry.side.width, height: entry.side.height }) }}
          ·
          {{ t('review.image.size', { size: formatBytes(entry.side.bytes) }) }}
        </span>
      </figcaption>
      <div class="flex min-h-[8rem] items-center justify-center rounded border border-border bg-canvas p-2">
        <img
          :src="entry.side.dataUrl"
          :alt="entry.label"
          class="max-h-[52vh] max-w-full object-contain"
        />
      </div>
    </figure>
  </div>
</template>
