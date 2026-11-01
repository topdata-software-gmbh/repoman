<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { FileText, RefreshCw, Save, Sparkles } from 'lucide-vue-next';
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
import type { CommitGenRowStatus, FileChange, FileChangeStatus, Repository } from '../types/domain';

const store = useRepoStore();

const repos = ref<Repository[]>([]);
const changes = ref<Record<string, FileChange[]>>({});
const loading = ref(false);
const saving = ref(false);
const generating = ref(false);
const error = ref('');

const rows = computed(() => store.commitgen.list);

const totalFiles = computed(() =>
  repos.value.reduce((sum, repo) => sum + (changes.value[repo.path]?.length ?? 0), 0),
);

const canGenerate = computed(
  () =>
    store.commitgen.configured &&
    repos.value.length > 0 &&
    !loading.value &&
    !saving.value &&
    !generating.value &&
    !store.commitgen.generating,
);

const canSave = computed(
  () => store.commitgen.canCommit && repos.value.length > 0 && !loading.value && !saving.value,
);

watch(
  () => store.saveModalOpen,
  (open) => {
    if (!open) return;

    repos.value = store.dirtyRepositories.filter(
      (repo) => !store.saveModalPaths || store.saveModalPaths.includes(repo.path),
    );
    store.ensureCommitGenRows(repos.value.map((repo) => repo.path));
    changes.value = {};
    error.value = '';
    saving.value = false;
    generating.value = false;
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

function rowStatusLabel(status: CommitGenRowStatus): string {
  switch (status) {
    case 'generating':
      return t('save.ai.status.generating');
    case 'ready':
      return t('save.ai.status.ready');
    case 'error':
      return t('save.ai.status.error');
    default:
      return t('save.ai.status.idle');
  }
}

function rowVariant(status: CommitGenRowStatus): ChangeVariant {
  switch (status) {
    case 'generating':
      return 'info';
    case 'ready':
      return 'success';
    case 'error':
      return 'error';
    default:
      return 'neutral';
  }
}

function branchFor(path: string): string {
  return repos.value.find((repo) => repo.path === path)?.state.currentBranch ?? '';
}

async function generate(): Promise<void> {
  if (!canGenerate.value) return;

  generating.value = true;
  error.value = '';
  try {
    await store.startRun(repos.value.map((repo) => repo.path));
  } catch (failure) {
    error.value = String(failure);
  } finally {
    generating.value = false;
  }
}

function retry(path: string): void {
  void store.retryRepo(path);
}

async function save(): Promise<void> {
  if (!canSave.value) return;

  saving.value = true;
  error.value = '';
  const messages = Object.fromEntries(rows.value.map((row) => [row.path, row.message.trim()]));
  const results = await store.commitRepositories(messages);
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
  store.ensureCommitGenRows(repos.value.map((repo) => repo.path));
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

      <div class="flex items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="text-sm font-medium text-fg">{{ t('save.ai.draftLabel') }}</p>
          <p class="text-xs text-fg-muted">
            {{ store.commitgen.configured ? t('save.ai.draftHint') : t('save.ai.notConfigured') }}
          </p>
        </div>
        <TdButton
          variant="secondary"
          :disabled="!canGenerate"
          :loading="generating || store.commitgen.generating"
          @click="generate"
        >
          <template #icon-left><Sparkles /></template>
          {{ store.commitgen.generating ? t('save.ai.generating') : t('save.ai.generate') }}
        </TdButton>
      </div>

      <div v-for="(row, index) in rows" :key="row.path" class="rounded-lg border border-border bg-surface">
        <div class="flex items-center justify-between gap-3 px-4 py-3">
          <div class="min-w-0">
            <p class="truncate text-sm font-medium text-fg">{{ row.name }}</p>
            <p class="truncate text-xs text-fg-muted">
              {{ t('save.workingOn', { branch: branchFor(row.path) }) }}
            </p>
          </div>
          <div class="flex shrink-0 items-center gap-2">
            <TdBadge :variant="rowVariant(row.status)" size="sm">
              {{ rowStatusLabel(row.status) }}
            </TdBadge>
            <TdButton
              v-if="row.status === 'error'"
              variant="ghost"
              size="sm"
              :title="t('save.ai.retry')"
              @click="retry(row.path)"
            >
              <template #icon-left><RefreshCw /></template>
            </TdButton>
          </div>
        </div>

        <div class="border-t border-border px-4 py-3">
          <TdFormField
            :label="t('save.ai.messageLabel', { name: row.name })"
            :hint="t('save.ai.messageHint')"
            required
          >
            <template #default="{ id, required }">
              <TdTextarea
                :id="id"
                :model-value="row.message"
                :required="required"
                :rows="3"
                :autofocus="index === 0"
                class="resize-none font-mono text-xs"
                :placeholder="t('save.messagePlaceholder')"
                @update:model-value="(value: string) => store.setCommitGenMessage(row.path, value)"
              />
            </template>
          </TdFormField>
          <p v-if="row.error" class="mt-2 text-xs text-error">{{ row.error }}</p>
        </div>

        <div v-if="(changes[row.path]?.length ?? 0) > 0" class="border-t border-border">
          <div class="max-h-40 overflow-auto px-4 py-2">
            <div
              v-for="change in changes[row.path]"
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

      <p v-if="rows.length === 0" class="text-xs text-fg-muted">
        {{ t('save.ai.manualHint') }}
      </p>

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
