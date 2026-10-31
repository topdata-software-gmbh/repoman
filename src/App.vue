<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { subscribeToEvents } from './lib/backend';
import { t } from './lib/i18n';
import { useRepoStore } from './stores/repoStore';
import AllReposPage from './components/AllReposPage.vue';
import AddScanPathModal from './components/AddScanPathModal.vue';
import GitLogDrawer from './components/GitLogDrawer.vue';
import GroupEditorModal from './components/GroupEditorModal.vue';
import GuidanceBanner from './components/GuidanceBanner.vue';
import HelpModal from './components/HelpModal.vue';
import OnboardingWizard from './components/OnboardingWizard.vue';
import RepoTable from './components/RepoTable.vue';
import SaveChangesModal from './components/SaveChangesModal.vue';
import Sidebar from './components/Sidebar.vue';
import ToastStack from './components/ToastStack.vue';
import Toolbar from './components/Toolbar.vue';

const store = useRepoStore();
let unsubscribe: (() => void) | null = null;

onMounted(async () => {
  await store.initialize();

  try {
    unsubscribe = await subscribeToEvents({
      onRepoChanged: (path) => void store.refreshSingleRepo(path),
      onGitStep: (step) => store.recordGitStep(step),
    });
  } catch (error) {
    store.showToast('error', t('app.liveUpdatesUnavailable.title'), String(error));
  }
});

onUnmounted(() => unsubscribe?.());
</script>

<template>
  <div class="flex h-screen w-screen overflow-hidden bg-canvas font-body text-fg">
    <OnboardingWizard v-if="store.onboardingOpen" />
    <template v-else>
      <Sidebar />
      <div class="flex h-full min-w-0 flex-1 flex-col overflow-hidden">
        <AllReposPage v-if="store.selectedGroupId === 'all'" />
        <template v-else>
          <Toolbar />
          <GuidanceBanner />
          <RepoTable />
        </template>
      </div>

      <GroupEditorModal />
      <AddScanPathModal />
      <SaveChangesModal />
      <HelpModal />
      <GitLogDrawer />
    </template>
    <ToastStack />
  </div>
</template>
