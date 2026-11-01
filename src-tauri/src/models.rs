use crate::commitgen::settings::AiSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanPath {
    pub id: String,
    pub path: String,
    pub max_depth: usize,
    pub enabled: bool,
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteInfo {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitRepoState {
    pub current_branch: String,
    pub is_detached_head: bool,
    pub head_commit_hash: String,
    pub head_commit_message: String,
    pub is_clean: bool,
    pub ahead_count: usize,
    pub behind_count: usize,
    pub modified_files: usize,
    pub untracked_files: usize,
    pub staged_files: usize,
    pub has_conflicts: bool,
    pub operation_state: String,
    pub remotes: Vec<RemoteInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum RepoKind {
    Theme,
    Plugin,
    #[default]
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_accessible: bool,
    pub state: GitRepoState,
    pub group_ids: Vec<String>,
    #[serde(default)]
    pub kind: RepoKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupSettings {
    pub sync_branching: bool,
    pub auto_stash_on_switch: bool,
    pub default_base_branch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub repo_paths: Vec<String>,
    pub settings: GroupSettings,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum GroupingStyle {
    #[default]
    TopLevel,
    Single,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub started: bool,
    #[serde(default)]
    pub current_step: u8,
    #[serde(default)]
    pub scan_path: Option<String>,
    #[serde(default)]
    pub owner_names: Vec<String>,
    #[serde(default)]
    pub grouping_style: GroupingStyle,
    #[serde(default)]
    pub selected_repo_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub scan_paths: Vec<ScanPath>,
    pub groups: Vec<RepoGroup>,
    #[serde(default)]
    pub onboarding: OnboardingState,
    /// Optional AI block for commit-message generation (specs/002, data-model E1).
    /// `None` means the feature is visibly unavailable, never an error (US-5).
    /// Persists the credential ENVIRONMENT VARIABLE NAME only — never a secret (FR-013).
    #[serde(default)]
    pub ai: Option<AiSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub repo_path: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    pub name: String,
    pub repo_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub status: String,
}
