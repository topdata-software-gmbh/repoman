<script setup lang="ts">
import { computed, ref } from 'vue';
import { FolderSearch } from 'lucide-vue-next';
import {
  TdAlert,
  TdButton,
  TdFormField,
  TdInput,
  TdSelect,
  TdSpinner,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../../lib/i18n';
import { useRepoStore } from '../../stores/repoStore';
import type { GroupingStyle } from '../../types/domain';

const store = useRepoStore();
const submitting = ref(false);

const maxDepth = computed({
  get: () => String(store.onboardingDraft.maxDepth),
  set: (value: string) => {
    const parsed = Number.parseInt(value, 10);
    store.onboardingDraft.maxDepth = Number.isFinite(parsed) ? parsed : 1;
  },
});

const groupingStyle = computed({
  get: () => store.onboardingDraft.groupingStyle,
  set: (value: string) => {
    store.onboardingDraft.groupingStyle = value as GroupingStyle;
  },
});

const groupingOptions = computed(() => [
  { value: 'topLevel', label: t('onboarding.find.groupingTopLevel') },
  { value: 'single', label: t('onboarding.find.groupingSingle') },
]);

async function submit(): Promise<void> {
  if (submitting.value) return;
  submitting.value = true;
  try {
    const found = await store.scanOnboardingFolder();
    if (found) {
      await store.goToOnboardingStep(1);
    }
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <form class="mx-auto max-w-xl space-y-6" @submit.prevent="submit">
    <div>
      <h2 class="text-lg font-semibold text-fg">{{ t('onboarding.find.title') }}</h2>
      <p class="mt-1.5 text-sm leading-relaxed text-fg-muted">
        {{ t('onboarding.find.text') }}
      </p>
    </div>

    <div class="space-y-4 rounded-xl border border-border bg-surface p-5">
      <TdFormField
        :label="t('onboarding.find.pathLabel')"
        :hint="t('onboarding.find.pathHint')"
        required
        v-slot="{ id, 'aria-describedby': describedBy, invalid, required }"
      >
        <TdInput
          :id="id"
          v-model="store.onboardingDraft.scanPath"
          type="text"
          spellcheck="false"
          placeholder="~/devel"
          :aria-describedby="describedBy"
          :invalid="invalid"
          :required="required"
        />
      </TdFormField>
    </div>

    <details class="rounded-xl border border-border bg-surface px-5 py-4">
      <summary class="cursor-pointer text-xs font-medium text-fg">{{ t('onboarding.find.moreOptions') }}</summary>
      <div class="mt-4 space-y-4">
        <TdFormField
          :label="t('onboarding.find.ownersLabel')"
          :hint="t('onboarding.find.ownersHint')"
          v-slot="{ id, 'aria-describedby': describedBy }"
        >
          <TdInput
            :id="id"
            v-model="store.onboardingDraft.ownerNames"
            type="text"
            spellcheck="false"
            placeholder="acme, my-company"
            :aria-describedby="describedBy"
          />
        </TdFormField>

        <div class="grid gap-4 sm:grid-cols-2">
          <TdFormField
            :label="t('onboarding.find.depthLabel')"
            :hint="t('onboarding.find.depthHint')"
            v-slot="{ id, 'aria-describedby': describedBy }"
          >
            <TdInput
              :id="id"
              v-model="maxDepth"
              type="number"
              min="1"
              max="12"
              :aria-describedby="describedBy"
            />
          </TdFormField>

          <TdFormField
            :label="t('onboarding.find.groupingLabel')"
            :hint="t('onboarding.find.groupingHint')"
            v-slot="{ id, 'aria-describedby': describedBy }"
          >
            <TdSelect
              :id="id"
              v-model="groupingStyle"
              :options="groupingOptions"
              :aria-describedby="describedBy"
            />
          </TdFormField>
        </div>
      </div>
    </details>

    <TdAlert v-if="store.onboardingError" variant="error">{{ store.onboardingError }}</TdAlert>

    <p
      v-if="submitting"
      class="flex items-center gap-2 text-sm text-fg-muted"
      role="status"
    >
      <TdSpinner size="sm" />
      {{ t('onboarding.find.scanning') }}
    </p>

    <div class="flex items-center gap-3">
      <TdButton
        type="submit"
        variant="primary"
        :disabled="submitting"
        :loading="submitting"
      >
        <template #icon-left><FolderSearch /></template>
        <span>{{ t('onboarding.find.submit') }}</span>
      </TdButton>
    </div>
  </form>
</template>
