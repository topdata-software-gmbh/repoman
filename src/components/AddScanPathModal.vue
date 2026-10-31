<script setup lang="ts">
import { ref, watch } from 'vue';
import { TdButton, TdFormField, TdInput, TdModal } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();
const path = ref('');
const maxDepth = ref('3');
const submitting = ref(false);

watch(
  () => store.scanModalOpen,
  (open) => {
    if (!open) return;
    path.value = '';
    maxDepth.value = '3';
    submitting.value = false;
  },
);

function newId(prefix: string): string {
  return `${prefix}_${typeof crypto.randomUUID === 'function' ? crypto.randomUUID() : Date.now()}`;
}

function onOpenChange(open: boolean): void {
  if (!open) store.closeScanPathModal();
}

async function submit(): Promise<void> {
  const value = path.value.trim();
  if (!value || submitting.value) return;

  submitting.value = true;
  try {
    const parsed = Number(maxDepth.value);
    const depth = Number.isFinite(parsed) ? Math.round(parsed) : 3;
    const added = await store.addScanPath({
      id: newId('scan'),
      path: value,
      maxDepth: Math.min(Math.max(depth, 1), 12),
      enabled: true,
      excludePatterns: [],
    });
    if (added) {
      store.closeScanPathModal();
    }
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <TdModal
    :open="store.scanModalOpen"
    :title="t('scan.title')"
    @update:open="onOpenChange"
  >
    <form id="scan-path-form" class="scan-form" @submit.prevent="submit">
      <TdFormField
        :label="t('scan.pathLabel')"
        :hint="t('scan.pathHint')"
        required
        v-slot="{ id, 'aria-describedby': describedBy, invalid, required }"
      >
        <TdInput
          :id="id"
          v-model="path"
          type="text"
          spellcheck="false"
          placeholder="~/devel"
          :aria-describedby="describedBy"
          :invalid="invalid"
          :required="required"
          data-autofocus
        />
      </TdFormField>

      <details class="scan-form__advanced">
        <summary class="cursor-pointer text-xs font-medium text-fg-muted">{{ t('scan.moreOptions') }}</summary>
        <div class="scan-form__depth mt-3">
          <TdFormField
            :label="t('scan.depthLabel')"
            :hint="t('scan.depthHint')"
            v-slot="{ id }"
          >
            <TdInput :id="id" v-model="maxDepth" type="number" min="1" max="12" />
          </TdFormField>
        </div>
      </details>
    </form>

    <template #footer>
      <TdButton variant="ghost" @click="store.closeScanPathModal">{{ t('scan.cancel') }}</TdButton>
      <TdButton
        variant="primary"
        type="submit"
        form="scan-path-form"
        :disabled="!path.trim() || submitting"
        :loading="submitting"
      >
        {{ t('scan.find') }}
      </TdButton>
    </template>
  </TdModal>
</template>

<style scoped>
.scan-form {
  display: grid;
  gap: var(--td-space-4);
}

.scan-form__depth {
  max-width: 10rem;
}
</style>
