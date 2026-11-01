use crate::commands::git_commands::GenerateCommitMessagesResponse;
use crate::config::ConfigManager;
use crate::models::{
    AppConfig, BranchInfo, FileChange, OnboardingState, OperationResult, RepoGroup, Repository,
    ScanPath,
};
use crate::service;
use crate::watcher::{EventBus, WatcherState};
use axum::extract::{Path as AxumPath, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::Deserialize;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::services::{ServeDir, ServeFile};

pub struct WebOptions {
    pub host: String,
    pub port: u16,
    pub token: Option<String>,
    pub config_path: Option<PathBuf>,
    pub dist_dir: Option<PathBuf>,
}

struct WebState {
    config: Arc<ConfigManager>,
    watcher: Arc<WatcherState>,
    event_bus: EventBus,
    token: Option<String>,
}

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

fn api_error(error: String) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, error)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverRequest {
    paths: Vec<String>,
    max_depth: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshRequest {
    path: String,
    group_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupActionRequest {
    repo_paths: Vec<String>,
    action: String,
    concurrency_limit: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwitchBranchRequest {
    repo_paths: Vec<String>,
    branch: String,
    auto_stash: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BranchListRequest {
    repo_paths: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepoRequest {
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommitRequest {
    path: String,
    message: String,
}

async fn get_config(State(state): State<Arc<WebState>>) -> ApiResult<AppConfig> {
    service::get_config(state.config.as_ref())
        .map(Json)
        .map_err(api_error)
}

async fn add_scan_path(
    State(state): State<Arc<WebState>>,
    Json(scan_path): Json<ScanPath>,
) -> ApiResult<AppConfig> {
    service::add_scan_path(state.config.as_ref(), scan_path)
        .map(Json)
        .map_err(api_error)
}

async fn remove_scan_path(
    State(state): State<Arc<WebState>>,
    AxumPath(id): AxumPath<String>,
) -> ApiResult<AppConfig> {
    service::remove_scan_path(state.config.as_ref(), id)
        .map(Json)
        .map_err(api_error)
}

async fn save_repo_group(
    State(state): State<Arc<WebState>>,
    Json(group): Json<RepoGroup>,
) -> ApiResult<AppConfig> {
    service::save_repo_group(state.config.as_ref(), group)
        .map(Json)
        .map_err(api_error)
}

async fn delete_repo_group(
    State(state): State<Arc<WebState>>,
    AxumPath(group_id): AxumPath<String>,
) -> ApiResult<AppConfig> {
    service::delete_repo_group(state.config.as_ref(), group_id)
        .map(Json)
        .map_err(api_error)
}

async fn save_onboarding_state(
    State(state): State<Arc<WebState>>,
    Json(onboarding): Json<OnboardingState>,
) -> ApiResult<AppConfig> {
    service::save_onboarding_state(state.config.as_ref(), onboarding)
        .map(Json)
        .map_err(api_error)
}

async fn discover_repositories(
    State(state): State<Arc<WebState>>,
    Json(request): Json<DiscoverRequest>,
) -> ApiResult<Vec<Repository>> {
    service::discover_repositories(
        state.config.as_ref(),
        state.watcher.as_ref(),
        request.paths,
        request.max_depth,
        Arc::new(state.event_bus.clone()),
    )
    .await
    .map(Json)
    .map_err(api_error)
}

async fn refresh_repository(Json(request): Json<RefreshRequest>) -> ApiResult<Repository> {
    Ok(Json(
        service::refresh_repository(request.path, request.group_ids).await,
    ))
}

async fn list_repo_changes(Json(request): Json<RepoRequest>) -> ApiResult<Vec<FileChange>> {
    Ok(Json(service::list_repo_changes(request.path).await))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateCommitMessagesRequest {
    repo_paths: Vec<String>,
    run_id: Option<String>,
}

/// Starts a generation run; answers 202 with the runId (contract C3).
async fn generate_commit_messages(
    State(state): State<Arc<WebState>>,
    Json(request): Json<GenerateCommitMessagesRequest>,
) -> Result<(StatusCode, Json<GenerateCommitMessagesResponse>), (StatusCode, String)> {
    let run_id = service::generate_commit_messages(
        request.repo_paths,
        request.run_id,
        state.config.as_ref(),
        Arc::new(state.event_bus.clone()),
    )
    .map_err(api_error)?;
    Ok((
        StatusCode::ACCEPTED,
        Json(GenerateCommitMessagesResponse { run_id }),
    ))
}

async fn commit_repo_changes(
    State(state): State<Arc<WebState>>,
    Json(request): Json<CommitRequest>,
) -> ApiResult<OperationResult> {
    Ok(Json(
        service::commit_repo_changes(request.path, request.message, &state.event_bus).await,
    ))
}

async fn execute_group_action(
    State(state): State<Arc<WebState>>,
    Json(request): Json<GroupActionRequest>,
) -> ApiResult<Vec<OperationResult>> {
    service::execute_group_action(
        request.repo_paths,
        request.action,
        request.concurrency_limit,
        Arc::new(state.event_bus.clone()),
    )
    .await
    .map(Json)
    .map_err(api_error)
}

async fn switch_group_branch(
    State(state): State<Arc<WebState>>,
    Json(request): Json<SwitchBranchRequest>,
) -> ApiResult<Vec<OperationResult>> {
    service::switch_group_branch(
        request.repo_paths,
        request.branch,
        request.auto_stash,
        Arc::new(state.event_bus.clone()),
    )
    .await
    .map(Json)
    .map_err(api_error)
}

async fn list_group_branches(Json(request): Json<BranchListRequest>) -> ApiResult<Vec<BranchInfo>> {
    service::list_group_branches(request.repo_paths)
        .await
        .map(Json)
        .map_err(api_error)
}

async fn repo_events(
    State(state): State<Arc<WebState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.event_bus.subscribe();
    let stream = BroadcastStream::new(receiver)
        .filter_map(|message| message.ok())
        .filter_map(|event| match event {
            crate::events::AppEvent::FsChanged(path) => Some(Ok::<_, Infallible>(
                Event::default().event("repo-fs-changed").data(path),
            )),
            crate::events::AppEvent::GitStep(step) => serde_json::to_string(&step)
                .ok()
                .map(|json| Ok::<_, Infallible>(Event::default().event("git-step").data(json))),
            crate::events::AppEvent::ScanProgress(progress) => {
                serde_json::to_string(&progress).ok().map(|json| {
                    Ok::<_, Infallible>(Event::default().event("scan-progress").data(json))
                })
            }
            crate::events::AppEvent::CommitGen(commit_gen) => {
                serde_json::to_string(&commit_gen).ok().map(|json| {
                    Ok::<_, Infallible>(Event::default().event("commitgen-repo").data(json))
                })
            }
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn require_token(
    State(state): State<Arc<WebState>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(expected) = state.token.as_deref() else {
        return next.run(request).await;
    };

    let expected_header = format!("Bearer {expected}");
    let header_ok = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == expected_header);
    let query_ok = request.uri().query().is_some_and(|query| {
        url::form_urlencoded::parse(query.as_bytes())
            .any(|(key, value)| key == "token" && value.as_ref() == expected)
    });

    if header_ok || query_ok {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Unauthorized: missing or invalid access token",
        )
            .into_response()
    }
}

pub fn run(options: WebOptions) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(serve(options))
}

async fn serve(options: WebOptions) -> Result<(), Box<dyn std::error::Error>> {
    let event_bus = EventBus::new();
    let watcher = Arc::new(WatcherState::new(event_bus.clone()));
    let config = Arc::new(match options.config_path.clone() {
        Some(path) => ConfigManager::from_path(path),
        None => ConfigManager::init(),
    });
    let config_path = config.config_path().to_path_buf();

    let token = options.token.clone().or_else(|| {
        if is_loopback(&options.host) {
            None
        } else {
            Some(generate_token())
        }
    });

    let state = Arc::new(WebState {
        config,
        watcher,
        event_bus,
        token: token.clone(),
    });

    let dist_dir = options
        .dist_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("dist"));
    let index_path = dist_dir.join("index.html");

    if !index_path.exists() {
        log::warn!(
            "{} not found. Build the frontend with `pnpm build` or pass --dist <dir>.",
            index_path.display()
        );
    }

    let static_service = ServeDir::new(&dist_dir).not_found_service(ServeFile::new(&index_path));

    let app = Router::new()
        .route("/api/config", get(get_config))
        .route("/api/scan-paths", post(add_scan_path))
        .route("/api/scan-paths/{id}", delete(remove_scan_path))
        .route("/api/groups", post(save_repo_group))
        .route("/api/groups/{id}", delete(delete_repo_group))
        .route("/api/onboarding", post(save_onboarding_state))
        .route("/api/repositories/discover", post(discover_repositories))
        .route("/api/repositories/refresh", post(refresh_repository))
        .route("/api/repositories/changes", post(list_repo_changes))
        .route("/api/repositories/commit", post(commit_repo_changes))
        .route(
            "/api/repositories/generate-commit-messages",
            post(generate_commit_messages),
        )
        .route("/api/repositories/actions", post(execute_group_action))
        .route("/api/repositories/switch-branch", post(switch_group_branch))
        .route("/api/branches", post(list_group_branches))
        .route("/api/events", get(repo_events))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token))
        .fallback_service(static_service)
        .with_state(state);

    let bind_address = format!("{}:{}", options.host, options.port);
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    let local_address = listener.local_addr()?;
    let display_host = display_host(&options.host);

    log::info!("configuration: {}", config_path.display());
    log::info!("static assets: {}", dist_dir.display());

    println!(
        "RepoMan web UI: http://{}:{}/",
        display_host,
        local_address.port()
    );
    if let Some(token) = &token {
        let encoded_token: String =
            url::form_urlencoded::byte_serialize(token.as_bytes()).collect();
        println!(
            "Access token: {token}\nOpen: http://{}:{}/?token={encoded_token}",
            display_host,
            local_address.port()
        );
    }
    if !is_loopback(&options.host) {
        println!("Listening on {bind_address} (network access enabled).");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn display_host(host: &str) -> &str {
    match host {
        "0.0.0.0" => "127.0.0.1",
        "::" | "[::]" => "[::1]",
        other => other,
    }
}

fn is_loopback(host: &str) -> bool {
    matches!(
        host,
        "127.0.0.1" | "localhost" | "::1" | "[::1]" | "0:0:0:0:0:0:0:1"
    )
}

fn generate_token() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
