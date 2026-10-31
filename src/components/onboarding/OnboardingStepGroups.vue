<script setup lang="ts">
import { computed, ref } from 'vue';
import { ArrowLeft, ArrowRight, Layers } from 'lucide-vue-next';
import {
  TdAlert,
  TdBadge,
  TdButton,
  TdCheckbox,
  TdInput,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t, tPlural } from '../../lib/i18n';
import { useRepoStore } from '../../stores/repoStore';
import { createGroup } from '../../lib/groupDefaults';

const store = useRepoStore();
const submitting = ref(false);
const localError = ref('');

const includedGroups = computed(() =>
  store.onboardingGroups.filter((group) => group.included),
);

const errorMessage = computed(() => localError.value || store.onboardingError);

function repoName(path: string): string {
  return store.repositories.find((repo) => repo.path === path)?.name ?? path.split(/[\\/]/).pop() ?? path;
}

function repoSummary(paths: string[]): string {
  const names = paths.slice(0, 3).map(repoName);
  const suffix = paths.length > 3 ? t('onboarding.groups.more', { count: paths.length - 3 }) : '';
  return names.join(', ') + suffix;
}

const acceptedLabel = computed(() =>
  t('onboarding.groups.accepted', {
    count: store.onboardingSelectedRepositories.length,
    projectWord: tPlural('common.project', store.onboardingSelectedRepositories.length),
  }),
);

async function finish(): Promise<void> {
  localError.value = '';
  const names = includedGroups.value.map((group) => group.name.trim().toLowerCase());

  if (names.length === 0) {
    localError.value = t('onboarding.groups.errorKeepOne');
    return;
  }
  if (names.some((name) => name.length === 0)) {
    localError.value = t('onboarding.groups.errorNameAll');
    return;
  }
  if (new Set(names).size !== names.length) {
    localError.value = t('onboarding.groups.errorUnique');
    return;
  }
  if (submitting.value) return;

  submitting.value = true;
  try {
    const groups = includedGroups.value.map((group, index) =>
      createGroup(group.name.trim(), group.repoPaths, index),
    );
    await store.prepareOnboardingGroups(groups);
  } catch (error) {
    localError.value = String(error);
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <div class="space-y-5">
    <div>
      <h2 class="text-lg font-semibold text-fg">{{ t('onboarding.groups.title') }}</h2>
      <p class="mt-1.5 text-sm leading-relaxed text-fg-muted">
        {{ t('onboarding.groups.text') }}
      </p>
    </div>

    <p class="font-mono text-xs text-fg-muted">
      {{ acceptedLabel }}
    </p>

    <div v-if="store.onboardingGroups.length > 0" class="space-y-3">
      <div
        v-for="(group, index) in store.onboardingGroups"
        :key="index"
        class="rounded-xl border border-border bg-canvas px-4 py-3"
        :class="{ 'opacity-60': !group.included }"
      >
        <div class="flex items-start gap-3">
          <TdCheckbox
            :model-value="group.included"
            :aria-label="t('onboarding.groups.include', { name: group.name })"
            class="mt-1"
            @update:model-value="store.toggleOnboardingGroup(index)"
          />
          <div class="min-w-0 flex-1 space-y-2">
            <div class="flex flex-wrap items-center gap-2">
              <Layers class="h-3.5 w-3.5 shrink-0 text-primary" />
              <div class="min-w-0 flex-1">
                <TdInput
                  :model-value="group.name"
                  type="text"
                  :aria-label="t('onboarding.groups.nameLabel', { index: index + 1 })"
                  @update:model-value="store.updateOnboardingGroupName(index, $event)"
                />
              </div>
              <TdBadge variant="neutral">
                {{ group.repoPaths.length }}
                {{ tPlural('common.project', group.repoPaths.length) }}
              </TdBadge>
            </div>
            <p class="text-[11px] text-fg-muted">{{ repoSummary(group.repoPaths) }}</p>
          </div>
        </div>
      </div>
    </div>

    <div
      v-else
      class="rounded-xl border border-dashed border-border px-5 py-10 text-center text-sm text-fg-muted"
    >
      {{ t('onboarding.groups.empty') }}
    </div>

    <TdAlert v-if="errorMessage" variant="error">{{ errorMessage }}</TdAlert>

    <div class="flex items-center justify-between gap-3">
      <TdButton variant="ghost" @click="store.goToOnboardingStep(1)">
        <template #icon-left><ArrowLeft /></template>
        {{ t('common.back') }}
      </TdButton>
      <TdButton
        variant="primary"
        :disabled="includedGroups.length === 0 || submitting"
        :loading="submitting"
        @click="finish"
      >
        {{ t('onboarding.groups.finish') }}
        <template #icon-right><ArrowRight /></template>
      </TdButton>
    </div>
  </div>
</template>
