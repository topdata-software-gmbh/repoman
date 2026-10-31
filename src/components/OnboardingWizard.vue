<script setup lang="ts">
import { computed, ref } from 'vue';
import { X } from 'lucide-vue-next';
import { TdButton, TdModal } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { useRepoStore } from '../stores/repoStore';
import OnboardingStepChoose from './onboarding/OnboardingStepChoose.vue';
import OnboardingStepFind from './onboarding/OnboardingStepFind.vue';
import OnboardingStepGroups from './onboarding/OnboardingStepGroups.vue';
import OnboardingStepReady from './onboarding/OnboardingStepReady.vue';

const store = useRepoStore();
const heading = ref<HTMLElement | null>(null);
const steps = ['Find projects', 'Choose projects', 'Confirm groups', 'Ready'] as const;
const currentLabel = computed(() => steps[store.onboardingStep] ?? steps[0]);
</script>

<template>
  <TdModal
    :open="store.onboardingOpen"
    size="full"
    flush
    :show-close="false"
    :close-on-backdrop="false"
    aria-labelledby="onboarding-title"
    @close="store.closeOnboarding"
  >
    <template #header>
      <div class="w-full">
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-xs font-medium text-fg-muted">
              Step {{ store.onboardingStep + 1 }} of 4 · {{ currentLabel }}
            </p>
            <h1
              id="onboarding-title"
              ref="heading"
              tabindex="-1"
              data-autofocus
              class="mt-1 text-xl font-semibold text-fg outline-none"
            >
              Let's set up your workspace
            </h1>
          </div>
          <TdButton
            variant="ghost"
            aria-label="Close setup and resume later"
            @click="store.closeOnboarding"
          >
            <template #icon-left><X /></template>
          </TdButton>
        </div>
        <ol class="mt-4 flex gap-2" aria-label="Setup progress">
          <li
            v-for="(step, index) in steps"
            :key="step"
            class="h-1 flex-1 rounded-full"
            :class="index <= store.onboardingStep ? 'bg-primary' : 'bg-surface-raised'"
          >
            <span class="sr-only">{{ step }}</span>
          </li>
        </ol>
      </div>
    </template>

    <main class="mx-auto w-full max-w-4xl px-6 py-8">
      <OnboardingStepFind v-if="store.onboardingStep === 0" />
      <OnboardingStepChoose v-else-if="store.onboardingStep === 1" />
      <OnboardingStepGroups v-else-if="store.onboardingStep === 2" />
      <OnboardingStepReady v-else />
    </main>
  </TdModal>
</template>
