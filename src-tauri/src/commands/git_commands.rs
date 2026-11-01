use crate::config::ConfigManager;
use crate::models::{BranchInfo, FileChange, OperationResult, Repository};
use crate::service;
use crate::watcher::{EventBus, WatcherState};
use tauri::State;

#[tauri::command]
pub async fn discover_repositories(
    paths: Vec<String>,
    max_depth: usize,
    config_state: State<'_, ConfigManager>,
    watcher_state: State<'_, WatcherState>,
    event_bus: State<'_, EventBus>,
) -> Result<Vec<Repository>, String> {
    service::discover_repositories(
        config_state.inner(),
        watcher_state.inner(),
        paths,
        max_depth,
        std::sync::Arc::new(event_bus.inner().clone()),
    )
    .await
}

#[tauri::command]
pub async fn refresh_repository(
    path: String,
    group_ids: Vec<String>,
) -> Result<Repository, String> {
    Ok(service::refresh_repository(path, group_ids).await)
}

#[tauri::command]
pub async fn list_repo_changes(path: String) -> Result<Vec<FileChange>, String> {
    Ok(service::list_repo_changes(path).await)
}

#[tauri::command]
pub async fn commit_repo_changes(
    path: String,
    message: String,
    event_bus: State<'_, EventBus>,
) -> Result<OperationResult, String> {
    Ok(service::commit_repo_changes(path, message, event_bus.inner()).await)
}

/// Response of the start-generation command: `{ "runId": … }` (contract C1).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateCommitMessagesResponse {
    pub run_id: String,
}

#[tauri::command]
pub async fn generate_commit_messages(
    repo_paths: Vec<String>,
    run_id: Option<String>,
    config: State<'_, ConfigManager>,
    event_bus: State<'_, EventBus>,
) -> Result<GenerateCommitMessagesResponse, String> {
    let run_id = service::generate_commit_messages(
        repo_paths,
        run_id,
        config.inner(),
        std::sync::Arc::new(event_bus.inner().clone()),
    )?;
    Ok(GenerateCommitMessagesResponse { run_id })
}

#[tauri::command]
pub async fn execute_group_action(
    repo_paths: Vec<String>,
    action: String,
    concurrency_limit: usize,
    event_bus: State<'_, EventBus>,
) -> Result<Vec<OperationResult>, String> {
    service::execute_group_action(
        repo_paths,
        action,
        concurrency_limit,
        std::sync::Arc::new(event_bus.inner().clone()),
    )
    .await
}

#[tauri::command]
pub async fn switch_group_branch(
    repo_paths: Vec<String>,
    branch: String,
    auto_stash: bool,
    event_bus: State<'_, EventBus>,
) -> Result<Vec<OperationResult>, String> {
    service::switch_group_branch(
        repo_paths,
        branch,
        auto_stash,
        std::sync::Arc::new(event_bus.inner().clone()),
    )
    .await
}

#[tauri::command]
pub async fn list_group_branches(repo_paths: Vec<String>) -> Result<Vec<BranchInfo>, String> {
    service::list_group_branches(repo_paths).await
}
