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

const groupingOptions = [
  { value: 'topLevel', label: 'By top-level folder' },
  { value: 'single', label: 'One group for everything' },
];

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
      <h2 class="text-lg font-semibold text-fg">Where do your projects live?</h2>
      <p class="mt-1.5 text-sm leading-relaxed text-fg-muted">
        Choose the folder that holds your themes and plugins. We will find the Git projects inside.
      </p>
    </div>

    <div class="space-y-4 rounded-xl border border-border bg-surface p-5">
      <TdFormField
        label="Project folder"
        hint="For example ~/devel or /home/you/projects."
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
      <summary class="cursor-pointer text-xs font-medium text-fg">More options</summary>
      <div class="mt-4 space-y-4">
        <TdFormField
          label="Remote owner names (optional)"
          hint="Used only to pre-select projects you own. This stays on this computer."
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
            label="How deep to look"
            hint="The default of 4 finds most setups."
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
            label="How to suggest groups"
            hint="You can rename or skip groups later."
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
      Looking for projects. Large folders can take a minute…
    </p>

    <div class="flex items-center gap-3">
      <TdButton
        type="submit"
        variant="primary"
        :disabled="submitting"
        :loading="submitting"
      >
        <template #icon-left><FolderSearch /></template>
        <span>Find my projects</span>
      </TdButton>
    </div>
  </form>
</template>
