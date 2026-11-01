<script setup lang="ts">
import { computed } from 'vue';
import {
  TdButton,
  TdIcon,
  TdLanguageSwitcher,
  TdMenu,
  TdMenuItem,
  TdSidebar,
  TdSidebarBody,
  TdSidebarFooter,
  TdSidebarHeader,
  TdSidebarItem,
  TdSidebarSection,
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
  <TdSidebar tone="canvas" class="w-64 shrink-0">
    <TdSidebarHeader>
      <div class="flex items-center gap-2.5">
        <div
          class="flex h-8 w-8 items-center justify-center rounded-[10px] bg-primary text-on-primary shadow-sm"
        >
          <TdIcon name="folder-git" />
        </div>
        <p class="truncate text-[13px] font-semibold text-fg">RepoMan</p>
      </div>
    </TdSidebarHeader>

    <TdSidebarBody as="nav">
      <TdSidebarSection>
        <TdSidebarItem
          icon="layers"
          :active="store.selectedGroupId === 'all'"
          :aria-current="store.selectedGroupId === 'all' ? 'page' : undefined"
          @click="store.selectGroup('all')"
        >
          {{ t('sidebar.allProjects') }}
          <template #trailing>
            <span class="text-[11px] text-fg-dim">{{ store.repositories.length }}</span>
          </template>
        </TdSidebarItem>
      </TdSidebarSection>

      <TdSidebarSection :label="t('sidebar.groups')">
        <template #action>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5 text-fg-dim"
            :title="t('sidebar.newGroup')"
            :aria-label="t('sidebar.newGroup')"
            @click="store.openGroupEditor(null)"
          >
            <template #icon-left><TdIcon name="plus" /></template>
          </TdButton>
        </template>

        <p
          v-if="store.config.groups.length === 0"
          class="px-2.5 pb-1 text-[11px] leading-relaxed text-fg-dim"
        >
          {{ t('sidebar.createGroupHint') }}
        </p>

        <div v-for="group in store.config.groups" :key="group.id" class="group">
          <TdSidebarItem
            :active="store.selectedGroupId === group.id"
            :aria-current="store.selectedGroupId === group.id ? 'page' : undefined"
            @click="store.selectGroup(group.id)"
          >
            <template #icon>
              <span
                class="h-2 w-2 shrink-0 rounded-full"
                :style="groupColorStyle(group.color)"
              ></span>
            </template>
            <span class="truncate">{{ group.name }}</span>
            <template #trailing>
              <span class="text-[11px] text-fg-dim">{{ repoCount(group) }}</span>
            </template>
            <template #actions>
              <TdButton
                variant="ghost"
                size="sm"
                class="p-1 text-fg-dim opacity-0 transition-opacity group-hover:opacity-100"
                :title="t('sidebar.editGroup')"
                :aria-label="t('sidebar.editGroup')"
                @click.stop="store.openGroupEditor(group)"
              >
                <template #icon-left><TdIcon name="pencil" /></template>
              </TdButton>
            </template>
          </TdSidebarItem>
        </div>
      </TdSidebarSection>

      <TdSidebarSection :label="t('sidebar.folders')">
        <template #action>
          <TdButton
            variant="ghost"
            size="sm"
            class="p-0.5 text-fg-dim"
            :title="t('sidebar.addFolder')"
            :aria-label="t('sidebar.addFolder')"
            @click="store.openAddScanPath"
          >
            <template #icon-left><TdIcon name="plus" /></template>
          </TdButton>
        </template>

        <p
          v-if="store.config.scanPaths.length === 0"
          class="px-2.5 pb-1 text-[11px] leading-relaxed text-fg-dim"
        >
          {{ t('sidebar.addFolderHint') }}
        </p>

        <div v-for="scanPath in store.config.scanPaths" :key="scanPath.id" class="group">
          <TdSidebarItem as="div">
            <template #leading>
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
                  <TdIcon :name="scanPath.enabled ? 'eye' : 'eye-off'" />
                </template>
              </TdButton>
            </template>

            <span
              class="min-w-0 flex-1 truncate text-[11px]"
              :class="scanPath.enabled ? 'text-fg-muted' : 'text-fg-dim line-through'"
              :title="scanPath.path"
            >
              {{ scanPath.path }}
            </span>

            <template #actions>
              <TdButton
                variant="ghost"
                size="sm"
                class="p-0.5 text-fg-dim opacity-0 transition-opacity hover:text-error-fg group-hover:opacity-100"
                :title="t('sidebar.removeFolder')"
                :aria-label="t('sidebar.removeFolder')"
                @click="removeScanPath(scanPath.id, scanPath.path)"
              >
                <template #icon-left><TdIcon name="x" /></template>
              </TdButton>
            </template>
          </TdSidebarItem>
        </div>
      </TdSidebarSection>
    </TdSidebarBody>

    <TdSidebarFooter class="space-y-2">
      <TdButton variant="primary" block size="sm" @click="store.openAddScanPath">
        <template #icon-left><TdIcon name="folder-search" /></template>
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
            <TdIcon
              v-else-if="store.scanActivity.summaryVisible"
              name="check-circle"
              class="text-success-fg"
            />
            <TdIcon v-else name="refresh" />
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
            <TdIcon :name="mode === 'dark' ? 'sun' : 'moon'" />
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
              <template #icon-left><TdIcon name="more-horizontal" /></template>
            </TdButton>
          </template>
          <TdMenuItem @select="store.openOnboarding(true)">
            <template #icon><TdIcon name="sparkles" /></template>
            {{ t('sidebar.runSetupAgain') }}
          </TdMenuItem>
          <TdMenuItem @select="store.openHelp">
            <template #icon><TdIcon name="help-circle" /></template>
            {{ t('sidebar.howThisWorks') }}
          </TdMenuItem>
        </TdMenu>
      </div>
    </TdSidebarFooter>
  </TdSidebar>
</template>
