<script setup lang="ts">
import { ref } from 'vue';
import {
  AlertCircle,
  ArrowDownToLine,
  GitBranch,
  GitMerge,
  RefreshCw,
  Save,
  Upload,
} from 'lucide-vue-next';
import { TdBadge, TdButton, TdSpinner } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { useRepoStore } from '../stores/repoStore';
import { t } from '../lib/i18n';
import { branchLabel, statusBadges } from '../lib/repoStatus';
import type { Repository } from '../types/domain';

const props = withDefaults(
  defineProps<{
    repos: Repository[];
    actions?: 'group' | 'browse';
  }>(),
  { actions: 'group' },
);

const store = useRepoStore();
const refreshing = ref<string | null>(null);
const pendingAction = ref<{ path: string; action: 'pull' | 'push' } | null>(null);

async function refresh(path: string): Promise<void> {
  if (refreshing.value) return;
  refreshing.value = path;
  try {
    await store.refreshSingleRepo(path);
  } finally {
    refreshing.value = null;
  }
}

async function runAction(repo: Repository, action: 'pull' | 'push'): Promise<void> {
  if (pendingAction.value || refreshing.value) return;
  pendingAction.value = { path: repo.path, action };
  try {
    await store.runRepoAction(repo.path, action);
  } finally {
    pendingAction.value = null;
  }
}

function isRowBusy(repo: Repository): boolean {
  return refreshing.value === repo.path || pendingAction.value?.path === repo.path;
}
</script>

<template>
  <div class="min-h-0 flex-1 overflow-auto">
    <table class="w-full min-w-[720px] border-collapse text-left">
      <thead
        class="sticky top-0 z-10 border-b border-border bg-surface/95 text-[11px] font-medium text-fg-muted backdrop-blur"
      >
        <tr>
          <th class="px-4 py-2.5">{{ t('table.project') }}</th>
          <th class="px-4 py-2.5">{{ t('table.status') }}</th>
          <th class="px-4 py-2.5">{{ t('table.lastChange') }}</th>
          <th class="w-28 px-3 py-2.5"><span class="sr-only">{{ t('table.actions') }}</span></th>
        </tr>
      </thead>
      <tbody class="divide-y divide-border">
        <tr
          v-for="repo in props.repos"
          :key="repo.id"
          class="group transition-colors hover:bg-canvas/70"
          :class="{ 'opacity-60': !repo.isAccessible }"
        >
          <td class="px-4 py-3.5">
            <div class="flex items-center gap-2 text-sm font-medium text-fg">
              <span class="truncate">{{ repo.name }}</span>
              <TdBadge v-if="repo.state.isDetachedHead" variant="warning">{{ t('table.notOnBranch') }}</TdBadge>
            </div>
            <div class="mt-1 flex flex-wrap items-center gap-2">
              <TdBadge
                v-if="repo.state.isDetachedHead"
                variant="warning"
                class="gap-1 font-mono"
                :title="t('table.branchHint')"
              >
                <template #icon><GitBranch /></template>
                {{ branchLabel(repo) }}
              </TdBadge>
              <span
                v-else
                class="inline-flex items-center gap-1 font-mono text-[10px] text-fg-muted"
                :title="t('table.branchHint')"
              >
                <GitBranch class="h-3 w-3" />
                {{ branchLabel(repo) }}
              </span>
              <span class="max-w-[28rem] truncate text-[10px] text-fg-dim" :title="repo.path">
                {{ repo.path }}
              </span>
            </div>
          </td>

          <td class="px-4 py-3.5">
            <div class="flex max-w-xs flex-wrap items-center gap-1.5">
              <TdBadge
                v-for="badge in statusBadges(repo)"
                :key="badge.label"
                :variant="badge.variant"
              >
                <template #icon>
                  <AlertCircle v-if="badge.kind === 'attention'" />
                  <GitMerge v-else-if="badge.kind === 'operation'" />
                </template>
                {{ badge.label }}
              </TdBadge>
            </div>
          </td>

          <td class="px-4 py-3.5">
            <div v-if="repo.state.headCommitHash" class="min-w-0">
              <p class="max-w-md truncate text-xs text-fg" :title="repo.state.headCommitMessage">
                {{ repo.state.headCommitMessage }}
              </p>
              <p class="mt-0.5 font-mono text-[10px] text-fg-muted">
                {{ repo.state.headCommitHash }}
              </p>
            </div>
            <span v-else class="text-[11px] italic text-fg-dim">
              {{ t('table.noCheckpoints') }}
            </span>
          </td>

          <td class="px-3 py-3.5">
            <div
              class="flex items-center justify-end gap-1.5 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100"
            >
              <TdButton
                v-if="!repo.state.isClean"
                variant="ghost"
                size="sm"
                :disabled="store.isLoading"
                :title="
                  repo.state.hasConflicts
                    ? t('table.reviewConflictsHint')
                    : t('table.saveHint')
                "
                @click="store.openSaveChanges([repo.path])"
              >
                <template #icon-left><Save /></template>
                <span>{{ repo.state.hasConflicts ? t('table.review') : t('table.save') }}</span>
              </TdButton>

              <template v-if="props.actions === 'browse'">
                <TdButton
                  variant="ghost"
                  size="sm"
                  :disabled="!repo.isAccessible || store.isLoading || isRowBusy(repo)"
                  :aria-label="t('table.getLatestFor', { name: repo.name })"
                  :title="t('table.getLatestForHint')"
                  @click="runAction(repo, 'pull')"
                >
                  <template #icon-left>
                    <TdSpinner
                      v-if="pendingAction?.path === repo.path && pendingAction.action === 'pull'"
                      size="sm"
                    />
                    <ArrowDownToLine v-else class="text-success-fg" />
                  </template>
                </TdButton>
                <TdButton
                  variant="ghost"
                  size="sm"
                  :disabled="!repo.isAccessible || store.isLoading || isRowBusy(repo)"
                  :aria-label="t('table.sendFor', { name: repo.name })"
                  :title="t('table.sendForHint')"
                  @click="runAction(repo, 'push')"
                >
                  <template #icon-left>
                    <TdSpinner
                      v-if="pendingAction?.path === repo.path && pendingAction.action === 'push'"
                      size="sm"
                    />
                    <Upload v-else class="text-info-fg" />
                  </template>
                </TdButton>
              </template>

              <TdButton
                variant="ghost"
                size="sm"
                :disabled="isRowBusy(repo)"
                :aria-label="t('table.refreshFor', { name: repo.name })"
                :title="t('table.refreshHint')"
                @click="refresh(repo.path)"
              >
                <template #icon-left>
                  <TdSpinner v-if="refreshing === repo.path" size="sm" />
                  <RefreshCw v-else />
                </template>
              </TdButton>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
