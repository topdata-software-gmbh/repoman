<script setup lang="ts">
import {
  Archive,
  Download,
  GitBranch,
  Save,
  Upload,
} from 'lucide-vue-next';
import { computed } from 'vue';
import { TdAlert, TdBadge, TdButton, TdModal } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();

const steps = computed(() => [
  { title: t('help.step1.title'), text: t('help.step1.text') },
  { title: t('help.step2.title'), text: t('help.step2.text') },
  { title: t('help.step3.title'), text: t('help.step3.text') },
  { title: t('help.step4.title'), text: t('help.step4.text') },
]);

const words = computed(() => [
  { icon: Save, name: t('help.word.save.name'), text: t('help.word.save.text') },
  { icon: Download, name: t('help.word.pull.name'), text: t('help.word.pull.text') },
  { icon: Upload, name: t('help.word.push.name'), text: t('help.word.push.text') },
  { icon: GitBranch, name: t('help.word.branch.name'), text: t('help.word.branch.text') },
  { icon: Archive, name: t('help.word.stash.name'), text: t('help.word.stash.text') },
]);
</script>

<template>
  <TdModal
    :open="store.helpModalOpen"
    :title="t('help.title')"
    :description="t('help.description')"
    size="lg"
    @close="store.closeHelp"
  >
    <div class="space-y-6">
      <ol class="space-y-3">
        <li v-for="(step, index) in steps" :key="step.title" class="flex gap-3">
          <TdBadge variant="primary" size="md" class="h-6 w-6 shrink-0 justify-center">
            {{ index + 1 }}
          </TdBadge>
          <div>
            <p class="text-sm font-medium text-fg">{{ step.title }}</p>
            <p class="mt-0.5 text-xs leading-relaxed text-fg-muted">{{ step.text }}</p>
          </div>
        </li>
      </ol>

      <div>
        <h3 class="mb-2 text-xs font-semibold text-fg-muted">
          {{ t('help.wordsTitle') }}
        </h3>
        <div class="space-y-2">
          <div
            v-for="word in words"
            :key="word.name"
            class="flex gap-3 rounded-lg border border-border bg-surface px-3 py-2.5"
          >
            <component :is="word.icon" class="mt-0.5 h-4 w-4 shrink-0 text-primary" />
            <div>
              <p class="text-xs font-medium text-fg">{{ word.name }}</p>
              <p class="text-xs leading-relaxed text-fg-muted">{{ word.text }}</p>
            </div>
          </div>
        </div>
      </div>

      <TdAlert variant="warning" :title="t('help.warningTitle')">
        {{ t('help.warningText') }}
      </TdAlert>
    </div>

    <template #footer>
      <TdButton variant="primary" @click="store.closeHelp">{{ t('help.gotIt') }}</TdButton>
    </template>
  </TdModal>
</template>
