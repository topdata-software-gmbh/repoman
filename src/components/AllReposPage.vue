<script setup lang="ts">
import { computed, ref } from 'vue';
import { FolderSearch, RefreshCw, Search, SlidersHorizontal } from 'lucide-vue-next';
import {
  TdButton,
  TdFormField,
  TdInput,
  TdSegmentedControl,
  TdSelect,
  TdSpinner,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import RepoListTable from './RepoListTable.vue';
import { t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';
import type { RepoStatusFilter } from '../lib/repoFilters';
import type { RepoKind } from '../types/domain';

const store = useRepoStore();
const filtersOpen = ref(false);

const statusOptions = computed<Array<{ value: RepoStatusFilter; label: string }>>(() => [
  { value: 'needsAttention', label: t('filters.needsAttention') },
  { value: 'changesNotSaved', label: t('filters.changesNotSaved') },
  { value: 'updatesAvailable', label: t('filters.updatesAvailable') },
  { value: 'toSend', label: t('filters.toSend') },
  { value: 'upToDate', label: t('filters.upToDate') },
  { value: 'localOnly', label: t('filters.localOnly') },
]);

const groupOptions = computed(() => [
  { value: 'all', label: t('filters.anyGroup') },
  { value: 'ungrouped', label: t('filters.ungrouped') },
  ...store.config.groups.map((group) => ({ value: group.id, label: group.name })),
]);

const kindOptions = computed<Array<{ value: RepoKind | 'all'; label: string }>>(() => [
  { value: 'all', label: t('filters.anyType') },
  { value: 'theme', label: t('filters.theme') },
  { value: 'plugin', label: t('filters.plugin') },
  { value: 'other', label: t('filters.other') },
]);

const kindFilter = computed({
  get: () => store.allReposFilters.kind,
  set: (value: string) => {
    store.allReposFilters.kind = value as RepoKind | 'all';
  },
});

const statusFilter = computed({
  get: () => store.allReposFilters.statuses,
  set: (value: string[]) => {
    store.allReposFilters.statuses = value as RepoStatusFilter[];
  },
});

const statusOptionsWithCounts = computed(() =>
  statusOptions.value.map((option) => ({
    ...option,
    count: store.allReposStatusCounts[option.value],
  })),
);

const hasFilters = computed(() => {
  const filters = store.allReposFilters;
  return (
    filters.statuses.length > 0 ||
    filters.groupId !== 'all' ||
    filters.kind !== 'all' ||
    filters.search.trim() !== ''
  );
});

const activeFilterCount = computed(() => {
  const filters = store.allReposFilters;
  return (
    filters.statuses.length +
    (filters.groupId !== 'all' ? 1 : 0) +
    (filters.kind !== 'all' ? 1 : 0)
  );
});

const folderLabel = computed(() =>
  store.config.scanPaths
    .filter((scanPath) => scanPath.enabled)
    .map((scanPath) => scanPath.path)
    .join(', '),
);

const hasRepos = computed(() => store.repositories.length > 0);
const isScanning = computed(() => store.isScanning);

// FR-011: same live stream as onboarding; the old "Prüfe {folders}" text is
// the fallback until this run's `started` event arrives.
const scanningLine = computed(() => {
  const activity = store.scanActivity;
  if (activity.running) {
    return t('scan.activity.progress', {
      dir: activity.currentDir ?? activity.roots[0] ?? folderLabel.value ?? t('allRepos.yourFolders'),
      count: store.scanActivityCount,
    });
  }
  return t('allRepos.checking', { folders: folderLabel.value || t('allRepos.yourFolders') });
});
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col bg-surface">
    <template v-if="hasRepos">
      <header class="border-b border-border px-5 pb-3 pt-5">
        <div class="flex items-baseline gap-2">
          <h1 class="text-[15px] font-semibold text-fg">{{ t('allRepos.title') }}</h1>
          <span class="text-xs text-fg-muted">
            <template v-if="hasFilters">
              {{ t('common.of', { current: store.allReposFiltered.length, total: store.repositories.length }) }}
            </template>
            <template v-else>{{ store.repositories.length }}</template>
          </span>
        </div>

        <div class="mt-3 flex items-center gap-2">
          <div class="min-w-56 flex-1">
            <TdInput
              v-model="store.allReposFilters.search"
              type="search"
              :placeholder="t('table.searchPlaceholder')"
              :aria-label="t('allRepos.searchLabel')"
            >
              <template #prefix><Search class="h-3.5 w-3.5 text-fg-muted" /></template>
            </TdInput>
          </div>
          <TdButton
            variant="secondary"
            size="sm"
            :aria-expanded="filtersOpen"
            aria-controls="all-repos-filters"
            @click="filtersOpen = !filtersOpen"
          >
            <template #icon-left><SlidersHorizontal class="h-3.5 w-3.5" /></template>
            <span>{{ t('allRepos.filter') }}</span>
            <span
              v-if="activeFilterCount > 0"
              class="ml-1 rounded-full bg-primary px-1.5 text-[10px] text-on-primary"
            >
              {{ activeFilterCount }}
            </span>
          </TdButton>
        </div>

        <div
          v-if="filtersOpen"
          id="all-repos-filters"
          class="mt-3 rounded-xl border border-border bg-canvas p-3"
        >
          <div class="grid gap-3 sm:grid-cols-2">
            <TdFormField :label="t('filters.group')">
              <template #default="{ id }">
                <TdSelect :id="id" v-model="store.allReposFilters.groupId" :options="groupOptions" />
              </template>
            </TdFormField>

            <TdFormField :label="t('filters.type')">
              <template #default="{ id }">
                <TdSelect :id="id" v-model="kindFilter" :options="kindOptions" />
              </template>
            </TdFormField>
          </div>

          <div class="mt-3">
            <p class="mb-1.5 text-[11px] font-medium text-fg-muted">{{ t('filters.status') }}</p>
            <TdSegmentedControl
              v-model="statusFilter"
              :options="statusOptionsWithCounts"
              :aria-label="t('filters.statusLabel')"
            />
          </div>

          <div v-if="hasFilters" class="mt-3 flex justify-end">
            <TdButton variant="ghost" size="sm" @click="store.clearAllReposFilters">
              {{ t('allRepos.clearFilters') }}
            </TdButton>
          </div>
        </div>
      </header>

      <RepoListTable
        v-if="store.allReposFiltered.length > 0"
        :repos="store.allReposFiltered"
        actions="browse"
        class="min-h-0 flex-1"
      />

      <div v-else class="flex flex-1 flex-col items-center justify-center px-6 text-center">
        <h2 class="text-sm font-semibold text-fg">{{ t('allRepos.noMatch') }}</h2>
        <p class="mt-1.5 text-sm text-fg-muted">{{ t('allRepos.noMatchHint') }}</p>
        <TdButton class="mt-4" variant="ghost" size="sm" @click="store.clearAllReposFilters">
          {{ t('allRepos.clearFilters') }}
        </TdButton>
      </div>
    </template>

    <div v-else class="flex flex-1 flex-col items-center justify-center px-6 text-center">
      <div
        class="flex h-12 w-12 items-center justify-center rounded-2xl bg-primary/10 text-primary"
      >
        <TdSpinner v-if="isScanning" size="md" />
        <FolderSearch v-else class="h-5 w-5" />
      </div>

      <template v-if="isScanning">
        <h2 class="mt-4 text-[15px] font-semibold text-fg">{{ t('allRepos.looking') }}</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted" role="status">
          {{ scanningLine }}
        </p>
      </template>

      <template v-else-if="store.config.scanPaths.length > 0">
        <h2 class="mt-4 text-[15px] font-semibold text-fg">{{ t('allRepos.noProjectsFound') }}</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted">
          {{ t('allRepos.lookedIn', { folders: folderLabel || t('allRepos.thatFolder') }) }}
        </p>
        <div class="mt-4 flex items-center gap-2">
          <TdButton variant="primary" size="sm" @click="store.openAddScanPath">
            <template #icon-left><FolderSearch class="h-3.5 w-3.5" /></template>
            <span>{{ t('allRepos.chooseAnother') }}</span>
          </TdButton>
          <TdButton variant="ghost" size="sm" @click="store.scanRepositories">
            <template #icon-left><RefreshCw class="h-3.5 w-3.5" /></template>
            <span>{{ t('allRepos.lookAgain') }}</span>
          </TdButton>
        </div>
      </template>

      <template v-else>
        <h2 class="mt-4 text-[15px] font-semibold text-fg">{{ t('allRepos.findProjects') }}</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted">
          {{ t('allRepos.findProjectsHint') }}
        </p>
        <TdButton class="mt-4" variant="primary" size="sm" @click="store.openAddScanPath">
          <template #icon-left><FolderSearch class="h-3.5 w-3.5" /></template>
          <span>{{ t('allRepos.chooseFolder') }}</span>
        </TdButton>
      </template>
    </div>
  </section>
</template>
