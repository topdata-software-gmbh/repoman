<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { TdButton } from '@topdata-software-gmbh/ui-kit-web-vue3';
import { t } from '../../lib/i18n';
import { highlightLine } from '../../lib/highlight';
import {
  displayModeLabelKey,
  foldLinesLabel,
  languageForPath,
  rawPresentation,
  toUnifiedRows,
  truncationLabelKey,
} from '../../lib/review';
import type { DiffLine, DiffRow, DisplayMode, TextPreview } from '../../types/domain';

const props = withDefaults(
  defineProps<{
    preview: TextPreview;
    path: string;
    displayMode?: DisplayMode;
  }>(),
  { displayMode: 'sideBySide' },
);

const emit = defineEmits<{ 'update:displayMode': [mode: DisplayMode] }>();

const expanded = ref<Set<string>>(new Set());

watch(
  () => props.preview,
  () => {
    expanded.value = new Set();
  },
);

watch(
  () => props.path,
  () => {
    expanded.value = new Set();
  },
);

const language = computed(() =>
  props.preview.language && props.preview.language !== 'plaintext'
    ? props.preview.language
    : languageForPath(props.path),
);

const modes: DisplayMode[] = ['sideBySide', 'unified', 'raw'];
const raw = computed(() => rawPresentation(props.preview));
const shownMode = computed<DisplayMode>(() =>
  props.displayMode === 'raw' && raw.value === null ? 'unified' : props.displayMode,
);
const unified = computed(() => toUnifiedRows(props.preview.rows));
const truncationKey = computed(() => truncationLabelKey(props.preview.truncationReason));

function setMode(mode: DisplayMode): void {
  emit('update:displayMode', mode);
}

function foldKey(line: DiffLine): string {
  return line.kind === 'fold' ? `${line.oldStart}:${line.newStart}` : '';
}

function toggleFold(key: string): void {
  const next = new Set(expanded.value);
  if (next.has(key)) next.delete(key);
  else next.add(key);
  expanded.value = next;
}

function oldSide(row: DiffLine): 'same' | 'removed' | 'empty' {
  if (row.kind === 'same') return 'same';
  if (row.kind === 'removed' || row.kind === 'changed') return 'removed';
  return 'empty';
}

function newSide(row: DiffLine): 'same' | 'added' | 'empty' {
  if (row.kind === 'same') return 'same';
  if (row.kind === 'added' || row.kind === 'changed') return 'added';
  return 'empty';
}

function oldNumber(row: DiffLine): number | null {
  return row.kind === 'fold' ? null : (row.oldNumber ?? null);
}

function newNumber(row: DiffLine): number | null {
  return row.kind === 'fold' ? null : (row.newNumber ?? null);
}

function oldText(row: DiffLine): string | null {
  return row.kind === 'fold' ? null : (row.oldText ?? null);
}

function newText(row: DiffLine): string | null {
  return row.kind === 'fold' ? null : (row.newText ?? null);
}

function foldCount(line: DiffLine): number {
  return line.kind === 'fold' ? line.count : 0;
}

function foldRows(line: DiffLine): DiffRow[] {
  return line.kind === 'fold' ? line.rows : [];
}

function unifiedFoldKey(row: { oldNumber: number | null; newNumber: number | null }): string {
  return `${row.oldNumber ?? 0}:${row.newNumber ?? 0}`;
}

function cellClass(side: 'same' | 'removed' | 'added' | 'empty'): string {
  switch (side) {
    case 'removed':
      return 'bg-error-soft text-error-fg';
    case 'added':
      return 'bg-success-soft text-success-fg';
    case 'empty':
      return 'bg-surface-hover';
    default:
      return '';
  }
}

function marker(side: 'same' | 'removed' | 'added' | 'empty'): string {
  if (side === 'removed') return '−';
  if (side === 'added') return '+';
  return ' ';
}

function markerLabel(side: 'removed' | 'added'): string {
  return side === 'removed' ? t('review.diff.removed') : t('review.diff.added');
}

function highlighted(text: string | null | undefined): string {
  return highlightLine(text ?? '', language.value);
}
</script>

<template>
  <div class="flex min-h-0 flex-col">
    <div class="mb-2 flex flex-wrap items-center justify-between gap-2">
      <div class="flex items-center gap-1" role="group" :aria-label="t('review.display.label')">
        <TdButton
          v-for="mode in modes"
          :key="mode"
          size="sm"
          :variant="props.displayMode === mode ? 'secondary' : 'ghost'"
          :aria-pressed="props.displayMode === mode"
          @click="setMode(mode)"
        >
          {{ t(displayModeLabelKey(mode)) }}
        </TdButton>
      </div>
      <p v-if="shownMode === 'raw'" class="text-[11px] text-fg-muted">
        {{ t('review.display.rawWarning') }}
      </p>
    </div>

    <div class="min-h-0 flex-1 overflow-auto rounded-lg border border-border bg-surface">
      <div v-if="shownMode === 'raw' && raw" class="p-3">
        <pre class="whitespace-pre-wrap font-mono text-xs leading-5 text-fg" v-html="highlightLine(raw, 'plaintext')"></pre>
      </div>

      <div v-else-if="shownMode === 'sideBySide'" class="min-w-[36rem]">
        <div class="sticky top-0 z-10 grid grid-cols-2 border-b border-border bg-surface text-[11px] font-medium text-fg-muted">
          <span class="px-3 py-1.5">{{ t('review.diff.old') }}</span>
          <span class="border-l border-border px-3 py-1.5">{{ t('review.diff.new') }}</span>
        </div>

        <template v-for="(line, index) in props.preview.rows" :key="index">
          <div v-if="line.kind === 'fold'" class="border-y border-border bg-surface-hover">
            <button
              type="button"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[11px] text-fg-muted hover:bg-surface-raised"
              :aria-expanded="expanded.has(line.kind === 'fold' ? foldKey(line) : '')"
              @click="toggleFold(foldKey(line))"
            >
              <span class="font-mono">{{ expanded.has(line.kind === 'fold' ? foldKey(line) : '') ? '▾' : '▸' }}</span>
              <span>
                {{
                  expanded.has(line.kind === 'fold' ? foldKey(line) : '')
                    ? t('review.fold.hide', { lines: foldLinesLabel(foldCount(line)) })
                    : t('review.fold.show', { lines: foldLinesLabel(foldCount(line)) })
                }}
              </span>
            </button>
            <template v-if="expanded.has(line.kind === 'fold' ? foldKey(line) : '')">
              <div
                v-for="(inner, innerIndex) in foldRows(line)"
                :key="`fold-${index}-${innerIndex}`"
                class="grid grid-cols-2 font-mono text-xs leading-5"
              >
                <div class="flex min-w-0 gap-2 px-3">
                  <span class="w-8 shrink-0 select-none text-right text-fg-dim">
                    {{ oldNumber(inner) ?? '' }}
                  </span>
                  <span class="whitespace-pre-wrap break-all" v-html="highlighted(oldText(inner))"></span>
                </div>
                <div class="flex min-w-0 gap-2 border-l border-border px-3">
                  <span class="w-8 shrink-0 select-none text-right text-fg-dim">
                    {{ newNumber(inner) ?? '' }}
                  </span>
                  <span class="whitespace-pre-wrap break-all" v-html="highlighted(newText(inner))"></span>
                </div>
              </div>
            </template>
          </div>

          <div v-else class="grid grid-cols-2 font-mono text-xs leading-5">
            <div class="flex min-w-0 gap-2 px-3" :class="cellClass(oldSide(line))">
              <span class="w-8 shrink-0 select-none text-right text-fg-dim">
                {{ oldNumber(line) ?? '' }}
              </span>
              <span class="w-3 shrink-0 select-none" aria-hidden="true">{{ marker(oldSide(line)) }}</span>
              <span v-if="oldSide(line) === 'removed'" class="sr-only">{{ markerLabel('removed') }}</span>
              <span class="whitespace-pre-wrap break-all" v-html="highlighted(oldText(line))"></span>
            </div>
            <div class="flex min-w-0 gap-2 border-l border-border px-3" :class="cellClass(newSide(line))">
              <span class="w-8 shrink-0 select-none text-right text-fg-dim">
                {{ newNumber(line) ?? '' }}
              </span>
              <span class="w-3 shrink-0 select-none" aria-hidden="true">{{ marker(newSide(line)) }}</span>
              <span v-if="newSide(line) === 'added'" class="sr-only">{{ markerLabel('added') }}</span>
              <span class="whitespace-pre-wrap break-all" v-html="highlighted(newText(line))"></span>
            </div>
          </div>
        </template>
      </div>

      <div v-else class="min-w-[24rem] font-mono text-xs leading-5">
        <template v-for="(row, index) in unified" :key="index">
          <div v-if="row.kind === 'fold'" class="border-y border-border bg-surface-hover">
            <button
              type="button"
              class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-[11px] text-fg-muted hover:bg-surface-raised"
              :aria-expanded="expanded.has(unifiedFoldKey(row))"
              @click="toggleFold(unifiedFoldKey(row))"
            >
              <span class="font-mono">{{ expanded.has(unifiedFoldKey(row)) ? '▾' : '▸' }}</span>
              <span>
                {{
                  expanded.has(unifiedFoldKey(row))
                    ? t('review.fold.hide', { lines: foldLinesLabel(row.fold?.count ?? 0) })
                    : t('review.fold.show', { lines: foldLinesLabel(row.fold?.count ?? 0) })
                }}
              </span>
            </button>
            <template v-if="expanded.has(unifiedFoldKey(row))">
              <div
                v-for="(inner, innerIndex) in row.fold?.rows ?? []"
                :key="`ufold-${index}-${innerIndex}`"
                class="flex min-w-0 gap-2 px-3"
              >
                <span class="w-8 shrink-0 select-none text-right text-fg-dim">{{ oldNumber(inner) ?? '' }}</span>
                <span class="w-8 shrink-0 select-none text-right text-fg-dim">{{ newNumber(inner) ?? '' }}</span>
                <span class="w-3 shrink-0 select-none"> </span>
                <span class="whitespace-pre-wrap break-all" v-html="highlighted(oldText(inner))"></span>
              </div>
            </template>
          </div>
          <div
            v-else
            class="flex min-w-0 gap-2 px-3"
            :class="cellClass(row.kind === 'added' ? 'added' : row.kind === 'removed' ? 'removed' : 'same')"
          >
            <span class="w-8 shrink-0 select-none text-right text-fg-dim">{{ row.oldNumber ?? '' }}</span>
            <span class="w-8 shrink-0 select-none text-right text-fg-dim">{{ row.newNumber ?? '' }}</span>
            <span class="w-3 shrink-0 select-none" aria-hidden="true">{{ marker(row.kind === 'added' ? 'added' : row.kind === 'removed' ? 'removed' : 'same') }}</span>
            <span class="whitespace-pre-wrap break-all" v-html="highlighted(row.text)"></span>
          </div>
        </template>
      </div>
    </div>

    <p v-if="truncationKey" class="mt-2 rounded-lg border border-border bg-warning-soft px-3 py-2 text-xs text-warning-fg" role="status">
      {{ t(truncationKey, { count: props.preview.rows.length }) }}
    </p>
  </div>
</template>
