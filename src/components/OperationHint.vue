<script setup lang="ts">
import { getCurrentInstance, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { usePopover } from '@topdata-software-gmbh/ui-kit-web-vue3';

const props = defineProps<{ hint: string }>();

const instanceId = getCurrentInstance()?.uid ?? 0;
const descriptionId = `td-operation-hint-${instanceId}-description`;

const root = ref<HTMLElement | null>(null);
const panel = ref<HTMLElement | null>(null);
const visible = ref(false);

const popover = usePopover(root, panel);

function show(): void {
  visible.value = true;
}

function hide(): void {
  visible.value = false;
}

function onFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && root.value?.contains(next)) return;
  hide();
}

function onDocumentKeydown(event: KeyboardEvent): void {
  if (event.key === 'Escape') hide();
}

watch(visible, async (open) => {
  if (open) {
    document.addEventListener('keydown', onDocumentKeydown);
    await nextTick();
    popover.open();
  } else {
    document.removeEventListener('keydown', onDocumentKeydown);
    popover.close();
  }
});

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onDocumentKeydown);
  popover.close();
});
</script>

<template>
  <div
    ref="root"
    data-operation-hint
    class="relative inline-flex"
    @mouseenter="show"
    @mouseleave="hide"
    @focusin="show"
    @focusout="onFocusOut"
    @keydown.esc="hide"
  >
    <slot :described-by="descriptionId" :title="props.hint" />
    <span :id="descriptionId" class="sr-only">{{ props.hint }}</span>
  </div>

  <Teleport to="body">
    <div
      v-if="visible"
      ref="panel"
      role="tooltip"
      aria-hidden="true"
      class="pointer-events-none fixed z-[100] max-w-xs rounded-lg border border-border bg-surface px-2.5 py-1.5 text-[11px] leading-snug text-fg shadow-lg"
      style="position: fixed; left: 0; top: 0; visibility: hidden"
    >
      {{ props.hint }}
    </div>
  </Teleport>
</template>
