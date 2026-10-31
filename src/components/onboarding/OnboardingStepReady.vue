<script setup lang="ts">
import { computed, ref } from 'vue';
import { ArrowRight, CheckCircle2, FolderSearch } from 'lucide-vue-next';
import { TdAlert, TdButton } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t, tPlural } from '../../lib/i18n';
import { useRepoStore } from '../../stores/repoStore';

const store = useRepoStore();
const busy = ref(false);

const groups = computed(() => store.onboardingPendingGroups);
const firstGroup = computed(() => groups.value[0] ?? null);

async function open(groupId: string | null): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    await store.completeOnboarding(groupId);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="space-y-6">
    <div class="flex items-start gap-3">
      <CheckCircle2 class="mt-1 h-6 w-6 shrink-0 text-success-fg" />
      <div>
        <h2 class="text-lg font-semibold text-fg">{{ t('onboarding.ready.title') }}</h2>
        <p class="mt-1.5 text-sm leading-relaxed text-fg-muted">
          {{
            t('onboarding.ready.summary', {
              projects: store.onboardingSelectedRepositories.length,
              projectWord: tPlural('common.project', store.onboardingSelectedRepositories.length),
              groups: groups.length,
              groupWord: tPlural('common.group', groups.length),
            })
          }}
        </p>
      </div>
    </div>

    <div class="grid gap-3 sm:grid-cols-3">
      <div class="rounded-xl border border-border bg-surface p-4">
        <h3 class="text-sm font-semibold text-fg">{{ t('onboarding.ready.saveTitle') }}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-fg-muted">
          {{ t('onboarding.ready.saveText') }}
        </p>
      </div>
      <div class="rounded-xl border border-border bg-surface p-4">
        <h3 class="text-sm font-semibold text-fg">{{ t('onboarding.ready.pullTitle') }}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-fg-muted">
          {{ t('onboarding.ready.pullText') }}
        </p>
      </div>
      <div class="rounded-xl border border-border bg-surface p-4">
        <h3 class="text-sm font-semibold text-fg">{{ t('onboarding.ready.pushTitle') }}</h3>
        <p class="mt-1.5 text-xs leading-relaxed text-fg-muted">
          {{ t('onboarding.ready.pushText') }}
        </p>
      </div>
    </div>

    <TdAlert v-if="store.onboardingError" variant="error">{{ store.onboardingError }}</TdAlert>

    <div class="space-y-2 rounded-xl border border-border bg-canvas p-4">
      <p class="text-xs font-medium text-fg-muted">{{ t('onboarding.ready.openTitle') }}</p>
      <div class="flex flex-wrap items-center gap-2">
        <TdButton
          v-if="firstGroup"
          variant="primary"
          :disabled="busy"
          @click="open(firstGroup.id)"
        >
          {{ t('onboarding.ready.openGroup', { name: firstGroup.name }) }}
          <template #icon-right><ArrowRight /></template>
        </TdButton>
        <TdButton variant="secondary" :disabled="busy" @click="open(null)">
          <template #icon-left><FolderSearch /></template>
          {{ t('onboarding.ready.openAll') }}
        </TdButton>
      </div>

      <div v-if="groups.length > 1" class="flex flex-wrap gap-2 pt-1">
        <TdButton
          v-for="group in groups.slice(1)"
          :key="group.id"
          variant="secondary"
          size="sm"
          :disabled="busy"
          @click="open(group.id)"
        >
          {{ group.name }}
        </TdButton>
      </div>
    </div>
  </div>
</template>
