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
import { useRepoStore } from '../stores/repoStore';
import type { RepoStatusFilter } from '../lib/repoFilters';
import type { RepoKind } from '../types/domain';

const store = useRepoStore();
const filtersOpen = ref(false);

const statusOptions: Array<{ value: RepoStatusFilter; label: string }> = [
  { value: 'needsAttention', label: 'Needs attention' },
  { value: 'changesNotSaved', label: 'Changes not saved' },
  { value: 'updatesAvailable', label: 'Updates available' },
  { value: 'toSend', label: 'To send' },
  { value: 'upToDate', label: 'Up to date' },
  { value: 'localOnly', label: 'On this computer only' },
];

const groupOptions = computed(() => [
  { value: 'all', label: 'Any group' },
  { value: 'ungrouped', label: 'Ungrouped' },
  ...store.config.groups.map((group) => ({ value: group.id, label: group.name })),
]);

const kindOptions: Array<{ value: RepoKind | 'all'; label: string }> = [
  { value: 'all', label: 'Any type' },
  { value: 'theme', label: 'Theme' },
  { value: 'plugin', label: 'Plugin' },
  { value: 'other', label: 'Other' },
];

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
  statusOptions.map((option) => ({
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
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col bg-surface">
    <template v-if="hasRepos">
      <header class="border-b border-border px-5 pb-3 pt-5">
        <div class="flex items-baseline gap-2">
          <h1 class="text-[15px] font-semibold text-fg">All projects</h1>
          <span class="text-xs text-fg-muted">
            <template v-if="hasFilters">
              {{ store.allReposFiltered.length }} of {{ store.repositories.length }}
            </template>
            <template v-else>{{ store.repositories.length }}</template>
          </span>
        </div>

        <div class="mt-3 flex items-center gap-2">
          <div class="min-w-56 flex-1">
            <TdInput
              v-model="store.allReposFilters.search"
              type="search"
              placeholder="Search projects…"
              aria-label="Search all projects"
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
            <span>Filter</span>
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
            <TdFormField label="Group">
              <template #default="{ id }">
                <TdSelect :id="id" v-model="store.allReposFilters.groupId" :options="groupOptions" />
              </template>
            </TdFormField>

            <TdFormField label="Type">
              <template #default="{ id }">
                <TdSelect :id="id" v-model="kindFilter" :options="kindOptions" />
              </template>
            </TdFormField>
          </div>

          <div class="mt-3">
            <p class="mb-1.5 text-[11px] font-medium text-fg-muted">Status</p>
            <TdSegmentedControl
              v-model="statusFilter"
              :options="statusOptionsWithCounts"
              aria-label="Filter by status"
            />
          </div>

          <div v-if="hasFilters" class="mt-3 flex justify-end">
            <TdButton variant="ghost" size="sm" @click="store.clearAllReposFilters">
              Clear filters
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
        <h2 class="text-sm font-semibold text-fg">No projects match</h2>
        <p class="mt-1.5 text-sm text-fg-muted">Try a different search or filter.</p>
        <TdButton class="mt-4" variant="ghost" size="sm" @click="store.clearAllReposFilters">
          Clear filters
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
        <h2 class="mt-4 text-[15px] font-semibold text-fg">Looking for projects…</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted">
          Checking {{ folderLabel || 'your folders' }}. Large folders can take a minute.
        </p>
      </template>

      <template v-else-if="store.config.scanPaths.length > 0">
        <h2 class="mt-4 text-[15px] font-semibold text-fg">No projects found</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted">
          We looked in {{ folderLabel || 'that folder' }} but did not find a Git project.
        </p>
        <div class="mt-4 flex items-center gap-2">
          <TdButton variant="primary" size="sm" @click="store.openAddScanPath">
            <template #icon-left><FolderSearch class="h-3.5 w-3.5" /></template>
            <span>Choose another folder</span>
          </TdButton>
          <TdButton variant="ghost" size="sm" @click="store.scanRepositories">
            <template #icon-left><RefreshCw class="h-3.5 w-3.5" /></template>
            <span>Look again</span>
          </TdButton>
        </div>
      </template>

      <template v-else>
        <h2 class="mt-4 text-[15px] font-semibold text-fg">Find your projects</h2>
        <p class="mt-1.5 max-w-sm text-sm leading-relaxed text-fg-muted">
          Choose the folder where your projects live. We will find the Git projects inside.
        </p>
        <TdButton class="mt-4" variant="primary" size="sm" @click="store.openAddScanPath">
          <template #icon-left><FolderSearch class="h-3.5 w-3.5" /></template>
          <span>Choose folder</span>
        </TdButton>
      </template>
    </div>
  </section>
</template>
