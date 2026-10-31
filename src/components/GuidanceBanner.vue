<script setup lang="ts">
import { computed } from 'vue';
import {
  AlertOctagon,
  AlertTriangle,
  CheckCircle2,
  Info,
} from 'lucide-vue-next';
import { TdAlert } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();

const icon = computed(() => {
  switch (store.insight?.tone) {
    case 'danger':
      return AlertOctagon;
    case 'warning':
      return AlertTriangle;
    case 'info':
      return Info;
    default:
      return CheckCircle2;
  }
});

const alertVariant = computed<'error' | 'warning' | 'info' | 'success'>(() => {
  switch (store.insight?.tone) {
    case 'danger':
      return 'error';
    case 'warning':
      return 'warning';
    case 'info':
      return 'info';
    default:
      return 'success';
  }
});
</script>

<template>
  <div v-if="store.insight" class="border-b border-border px-4 py-3">
    <TdAlert :variant="alertVariant" :title="store.insight.title">
      <template #icon><component :is="icon" /></template>
      {{ store.insight.message }}
    </TdAlert>
  </div>
</template>
