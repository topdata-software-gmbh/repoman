<script setup lang="ts">
import { computed } from 'vue';
import { ChevronLeft, ChevronRight, RefreshCw } from 'lucide-vue-next';
import { TdButton, TdModal, TdSpinner } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t, tPlural } from '../lib/i18n';
import { errorLabelKey, formatBytes, formatSavedAt, noticeLabelKey } from '../lib/review';
import { useRepoStore } from '../stores/repoStore';
import type { DisplayMode } from '../types/domain';
import ReviewFileList from './reviews/ReviewFileList.vue';
import ReviewImageDiff from './reviews/ReviewImageDiff.vue';
import ReviewNotice from './reviews/ReviewNotice.vue';
import ReviewTextDiff from './reviews/ReviewTextDiff.vue';

const store = useRepoStore();

const project = computed(() => store.reviewActiveProject);
const comparison = computed(() => store.reviewActiveComparison);
const preview = computed(() => store.reviewActivePreview);
const selectedPath = computed(() => store.reviewActiveSelectedFile);
const hasProjects = computed(() => store.reviewProjects.length > 0);
const position = computed(() =>
  t('review.position', {
    current: store.reviewActiveIndex + 1,
    total: store.reviewProjects.length,
  }),
);
const canPrevious = computed(() => store.reviewActiveIndex > 0);
const canNext = computed(() => store.reviewActiveIndex < store.reviewProjects.length - 1);
const displayMode = computed<DisplayMode>({
  get: () => store.reviewDisplayMode,
  set: (mode) => {
    store.reviewDisplayMode = mode;
  },
});

const loadingProject = computed(() => store.reviewLoadingProject && comparison.value === null);
const unreadable = computed(() => comparison.value?.state === 'unreadable');
const unavailable = computed(() => comparison.value?.state === 'unavailable');
const empty = computed(() => comparison.value?.state === 'empty');
const ready = computed(() => comparison.value?.state === 'ready');

const fileWord = computed(() =>
  tPlural('review.file', comparison.value?.summary.fileCount ?? 0),
);

function close(): void {
  store.closeReview();
}

function selectFile(path: string): void {
  store.selectReviewFile(path);
}

function previous(): void {
  store.prevReviewProject();
}

function next(): void {
  store.nextReviewProject();
}

function refresh(): void {
  store.refreshReviewProject();
}
</script>

<template>
  <TdModal
    :open="store.reviewOpen"
    :title="t('review.title')"
    size="full"
    @close="close"
  >
    <div class="flex h-full min-h-[24rem] flex-col gap-4">
      <div v-if="hasProjects" class="flex flex-wrap items-start justify-between gap-3">
        <div class="min-w-0">
          <p class="truncate text-base font-semibold text-fg">{{ project?.name }}</p>
          <p v-if="comparison?.saved" class="truncate text-xs text-fg-muted">
            {{ t('review.saved.message', { message: comparison.saved.message }) }}
            · {{ t('review.saved.when', { date: formatSavedAt(comparison.saved.timestamp) }) }}
          </p>
          <p v-if="comparison && comparison.state === 'ready'" class="mt-1 text-xs text-fg-muted">
            {{ comparison.summary.fileCount }} {{ fileWord }}
            · <span class="text-success-fg">+{{ comparison.summary.addedLines }}</span>
            · <span class="text-error-fg">−{{ comparison.summary.removedLines }}</span>
          </p>
        </div>

        <div v-if="store.reviewProjects.length > 1" class="flex items-center gap-2">
          <TdButton
            variant="ghost"
            size="sm"
            :disabled="!canPrevious"
            :aria-label="t('review.previous')"
            @click="previous"
          >
            <template #icon-left><ChevronLeft /></template>
          </TdButton>
          <p class="text-xs text-fg-muted" role="status" aria-live="polite">{{ position }}</p>
          <TdButton
            variant="ghost"
            size="sm"
            :disabled="!canNext"
            :aria-label="t('review.next')"
            @click="next"
          >
            <template #icon-right><ChevronRight /></template>
          </TdButton>
        </div>
      </div>

      <div
        v-if="store.reviewIsStale"
        class="flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border bg-warning-soft px-3 py-2"
        role="status"
      >
        <div class="min-w-0">
          <p class="text-xs font-medium text-warning-fg">{{ t('review.stale') }}</p>
          <p class="text-[11px] text-warning-fg">{{ t('review.stale.text') }}</p>
        </div>
        <TdButton variant="secondary" size="sm" @click="refresh">
          <template #icon-left><RefreshCw /></template>
          {{ t('review.stale.action') }}
        </TdButton>
      </div>

      <div
        v-if="loadingProject"
        class="flex flex-1 items-center justify-center gap-2 text-xs text-fg-muted"
        role="status"
      >
        <TdSpinner size="sm" :label="t('review.loading')" />
        {{ t('review.loading') }}
      </div>

      <ReviewNotice
        v-else-if="unavailable"
        :title="
          comparison?.error?.code === 'head-not-found'
            ? t('review.headNotFound.title')
            : t('review.unavailable.title')
        "
        :message="
          comparison?.error?.code === 'head-not-found'
            ? t('review.headNotFound.text')
            : t('review.unavailable.text')
        "
        tone="warning"
      />

      <ReviewNotice
        v-else-if="unreadable"
        :title="t('review.unreadable.title')"
        :message="t(errorLabelKey(comparison?.error?.code))"
        tone="error"
      />

      <ReviewNotice
        v-else-if="empty"
        :title="t('review.empty.title')"
        :message="t('review.empty.text')"
      />

      <div v-else-if="ready" class="grid min-h-0 flex-1 grid-cols-[minmax(14rem,22rem)_1fr] gap-4">
        <ReviewFileList
          :files="store.reviewFileEntries"
          :selected-path="selectedPath"
          @select="selectFile"
        />

        <section class="flex min-h-0 min-w-0 flex-col" :aria-label="t('review.preview')">
          <div
            v-if="store.reviewLoadingFile"
            class="flex flex-1 items-center justify-center gap-2 text-xs text-fg-muted"
            role="status"
          >
            <TdSpinner size="sm" :label="t('review.loadingFile')" />
            {{ t('review.loadingFile') }}
          </div>

          <ReviewNotice
            v-else-if="!selectedPath || !preview"
            :title="t('review.noSelection')"
          />

          <template v-else>
            <p class="mb-2 truncate font-mono text-xs text-fg-muted" :title="preview.path">
              {{ preview.path }}
            </p>

            <ReviewImageDiff v-if="preview.image" :preview="preview" />

            <div
              v-else-if="preview.binary"
              class="rounded-lg border border-border bg-surface px-4 py-6 text-center"
            >
              <p class="text-sm font-medium text-fg">{{ t('review.binary.changed') }}</p>
              <p class="mt-1 text-xs text-fg-muted">
                {{
                  t('review.binary.size', {
                    before: formatBytes(preview.binary.oldBytes),
                    after: formatBytes(preview.binary.newBytes),
                  })
                }}
              </p>
            </div>

            <ReviewNotice
              v-else-if="preview.text && preview.notice"
              :title="t(noticeLabelKey(preview.notice) ?? 'review.notice.missing')"
              tone="warning"
            />

            <ReviewTextDiff
              v-else-if="preview.text"
              :preview="preview.text"
              :path="preview.path"
              :display-mode="displayMode"
              @update:display-mode="displayMode = $event"
            />
          </template>
        </section>
      </div>
    </div>

    <template #footer>
      <TdButton variant="primary" @click="close">{{ t('review.close') }}</TdButton>
    </template>
  </TdModal>
</template>
