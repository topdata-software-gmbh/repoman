<script setup lang="ts">
import {
  Archive,
  Download,
  GitBranch,
  Save,
  Upload,
} from 'lucide-vue-next';
import { TdAlert, TdBadge, TdButton, TdModal } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();

const steps = [
  {
    title: 'Add your project folder once',
    text: 'Choose the folder where your themes and projects live. The app finds the Git projects inside it.',
  },
  {
    title: 'Edit your files like you normally do',
    text: 'Use your editor. The app watches each project and shows when something changed.',
  },
  {
    title: 'Save to Git when you reach a checkpoint',
    text: 'A checkpoint is a commit: a snapshot of your work that you can return to later.',
  },
  {
    title: 'Get latest, then send changes',
    text: 'Get latest before you start working. Send your saved changes when you are finished.',
  },
];

const words = [
  {
    icon: Save,
    name: 'Save to Git',
    text: 'Creates a checkpoint (commit) of your changes in that project. Your files stay in the same folder.',
  },
  {
    icon: Download,
    name: 'Get latest',
    text: 'Downloads new work from the server and updates your project folder. Local changes are set aside and put back afterwards.',
  },
  {
    icon: Upload,
    name: 'Send changes',
    text: 'Uploads your saved checkpoints so other people can see them.',
  },
  {
    icon: GitBranch,
    name: 'Branch',
    text: 'A separate line of work. Switching branches changes the files in your project folder to that version.',
  },
  {
    icon: Archive,
    name: 'Set aside changes',
    text: 'Temporarily stores unfinished changes (a stash) so the folder is clean. Bring them back when you need them.',
  },
];
</script>

<template>
  <TdModal
    :open="store.helpModalOpen"
    title="How this works"
    description="A short guide to working with Git without the jargon."
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
          Git words in plain English
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

      <TdAlert variant="warning" title="If something goes wrong">
        Read the message at the bottom of the screen. It always explains the next step in plain
        language. Your work is not deleted by the buttons in this app.
      </TdAlert>
    </div>

    <template #footer>
      <TdButton variant="primary" @click="store.closeHelp">Got it</TdButton>
    </template>
  </TdModal>
</template>
