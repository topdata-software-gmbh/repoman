<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { FileText, Save } from 'lucide-vue-next';
import {
  TdAlert,
  TdBadge,
  TdButton,
  TdFormField,
  TdModal,
  TdSpinner,
  TdTextarea,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { backend } from '../lib/backend';
import { t, tPlural } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';
import type { FileChange, FileChangeStatus, Repository } from '../types/domain';

const store = useRepoStore();

const repos = ref<Repository[]>([]);
const changes = ref<Record<string, FileChange[]>>({});
const message = ref('');
const loading = ref(false);
const saving = ref(false);
const error = ref('');

const totalFiles = computed(() =>
  repos.value.reduce((sum, repo) => sum + (changes.value[repo.path]?.length ?? 0), 0),
);

const canSave = computed(
  () => message.value.trim().length > 0 && repos.value.length > 0 && !loading.value && !saving.value,
);

watch(
  () => store.saveModalOpen,
  (open) => {
    if (!open) return;

    repos.value = store.dirtyRepositories.filter(
      (repo) => !store.saveModalPaths || store.saveModalPaths.includes(repo.path),
    );
    changes.value = {};
    message.value = '';
    error.value = '';
    saving.value = false;
    void loadChanges();
  },
);

async function loadChanges(): Promise<void> {
  loading.value = true;
  error.value = '';

  const results = await Promise.allSettled(
    repos.value.map(async (repo) => ({
      path: repo.path,
      changes: await backend.listRepoChanges(repo.path),
    })),
  );

  const next: Record<string, FileChange[]> = {};
  const failed: string[] = [];
  results.forEach((result, index) => {
    const repo = repos.value[index];
    if (!repo) return;
    if (result.status === 'fulfilled') {
      next[repo.path] = result.value.changes;
    } else {
      failed.push(repo.name);
      next[repo.path] = [];
    }
  });

  changes.value = next;
  if (failed.length > 0) {
    error.value = t('save.couldNotRead', { names: failed.join(', ') });
  }
  loading.value = false;
}

function statusLabel(status: FileChangeStatus): string {
  switch (status) {
    case 'new':
      return t('save.status.new');
    case 'deleted':
      return t('save.status.deleted');
    case 'renamed':
      return t('save.status.renamed');
    case 'conflict':
      return t('save.status.conflict');
    default:
      return t('save.status.changed');
  }
}

type ChangeVariant = 'success' | 'error' | 'info' | 'warning' | 'neutral';

function statusVariant(status: FileChangeStatus): ChangeVariant {
  switch (status) {
    case 'new':
      return 'success';
    case 'deleted':
    case 'conflict':
      return 'error';
    case 'renamed':
      return 'info';
    default:
      return 'warning';
  }
}

async function save(): Promise<void> {
  if (!canSave.value) return;

  saving.value = true;
  error.value = '';
  const paths = repos.value.map((repo) => repo.path);
  const results = await store.commitRepositories(paths, message.value.trim());
  const failures = results.filter((result) => !result.success);

  if (failures.length === 0) {
    store.showToast(
      'success',
      t('save.savedTitle'),
      t('save.savedBody', {
        count: repos.value.length,
        projectWord: tPlural('common.project', repos.value.length),
      }),
    );
    saving.value = false;
    store.closeSaveChanges();
    return;
  }

  const failedPaths = new Set(failures.map((failure) => failure.repoPath));
  repos.value = repos.value.filter((repo) => failedPaths.has(repo.path));
  message.value = '';
  error.value = failures
    .map((failure) => `${failure.repoPath.split(/[\\/]/).pop()}: ${failure.error ?? t('common.unknownError')}`)
    .join('\n\n');
  saving.value = false;
  await loadChanges();
}
</script>

<template>
  <TdModal
    :open="store.saveModalOpen"
    :title="t('save.title')"
    :description="t('save.description')"
    size="lg"
    :show-close="!saving"
    :close-on-backdrop="!saving"
    :close-on-escape="!saving"
    @close="!saving && store.closeSaveChanges()"
  >
    <form id="save-changes-form" class="space-y-4" @submit.prevent="save">
      <div
        v-if="loading"
        class="flex items-center gap-2 rounded-lg border border-border bg-surface px-4 py-3 text-xs text-fg-muted"
      >
        <TdSpinner size="sm" :label="t('save.reading')" />
        {{ t('save.readingEllipsis') }}
      </div>

      <div v-for="repo in repos" :key="repo.id" class="rounded-lg border border-border bg-surface">
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div class="min-w-0">
            <p class="truncate text-sm font-medium text-fg">{{ repo.name }}</p>
            <p class="truncate text-xs text-fg-muted">
              {{ t('save.workingOn', { branch: repo.state.currentBranch }) }}
            </p>
          </div>
          <TdBadge variant="neutral">{{ t('save.changed', { count: changes[repo.path]?.length ?? 0 }) }}</TdBadge>
        </div>

        <div v-if="(changes[repo.path]?.length ?? 0) > 0" class="border-t border-border">
          <div class="max-h-40 overflow-auto px-4 py-2">
            <div
              v-for="change in changes[repo.path]"
              :key="`${change.status}:${change.path}`"
              class="flex items-center gap-2 py-0.5 font-mono text-xs"
            >
              <FileText class="h-3 w-3 shrink-0 text-fg-dim" />
              <span class="min-w-0 flex-1 truncate text-fg-muted" :title="change.path">
                {{ change.path }}
              </span>
              <TdBadge :variant="statusVariant(change.status)" size="sm">
                {{ statusLabel(change.status) }}
              </TdBadge>
            </div>
          </div>
        </div>
      </div>

      <TdFormField
        :label="t('save.messageLabel')"
        :hint="t('save.messageHint')"
        required
      >
        <template #default="{ id, required }">
          <TdTextarea
            :id="id"
            v-model="message"
            :required="required"
            :rows="3"
            autofocus
            class="resize-none"
            :placeholder="t('save.messagePlaceholder')"
          />
        </template>
      </TdFormField>

      <TdAlert v-if="error" variant="error">
        <pre class="whitespace-pre-wrap font-mono text-xs">{{ error }}</pre>
      </TdAlert>
    </form>

    <template #footer>
      <div class="flex w-full items-center justify-between">
        <p class="text-xs text-fg-muted">
          {{
            t('save.total', {
              files: totalFiles,
              fileWord: tPlural('common.file', totalFiles),
              projects: repos.length,
              projectWord: tPlural('common.project', repos.length),
            })
          }}
        </p>
        <div class="flex items-center gap-2">
          <TdButton variant="ghost" :disabled="saving" @click="store.closeSaveChanges">
            {{ t('save.cancel') }}
          </TdButton>
          <TdButton
            type="submit"
            form="save-changes-form"
            variant="primary"
            :loading="saving"
            :disabled="!canSave"
          >
            <template #icon-left><Save /></template>
            {{ t('save.submit') }}
          </TdButton>
        </div>
      </div>
    </template>
  </TdModal>
</template>
