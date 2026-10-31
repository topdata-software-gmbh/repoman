<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { X } from 'lucide-vue-next';
import { TdBadge, TdButton } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { groupGitRuns, shopName } from '../lib/gitLog';
import { t, tPlural } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';
import type { GitRun } from '../types/domain';

// Git action titles/comments come from src-tauri/macros/git-actions.yaml and
// stay English in this increment; see the plan's scope boundary.
const store = useRepoStore();
const expanded = ref<Set<string>>(new Set());

const runs = computed(() => groupGitRuns(store.gitSteps));

function shopCount(run: GitRun): number {
  return new Set(run.rows.flatMap((row) => row.repoPaths)).size;
}

function toggle(runId: string): void {
  const next = new Set(expanded.value);
  if (next.has(runId)) next.delete(runId);
  else next.add(runId);
  expanded.value = next;
}

function onKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape' && store.gitLogOpen) store.closeGitLog();
}

watch(
  () => store.gitLogOpen,
  (open) => {
    if (!open) return;
    expanded.value = new Set();
  },
);

onMounted(() => window.addEventListener('keydown', onKeydown));
onUnmounted(() => window.removeEventListener('keydown', onKeydown));
</script>

<template>
  <aside
    v-if="store.gitLogOpen"
    class="absolute inset-x-0 bottom-0 z-30 max-h-[45%] overflow-hidden border-t border-border bg-surface shadow-lg"
    role="region"
    :aria-label="t('gitLog.region')"
  >
    <header class="flex items-center justify-between border-b border-border px-4 py-2">
      <div class="flex items-center gap-2">
        <span class="text-sm font-semibold text-fg">{{ t('gitLog.title') }}</span>
        <TdBadge variant="neutral">{{ runs.length }}</TdBadge>
      </div>
      <TdButton variant="ghost" size="sm" :aria-label="t('gitLog.close')" @click="store.closeGitLog()">
        <template #icon-left><X /></template>
      </TdButton>
    </header>

    <div v-if="runs.length === 0" class="px-4 py-6 text-sm text-fg-muted">
      {{ t('gitLog.empty') }}
    </div>

    <ol v-else class="max-h-[38vh] overflow-y-auto px-4 py-2">
      <li v-for="run in runs" :key="run.runId" class="border-b border-border py-2 last:border-b-0">
        <button
          type="button"
          class="flex w-full items-start gap-2 text-left"
          :aria-expanded="expanded.has(run.runId)"
          @click="toggle(run.runId)"
        >
          <span
            class="mt-1.5 h-2 w-2 shrink-0 rounded-full"
            :class="{
              'bg-success': run.status === 'ok',
              'bg-warning': run.status === 'running',
              'bg-error': run.status === 'failed',
              'bg-fg-muted': run.status === 'skipped',
            }"
          ></span>
          <span class="min-w-0 flex-1">
            <span class="block text-sm font-medium text-fg">{{ run.title }}</span>
            <span class="block text-[11px] text-fg-muted">
              {{ t('gitLog.shops', { count: shopCount(run), shopWord: tPlural('common.shop', shopCount(run)) }) }}
              · {{ run.durationMs }} ms
            </span>
          </span>
          <span class="text-[11px] text-fg-muted">
            {{ expanded.has(run.runId) ? t('gitLog.hideCommands') : t('gitLog.showCommands') }}
          </span>
        </button>

        <div v-if="expanded.has(run.runId)" class="mt-2 space-y-1 pl-4">
          <div v-for="row in run.rows" :key="row.key">
            <p class="font-mono text-[11px] text-fg-muted"># {{ row.comment }}</p>
            <p class="font-mono text-[12px] text-fg">
              {{ row.command }}
              <span v-if="row.alias" class="text-fg-muted">  # oh-my-zsh: {{ row.alias }}</span>
            </p>
            <p class="text-[11px] text-fg-muted">
              {{ row.repoPaths.map(shopName).join(' · ') }}
            </p>
            <p v-if="row.status === 'skipped'" class="text-[11px] text-fg-muted">
              {{ t('gitLog.notNeeded') }}
            </p>
          </div>
        </div>

        <p
          v-for="row in run.rows.filter((entry) => entry.message)"
          :key="`${row.key}-error`"
          class="mt-2 rounded border border-border bg-surface-raised px-2 py-1 text-[11px] text-error-fg"
        >
          {{ row.message }}
        </p>
      </li>
    </ol>
  </aside>
</template>
