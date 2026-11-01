<script setup lang="ts">
import { computed } from 'vue';
import {
  CheckCircle2,
  Eye,
  EyeOff,
  FolderGit2,
  FolderSearch,
  HelpCircle,
  Layers,
  MoreHorizontal,
  Moon,
  Pencil,
  Plus,
  RefreshCw,
  Sparkles,
  Sun,
  X,
} from 'lucide-vue-next';
import {
  TdButton,
  TdLanguageSwitcher,
  TdMenu,
  TdMenuItem,
  TdSpinner,
  useTheme,
  type TdLanguageOption,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { groupColorStyle } from '../lib/groupDefaults';
import { language, setLanguage, t } from '../lib/i18n';
import { useRepoStore } from '../stores/repoStore';
import type { RepoGroup } from '../types/domain';

const store = useRepoStore();
const { mode, toggle: toggleTheme } = useTheme();

const languageOptions: TdLanguageOption[] = [
  { value: 'de', label: 'Deutsch', short: 'DE' },
  { value: 'en', label: 'English', short: 'EN' },
];

function chooseLanguage(value: string): void {
  if (value === 'de' || value === 'en') setLanguage(value);
}

function repoCount(group: RepoGroup): number {
  return store.repositories.filter((repo) => group.repoPaths.includes(repo.path)).length;
}

async function removeScanPath(id: string, path: string): Promise<void> {
  if (!window.confirm(t('sidebar.confirmRemoveFolder', { path }))) {
    return;
  }
  await store.removeScanPath(id);
}

// FR-003/FR-015: while scanning (or for the ~10 s summary window) the chip is
// a button into the live activity modal; otherwise it starts a refresh.
const scanChipActive = computed(() => store.isScanning || store.scanActivity.summaryVisible);

function onScanChipClick(): void {
  if (scanChipActive.value) {
    store.openScanModal();
    return;
  }
  void store.scanRepositories();
}

function scanChipLabel(): string {
  if (store.isScanning) return t('sidebar.looking');
  if (store.scanActivity.summaryVisible) {
    return t('scan.activity.chipDone', { count: store.scanActivityCount });
  }
  return t('sidebar.lookAgain');
}
</script>

<template>
  <aside class="flex w-64 shrink-0 flex-col border-r border-border bg-canvas">
    <div class="flex-1 overflow-y-auto px-3 py-4">
      <div class="mb-5 flex items-center gap-2.5 px-2">
        <div
          class="flex h-8 w-8 items-center justify-center rounded-[10px] bg-primary text-on-primary shadow-sm"
        >
          <FolderGit2 class="h-4 w-4" />
        </div>
        <p class="truncate text-[13px] font-semibold text-fg">RepoMan</p>
      </div>

      <nav class="sidebar-nav space-y-0.5">
        <TdButton
          variant="ghost"
          block
          class="justify-between px-2.5 py-2 text-[13px]"
          :class="
            store.selectedGroupId === 'all'
              ? 'bg-surface font-medium text-fg shadow-sm'
              : 'text-fg-muted hover:text-fg'
          "
          @click="store.selectGroup('all')"
        >
          <template #icon-left><Layers class="h-4 w-4" /></template>
          <span class="flex w-full items-center justify-between">
            <span>{{ t('sidebar.allProjects') }}</span>
            <span class="text-[11px] text-fg-dim">{{ store.repositories.length }}</span>
          </span>
        </TdButton>

        <div class="flex items-center justify-between px-2 pb-1 pt-5">
          <span class="text-[11px] font-medium text-fg-muted">{{ t('sidebar.groups') }}</span>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5 text-fg-dim"
            :title="t('sidebar.newGroup')"
            :aria-label="t('sidebar.newGroup')"
            @click="store.openGroupEditor(null)"
          >
            <template #icon-left><Plus class="h-3.5 w-3.5" /></template>
          </TdButton>
        </div>

        <p
          v-if="store.config.groups.length === 0"
          class="px-2.5 pb-1 text-[11px] leading-relaxed text-fg-dim"
        >
          {{ t('sidebar.createGroupHint') }}
        </p>

        <div
          v-for="group in store.config.groups"
          :key="group.id"
          class="group flex items-center gap-1 rounded-lg pr-1 transition-colors"
          :class="
            store.selectedGroupId === group.id
              ? 'bg-surface shadow-sm'
              : 'hover:bg-surface/60'
          "
        >
          <TdButton
            variant="ghost"
            block
            class="min-w-0 flex-1 justify-between px-2.5 py-2 text-[13px]"
            :class="
              store.selectedGroupId === group.id
                ? 'font-medium text-fg'
                : 'text-fg-muted hover:text-fg'
            "
            @click="store.selectGroup(group.id)"
          >
            <span class="flex w-full items-center justify-between gap-2">
              <span class="flex min-w-0 items-center gap-2">
                <span
                  class="h-2 w-2 shrink-0 rounded-full"
                  :style="groupColorStyle(group.color)"
                ></span>
                <span class="truncate">{{ group.name }}</span>
              </span>
              <span class="text-[11px] text-fg-dim">{{ repoCount(group) }}</span>
            </span>
          </TdButton>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-1 text-fg-dim opacity-0 transition-opacity group-hover:opacity-100"
            :title="t('sidebar.editGroup')"
            :aria-label="t('sidebar.editGroup')"
            @click.stop="store.openGroupEditor(group)"
          >
            <template #icon-left><Pencil class="h-3.5 w-3.5" /></template>
          </TdButton>
        </div>

        <div class="flex items-center justify-between px-2 pb-1 pt-5">
          <span class="text-[11px] font-medium text-fg-muted">{{ t('sidebar.folders') }}</span>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5 text-fg-dim"
            :title="t('sidebar.addFolder')"
            :aria-label="t('sidebar.addFolder')"
            @click="store.openAddScanPath"
          >
            <template #icon-left><Plus class="h-3.5 w-3.5" /></template>
          </TdButton>
        </div>

        <p
          v-if="store.config.scanPaths.length === 0"
          class="px-2.5 pb-1 text-[11px] leading-relaxed text-fg-dim"
        >
          {{ t('sidebar.addFolderHint') }}
        </p>

        <div
          v-for="scanPath in store.config.scanPaths"
          :key="scanPath.id"
          class="group flex items-center gap-1.5 rounded-lg px-2 py-1.5 transition-colors hover:bg-surface"
        >
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5"
            :class="scanPath.enabled ? 'text-fg-muted' : 'text-fg-dim'"
            :title="scanPath.enabled ? t('sidebar.stopLooking') : t('sidebar.startLooking')"
            :aria-label="scanPath.enabled ? t('sidebar.stopLooking') : t('sidebar.startLooking')"
            @click="store.toggleScanPath(scanPath)"
          >
            <template #icon-left>
              <Eye v-if="scanPath.enabled" class="h-3.5 w-3.5" />
              <EyeOff v-else class="h-3.5 w-3.5" />
            </template>
          </TdButton>
          <span
            class="min-w-0 flex-1 truncate text-[11px]"
            :class="scanPath.enabled ? 'text-fg-muted' : 'text-fg-dim line-through'"
            :title="scanPath.path"
          >
            {{ scanPath.path }}
          </span>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5 text-fg-dim opacity-0 transition-opacity hover:text-error-fg group-hover:opacity-100"
            :title="t('sidebar.removeFolder')"
            :aria-label="t('sidebar.removeFolder')"
            @click="removeScanPath(scanPath.id, scanPath.path)"
          >
            <template #icon-left><X class="h-3.5 w-3.5" /></template>
          </TdButton>
        </div>
      </nav>
    </div>

    <div class="space-y-2 border-t border-border p-3">
      <TdButton variant="primary" block size="sm" @click="store.openAddScanPath">
        <template #icon-left><FolderSearch class="h-3.5 w-3.5" /></template>
        <span>{{ t('sidebar.addFolder') }}</span>
      </TdButton>

      <div class="flex items-center gap-1">
        <TdButton
          variant="ghost"
          size="sm"
          class="flex-1 justify-start text-[11px] text-fg-muted"
          :title="scanChipActive ? t('scan.activity.tooltip') : undefined"
          @click="onScanChipClick"
        >
          <template #icon-left>
            <TdSpinner v-if="store.isScanning" size="sm" />
            <CheckCircle2
              v-else-if="store.scanActivity.summaryVisible"
              class="h-3.5 w-3.5 text-success-fg"
            />
            <RefreshCw v-else class="h-3.5 w-3.5" />
          </template>
          <span>{{ scanChipLabel() }}</span>
        </TdButton>

        <TdLanguageSwitcher
          :model-value="language"
          :languages="languageOptions"
          :aria-label="t('sidebar.changeLanguage')"
          @update:model-value="chooseLanguage"
        />

        <TdButton
          variant="ghost"
          size="sm"
          class="p-2 text-fg-muted"
          :title="mode === 'dark' ? t('sidebar.switchToLight') : t('sidebar.switchToDark')"
          :aria-label="mode === 'dark' ? t('sidebar.switchToLight') : t('sidebar.switchToDark')"
          @click="toggleTheme"
        >
          <template #icon-left>
            <Sun v-if="mode === 'dark'" class="h-3.5 w-3.5" />
            <Moon v-else class="h-3.5 w-3.5" />
          </template>
        </TdButton>

        <TdMenu align="end" :aria-label="t('sidebar.moreOptions')">
          <template #trigger="{ open }">
            <TdButton
              variant="ghost"
              size="sm"
              class="p-2 text-fg-muted"
              :aria-label="t('sidebar.moreOptions')"
              aria-haspopup="menu"
              :aria-expanded="open"
            >
              <template #icon-left><MoreHorizontal class="h-3.5 w-3.5" /></template>
            </TdButton>
          </template>
          <TdMenuItem @select="store.openOnboarding(true)">
            <template #icon><Sparkles class="h-3.5 w-3.5" /></template>
            {{ t('sidebar.runSetupAgain') }}
          </TdMenuItem>
          <TdMenuItem @select="store.openHelp">
            <template #icon><HelpCircle class="h-3.5 w-3.5" /></template>
            {{ t('sidebar.howThisWorks') }}
          </TdMenuItem>
        </TdMenu>
      </div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar-nav :deep(.td-button__label) {
  flex: 1 1 auto;
  min-width: 0;
}
</style>
