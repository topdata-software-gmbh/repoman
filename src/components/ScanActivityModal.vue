<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue';
import { TdBadge, TdModal, TdSpinner } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';
import type { ScanActivityEntry } from '../types/domain';

/** FR-013: the log never renders more than 500 rows; older hits fold into one row. */
const RENDER_CAP = 500;

const store = useRepoStore();
const listRef = ref<HTMLElement | null>(null);
const stickToBottom = ref(true);

const activity = computed(() => store.scanActivity);

interface ScanRow {
  path: string;
  entry: ScanActivityEntry;
}

const rows = computed<ScanRow[]>(() =>
  Array.from(activity.value.entries.entries()).map(([path, entry]) => ({ path, entry })),
);

const hiddenCount = computed(() => Math.max(0, rows.value.length - RENDER_CAP));
const visibleRows = computed(() =>
  hiddenCount.value > 0 ? rows.value.slice(-RENDER_CAP) : rows.value,
);

/** FR-014: keep both ends of long paths, elide the middle. */
function middleEllipsis(value: string, head = 24, tail = 32): string {
  if (value.length <= head + tail + 1) return value;
  return `${value.slice(0, head)}…${value.slice(value.length - tail)}`;
}

function rootName(root: string): string {
  const parts = root.split(/[\\/]/).filter(Boolean);
  return parts[parts.length - 1] ?? root;
}

function formatElapsed(ms: number): string {
  const seconds = Math.round(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const rest = seconds % 60;
  return `${minutes}m ${rest}s`;
}

function kindLabel(kind: ScanActivityEntry['kind']): string {
  switch (kind) {
    case 'theme':
      return t('filters.theme');
    case 'plugin':
      return t('filters.plugin');
    case 'other':
      return t('filters.other');
    default:
      return t('scan.activity.kindPending');
  }
}

function onScroll(): void {
  const element = listRef.value;
  if (!element) return;
  stickToBottom.value = element.scrollHeight - element.scrollTop - element.clientHeight < 24;
}

watch(
  () => visibleRows.value.length,
  async () => {
    if (!stickToBottom.value) return;
    await nextTick();
    const element = listRef.value;
    if (element) element.scrollTop = element.scrollHeight;
  },
);

watch(
  () => activity.value.runId,
  () => {
    stickToBottom.value = true;
  },
);

function onOpenChange(open: boolean): void {
  if (!open) store.closeScanModal();
}
</script>

<template>
  <TdModal
    :open="activity.modalOpen"
    :title="t('scan.activity.title')"
    @update:open="onOpenChange"
  >
    <div class="space-y-3">
      <header class="rounded-lg border border-border bg-canvas px-3 py-2" role="status">
        <template v-if="activity.finished">
          <p class="text-sm font-medium text-fg">
            <template v-if="activity.finished.error">
              {{ t('scan.activity.failed') }}
            </template>
            <template v-else>
              {{
                t('scan.activity.summary', {
                  count: store.scanActivityCount,
                  elapsed: formatElapsed(activity.finished.elapsedMs),
                })
              }}
            </template>
          </p>
          <p
            v-if="activity.finished.error"
            class="mt-1 text-[11px] leading-relaxed text-error-fg"
          >
            {{ activity.finished.error }}
          </p>
        </template>
        <template v-else>
          <div class="flex items-center gap-2">
            <TdSpinner v-if="activity.running" size="sm" />
            <span
              v-if="activity.roots.length > 1 && activity.currentRoot"
              class="shrink-0 text-[11px] font-medium text-fg-muted"
              data-testid="header-root"
              :title="activity.currentRoot"
            >
              {{ rootName(activity.currentRoot) }}
            </span>
            <span class="min-w-0 flex-1 truncate text-[12px] text-fg-muted" :title="activity.currentDir ?? ''">
              {{ activity.currentDir ? middleEllipsis(activity.currentDir) : t('scan.activity.empty') }}
            </span>
          </div>
          <p class="mt-1 flex items-center gap-2 text-[12px] text-fg">
            <TdBadge variant="neutral">
              {{ t('scan.activity.found', { count: store.scanActivityCount }) }}
            </TdBadge>
            <span class="text-fg-muted">
              {{ t('scan.activity.elapsed', { time: formatElapsed(activity.elapsedMs) }) }}
            </span>
          </p>
        </template>
      </header>

      <div
        ref="listRef"
        class="scan-activity__log max-h-80 overflow-y-auto rounded-lg border border-border"
        @scroll="onScroll"
      >
        <ol class="divide-y divide-border">
          <li
            v-if="hiddenCount > 0"
            class="px-3 py-1.5 text-[11px] italic text-fg-dim"
            data-testid="overflow-row"
          >
            {{ t('scan.activity.andMore', { count: hiddenCount }) }}
          </li>
          <li
            v-for="row in visibleRows"
            :key="row.path"
            class="flex items-center gap-2 px-3 py-1.5"
            data-testid="activity-row"
          >
            <span class="min-w-0 flex-1 truncate text-[12px] text-fg" :title="row.path">
              {{ row.entry.name }}
            </span>
            <span
              class="shrink-0 text-[11px] text-fg-dim"
              :title="row.entry.root"
              data-testid="activity-root"
            >
              {{ rootName(row.entry.root) }}
            </span>
            <TdBadge
              variant="neutral"
              class="shrink-0"
              :class="row.entry.inspected ? '' : 'opacity-40'"
              data-testid="activity-badge"
            >
              {{ kindLabel(row.entry.kind) }}
            </TdBadge>
          </li>
        </ol>
      </div>
    </div>
  </TdModal>
</template>

<style scoped>
.scan-activity__log ol {
  list-style: none;
  margin: 0;
  padding: 0;
}
</style>
