<script setup lang="ts">
import { computed, ref } from 'vue';
import { BookOpen, FolderPlus, GitBranch, Search } from 'lucide-vue-next';
import { TdButton, TdInput } from '@topdata-software-gmbh/ui-kit-web-vue3';
import RepoListTable from './RepoListTable.vue';
import { t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();
const search = ref('');

const rows = computed(() => {
  const term = search.value.trim().toLowerCase();
  if (!term) return store.filteredRepositories;

  return store.filteredRepositories.filter((repo) =>
    `${repo.name} ${repo.path} ${repo.state.currentBranch}`.toLowerCase().includes(term),
  );
});

const hasScanPaths = computed(() => store.config.scanPaths.length > 0);
</script>

<template>
  <section class="flex min-h-0 flex-1 flex-col bg-surface">
    <div class="flex shrink-0 items-center justify-between gap-4 border-b border-border px-4 py-2.5">
      <div class="w-full max-w-sm">
        <TdInput
          v-model="search"
          type="search"
          :placeholder="t('table.searchPlaceholder')"
          :aria-label="t('table.searchLabel')"
        >
          <template #prefix><Search class="h-3.5 w-3.5 text-fg-muted" /></template>
        </TdInput>
      </div>
      <span v-if="search.trim()" class="shrink-0 text-[11px] text-fg-muted">
        {{ t('common.of', { current: rows.length, total: store.filteredRepositories.length }) }}
      </span>
    </div>

    <div class="min-h-0 flex-1 overflow-auto">
      <RepoListTable v-if="rows.length > 0" :repos="rows" actions="group" />

      <div v-else-if="store.isScanning" class="space-y-2 p-6">
        <div
          v-for="index in 5"
          :key="index"
          class="h-14 animate-pulse rounded-lg border border-border bg-surface-raised/30"
        ></div>
      </div>

      <div
        v-else
        class="flex h-full min-h-72 flex-col items-center justify-center px-6 text-center"
      >
        <div class="mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-surface-raised">
          <FolderPlus v-if="!hasScanPaths" class="h-5 w-5 text-primary" />
          <GitBranch v-else class="h-5 w-5 text-fg-muted" />
        </div>

        <template v-if="!hasScanPaths">
          <h2 class="text-[15px] font-semibold text-fg">{{ t('empty.findProjects.title') }}</h2>
          <p class="mt-1.5 max-w-md text-sm leading-relaxed text-fg-muted">
            {{ t('empty.findProjects.text') }}
          </p>
          <div class="mt-4 flex items-center gap-2">
            <TdButton variant="primary" @click="store.openAddScanPath">
              {{ t('empty.addFolder') }}
            </TdButton>
            <TdButton variant="ghost" @click="store.openHelp">
              <template #icon-left><BookOpen /></template>
              {{ t('empty.howThisWorks') }}
            </TdButton>
          </div>
        </template>

        <template
          v-else-if="store.selectedGroupId !== 'all' && store.filteredRepositories.length === 0"
        >
          <h2 class="text-[15px] font-semibold text-fg">{{ t('empty.noProjectsInGroup.title') }}</h2>
          <p class="mt-1.5 max-w-md text-sm leading-relaxed text-fg-muted">
            {{ t('empty.noProjectsInGroup.text', { group: store.currentGroup?.name ?? '' }) }}
          </p>
          <TdButton class="mt-4" variant="primary" @click="store.openGroupEditor(store.currentGroup)">
            {{ t('empty.editGroup') }}
          </TdButton>
        </template>

        <template v-else-if="store.filteredRepositories.length === 0">
          <h2 class="text-[15px] font-semibold text-fg">{{ t('empty.noGitProjects.title') }}</h2>
          <p class="mt-1.5 max-w-md text-sm leading-relaxed text-fg-muted">
            {{ t('empty.noGitProjects.text') }}
          </p>
          <div class="mt-4 flex gap-2">
            <TdButton variant="secondary" @click="store.scanRepositories">{{ t('empty.lookAgain') }}</TdButton>
            <TdButton variant="primary" @click="store.openAddScanPath">
              {{ t('empty.addFolder') }}
            </TdButton>
          </div>
        </template>

        <template v-else>
          <h2 class="text-[15px] font-semibold text-fg">{{ t('empty.noMatch.title') }}</h2>
          <p class="mt-1.5 text-sm text-fg-muted">{{ t('empty.noMatch.text') }}</p>
          <TdButton class="mt-4" variant="ghost" @click="search = ''">{{ t('empty.clearSearch') }}</TdButton>
        </template>
      </div>
    </div>
  </section>
</template>
