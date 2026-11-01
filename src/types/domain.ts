export interface ScanPath {
  id: string;
  path: string;
  maxDepth: number;
  enabled: boolean;
  excludePatterns: string[];
}

export interface RemoteInfo {
  name: string;
  url: string;
}

export type OperationState = 'idle' | 'merging' | 'rebasing' | 'cherry-picking';

export type RepoKind = 'theme' | 'plugin' | 'other';
export type GroupingStyle = 'topLevel' | 'single';

export interface OnboardingState {
  completed: boolean;
  started: boolean;
  currentStep: number;
  scanPath: string | null;
  ownerNames: string[];
  groupingStyle: GroupingStyle;
  selectedRepoPaths: string[];
}

export interface GitRepoState {
  currentBranch: string;
  isDetachedHead: boolean;
  headCommitHash: string;
  headCommitMessage: string;
  isClean: boolean;
  aheadCount: number;
  behindCount: number;
  modifiedFiles: number;
  untrackedFiles: number;
  stagedFiles: number;
  hasConflicts: boolean;
  operationState: OperationState;
  remotes: RemoteInfo[];
}

export interface Repository {
  id: string;
  name: string;
  path: string;
  isAccessible: boolean;
  state: GitRepoState;
  groupIds: string[];
  kind: RepoKind;
}

export interface GroupSettings {
  syncBranching: boolean;
  autoStashOnSwitch: boolean;
  defaultBaseBranch: string;
}

export interface RepoGroup {
  id: string;
  name: string;
  color: string;
  repoPaths: string[];
  settings: GroupSettings;
}

export interface AppConfig {
  scanPaths: ScanPath[];
  groups: RepoGroup[];
  onboarding: OnboardingState;
  ai?: AiSettings | null;
}

export interface OperationResult {
  repoPath: string;
  success: boolean;
  output: string;
  error: string | null;
}

export interface BranchInfo {
  name: string;
  repoCount: number;
}

export type FileChangeStatus = 'modified' | 'new' | 'deleted' | 'renamed' | 'conflict';

export interface FileChange {
  path: string;
  status: FileChangeStatus;
}

export interface OperationProgressEvent {
  repoPath: string;
  operation: 'fetch' | 'pull' | 'push' | 'checkout' | 'stash';
  status: 'pending' | 'running' | 'success' | 'failed';
  message?: string;
}

export type GitStepStatus = 'running' | 'ok' | 'failed' | 'skipped';

export interface GitStepEvent {
  runId: string;
  repoPath: string;
  macroId: string;
  title: string;
  stepIndex: number;
  comment: string;
  command: string;
  status: GitStepStatus;
  durationMs: number;
  message: string | null;
  alias: string | null;
}

export interface GitLogRow {
  key: string;
  stepIndex: number;
  comment: string;
  command: string;
  alias: string | null;
  status: GitStepStatus;
  repoPaths: string[];
  message: string | null;
}

export interface GitRun {
  runId: string;
  title: string;
  macroId: string;
  status: GitStepStatus;
  durationMs: number;
  rows: GitLogRow[];
  finishedAt: string;
}

export type ScanProgressPhase = 'started' | 'found' | 'tick' | 'inspected' | 'done';

/** Live scan progress payload (contracts/scan-progress-event.md §C4). */
export interface ScanProgressEvent {
  phase: ScanProgressPhase;
  runId: string;
  roots?: string[];
  startedAt?: number;
  root?: string;
  path?: string;
  name?: string;
  kind?: RepoKind;
  currentDir?: string;
  foundCount?: number;
  elapsedMs?: number;
  error?: string | null;
}

export interface ScanActivityEntry {
  name: string;
  root: string;
  kind: RepoKind | null;
  inspected: boolean;
}

export interface ScanActivityFinished {
  count: number;
  elapsedMs: number;
  error?: string | null;
}

export interface ScanActivityState {
  runId: string;
  roots: string[];
  running: boolean;
  currentDir: string | null;
  currentRoot: string | null;
  startedAt: number;
  entries: Map<string, ScanActivityEntry>;
  finished: ScanActivityFinished | null;
  modalOpen: boolean;
  summaryVisible: boolean;
  elapsedMs: number;
}

export type CommitGenPhase = 'started' | 'done' | 'error';

/** One AI-drafted Conventional Commit message (contract C4 §payload). */
export interface CommitDraft {
  type: string;
  scope: string | null;
  subject: string;
  body: string | null;
  rendered: string;
}

/** Live commit-message generation event (contracts/commitgen-events.md §C4). */
export interface CommitGenEvent {
  phase: CommitGenPhase;
  runId: string;
  path: string;
  draft?: CommitDraft;
  error?: string | null;
}

export type CommitGenRowStatus = 'idle' | 'generating' | 'ready' | 'error';

/** One dialog row while a generation run is live (data-model §CommitGenRow). */
export interface RepoDraftRow {
  path: string;
  name: string;
  status: CommitGenRowStatus;
  message: string;
  dirty: boolean;
  error: string | null;
}

/** Non-secret AI settings as exposed by the backend (data-model §AiSettings). */
export interface AiSettings {
  baseUrl: string;
  model: string;
  apiKeyEnv: string;
  maxToolCalls: number;
  diffBudgetChars: number;
}
