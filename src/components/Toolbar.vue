<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import {
  Archive,
  ArchiveRestore,
  ArrowDownToLine,
  FolderSearch,
  GitBranch,
  HelpCircle,
  MoreHorizontal,
  RefreshCw,
  Save,
  Terminal,
  Upload,
} from 'lucide-vue-next';
import {
  TdBadge,
  TdButton,
  TdMenu,
  TdMenuItem,
  TdSelect,
  TdSpinner,
} from '@topdata-software-gmbh/ui-kit-web-vue3';
import { useRepoStore } from '../stores/repoStore';

const store = useRepoStore();
const selectedBranch = ref('');
const switching = ref(false);
const activeAction = ref<string | null>(null);

const busy = computed(() => store.isLoading);
const canSwitchBranch = computed(
  () => !busy.value && !switching.value && store.groupBranches.length > 0,
);
const showBranchSelector = computed(
  () => !store.currentGroup || store.currentGroup.settings.syncBranching,
);
const dirtyCount = computed(() => store.dirtyRepositories.length);
const branchSelection = computed({
  get: () => selectedBranch.value || store.synchronizedBranch || '',
  set: (value: string) => {
    selectedBranch.value = value;
  },
});

const branchOptions = computed(() => [
  {
    value: '',
    label:
      store.filteredRepositories.length === 0 ? 'No projects yet' : 'Choose a branch…',
    disabled: true,
  },
  ...store.groupBranches.map((branch) => ({
    value: branch.name,
    label: `${branch.name} (${branch.repoCount}/${store.filteredRepositories.length})`,
  })),
]);

watch(
  () => store.selectedGroupId,
  () => {
    selectedBranch.value = '';
  },
);

watch(
  () => store.groupBranches,
  (branches) => {
    if (selectedBranch.value && !branches.some((branch) => branch.name === selectedBranch.value)) {
      selectedBranch.value = '';
    }
  },
);

async function onBranchChange(): Promise<void> {
  const branch = selectedBranch.value;
  if (!branch || switching.value) return;

  switching.value = true;
  selectedBranch.value = '';
  try {
    const autoStash = store.currentGroup?.settings.autoStashOnSwitch ?? true;
    await store.switchGroupBranch(branch, autoStash);
  } finally {
    switching.value = false;
  }
}

async function runAction(
  action: 'fetch' | 'pull' | 'push' | 'stash' | 'stash_pop',
): Promise<void> {
  if (activeAction.value || busy.value) return;
  activeAction.value = action;
  try {
    await store.executeGroupAction(action);
  } finally {
    activeAction.value = null;
  }
}
</script>

<template>
  <header
    class="flex min-h-14 shrink-0 select-none flex-wrap items-center justify-between gap-3 border-b border-border bg-surface px-4 py-2"
  >
    <div class="flex min-w-0 flex-wrap items-center gap-3">
      <span class="max-w-48 truncate text-sm font-semibold text-fg">
        {{ store.currentGroup ? store.currentGroup.name : 'Project group' }}
      </span>
      <TdBadge variant="neutral">
        {{ store.filteredRepositories.length }}
        {{ store.filteredRepositories.length === 1 ? 'project' : 'projects' }}
      </TdBadge>
      <span
        v-if="store.cleanRepositoryCount > 0"
        class="hidden items-center gap-1 text-[11px] text-success-fg lg:flex"
        title="Projects with no changes waiting"
      >
        <span class="h-1.5 w-1.5 rounded-full bg-success"></span>
        {{ store.cleanRepositoryCount }} up to date
      </span>

      <div
        v-if="showBranchSelector"
        class="flex items-center gap-2 rounded-lg border border-border bg-surface-raised px-2.5 py-1.5"
        title="Choose which version of the project you want to work on"
      >
        <TdSpinner
          v-if="switching || store.isBranchListLoading"
          size="sm"
          label="Loading branches"
          class="text-primary"
        />
        <GitBranch v-else class="h-3.5 w-3.5 text-primary" />
        <span class="text-[11px] text-fg-muted">Branch</span>
        <TdSelect
          v-model="branchSelection"
          :options="branchOptions"
          :disabled="!canSwitchBranch"
          aria-label="Working on branch"
          size="sm"
          class="max-w-44"
          @change="onBranchChange"
        />
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-1.5">
      <TdButton
        variant="primary"
        size="sm"
        :disabled="busy || dirtyCount === 0"
        title="Create a checkpoint of your changed files in Git"
        @click="store.openSaveChanges()"
      >
        <template #icon-left><Save /></template>
        <span>Save to Git</span>
        <TdBadge v-if="dirtyCount > 0" variant="neutral">{{ dirtyCount }}</TdBadge>
      </TdButton>

      <TdButton
        variant="secondary"
        size="sm"
        :disabled="busy || store.filteredRepositories.length === 0"
        title="Download new work from the server and update your project folders"
        @click="runAction('pull')"
      >
        <template #icon-left>
          <TdSpinner v-if="activeAction === 'pull'" size="sm" />
          <ArrowDownToLine v-else class="text-success-fg" />
        </template>
        <span>Get latest</span>
      </TdButton>

      <TdButton
        variant="secondary"
        size="sm"
        :disabled="busy || store.filteredRepositories.length === 0"
        title="Upload your saved checkpoints to the server"
        @click="runAction('push')"
      >
        <template #icon-left>
          <TdSpinner v-if="activeAction === 'push'" size="sm" />
          <Upload v-else class="text-info-fg" />
        </template>
        <span>Send changes</span>
      </TdButton>

      <TdMenu aria-label="More Git actions">
        <template #trigger="{ open }">
          <TdButton
            variant="ghost"
            size="sm"
            aria-label="More Git actions"
            aria-haspopup="menu"
            :aria-expanded="open"
          >
            <template #icon-left><MoreHorizontal /></template>
          </TdButton>
        </template>
        <TdMenuItem
          :disabled="busy || store.filteredRepositories.length === 0"
          @select="runAction('fetch')"
        >
          <template #icon><RefreshCw /></template>
          Check for updates
          <template #description>Looks for new work without changing your files.</template>
        </TdMenuItem>
        <TdMenuItem
          :disabled="busy || store.filteredRepositories.length === 0"
          @select="runAction('stash')"
        >
          <template #icon><Archive /></template>
          Set aside changes
          <template #description>Temporarily store unfinished work in all projects.</template>
        </TdMenuItem>
        <TdMenuItem
          :disabled="busy || store.filteredRepositories.length === 0"
          @select="runAction('stash_pop')"
        >
          <template #icon><ArchiveRestore /></template>
          Bring back set-aside changes
          <template #description>Restore the last set of changes in all projects.</template>
        </TdMenuItem>
        <TdMenuItem :disabled="busy" @select="store.scanRepositories">
          <template #icon><FolderSearch /></template>
          Look for new projects
          <template #description>Check your folders for projects again.</template>
        </TdMenuItem>
        <TdMenuItem @select="store.openGitLog()">
          <template #icon><Terminal /></template>
          What Git did
          <template #description>See the exact commands behind the last actions.</template>
        </TdMenuItem>
        <TdMenuItem @select="store.openHelp">
          <template #icon><HelpCircle /></template>
          How this works
        </TdMenuItem>
      </TdMenu>
    </div>
  </header>
</template>
