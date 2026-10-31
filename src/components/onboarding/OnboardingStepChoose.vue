<script setup lang="ts">
import { computed } from 'vue';
import { ArrowLeft, ArrowRight, Search } from 'lucide-vue-next';
import {
  TdAlert,
  TdBadge,
  TdButton,
  TdCheckbox,
  TdInput,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t, tPlural } from '../../lib/i18n';
import { useRepoStore } from '../../stores/repoStore';
import type { RepoKind } from '../../types/domain';

const store = useRepoStore();

const visibleRepositories = computed(() => {
  const term = store.onboardingSearch.trim().toLowerCase();
  if (!term) return store.repositories;
  return store.repositories.filter((repo) =>
    `${repo.name} ${repo.path} ${repo.state.currentBranch}`.toLowerCase().includes(term),
  );
});

const selectedCount = computed(() => store.onboardingSelectedPaths.length);

function kindLabel(kind: RepoKind): string {
  switch (kind) {
    case 'theme':
      return t('filters.theme');
    case 'plugin':
      return t('filters.plugin');
    default:
      return t('filters.other');
  }
}

function isSelected(path: string): boolean {
  return store.onboardingSelectedPaths.includes(path);
}

function selectVisible(): void {
  store.selectAllOnboardingRepos([
    ...store.onboardingSelectedPaths,
    ...visibleRepositories.value.map((repo) => repo.path),
  ]);
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-semibold text-fg">{{ t('onboarding.choose.title') }}</h2>
      <p class="mt-1.5 text-sm leading-relaxed text-fg-muted">
        {{ t('onboarding.choose.text') }}
      </p>
    </div>

    <div class="flex flex-wrap items-center gap-2">
      <div class="min-w-56 flex-1">
        <TdInput
          v-model="store.onboardingSearch"
          type="search"
          :placeholder="t('onboarding.choose.searchPlaceholder')"
          :aria-label="t('onboarding.choose.searchLabel')"
        >
          <template #prefix><Search class="h-3.5 w-3.5 text-fg-muted" /></template>
        </TdInput>
      </div>
      <TdButton variant="secondary" @click="selectVisible">{{ t('onboarding.choose.selectVisible') }}</TdButton>
      <TdButton variant="ghost" @click="store.clearOnboardingSelection">
        {{ t('onboarding.choose.clearSelection') }}
      </TdButton>
    </div>

    <p class="font-mono text-xs text-fg-muted">
      {{
        t('common.projectsSelected', {
          count: selectedCount,
          total: store.repositories.length,
          projectWord: tPlural('common.project', store.repositories.length),
        })
      }}
    </p>

    <div v-if="store.isScanning" class="space-y-2">
      <div
        v-for="index in 5"
        :key="index"
        class="h-14 animate-pulse rounded-lg border border-border bg-surface-raised/30"
      ></div>
    </div>

    <div
      v-else-if="visibleRepositories.length > 0"
      class="max-h-[50vh] divide-y divide-border overflow-auto rounded-xl border border-border bg-canvas"
    >
      <TdCheckbox
        v-for="repo in visibleRepositories"
        :key="repo.id"
        :model-value="isSelected(repo.path)"
        class="w-full px-4 py-2.5 hover:bg-surface"
        @update:model-value="store.toggleOnboardingRepo(repo.path)"
      >
        <span class="flex min-w-0 flex-1 items-center gap-3">
          <span class="min-w-0 flex-1">
            <span class="flex items-center gap-2">
              <span class="truncate text-sm text-fg">{{ repo.name }}</span>
              <TdBadge variant="neutral">{{ kindLabel(repo.kind) }}</TdBadge>
            </span>
            <span class="block truncate font-mono text-[10px] text-fg-muted">{{ repo.path }}</span>
          </span>
          <span class="shrink-0 font-mono text-[10px] text-primary">
            {{ repo.state.currentBranch }}
          </span>
        </span>
      </TdCheckbox>
    </div>

    <div
      v-else-if="store.onboardingSearch.trim()"
      class="rounded-xl border border-dashed border-border px-5 py-10 text-center"
    >
      <h3 class="text-sm font-semibold text-fg">{{ t('onboarding.choose.noMatch') }}</h3>
      <p class="mt-1 text-xs text-fg-muted">{{ t('onboarding.choose.noMatchHint') }}</p>
      <TdButton class="mt-3" variant="ghost" size="sm" @click="store.onboardingSearch = ''">
        {{ t('onboarding.choose.clearSearch') }}
      </TdButton>
    </div>

    <div
      v-else
      class="rounded-xl border border-dashed border-border px-5 py-10 text-center"
    >
      <h3 class="text-sm font-semibold text-fg">{{ t('onboarding.choose.noProjects') }}</h3>
      <p class="mt-1 text-xs text-fg-muted">
        {{ t('onboarding.choose.noProjectsHint') }}
      </p>
    </div>

    <TdAlert v-if="store.onboardingError" variant="error">{{ store.onboardingError }}</TdAlert>

    <div class="flex items-center justify-between gap-3">
      <TdButton variant="ghost" @click="store.goToOnboardingStep(0)">
        <template #icon-left><ArrowLeft /></template>
        {{ t('common.back') }}
      </TdButton>
      <TdButton
        variant="primary"
        :disabled="selectedCount === 0 || store.isScanning"
        @click="store.goToOnboardingStep(2)"
      >
        {{ t('common.continue') }}
        <template #icon-right><ArrowRight /></template>
      </TdButton>
    </div>
  </div>
</template>
