mod commands {
    pub mod config_commands;
    pub mod git_commands;
}
mod config;
mod commitgen;
mod events;
mod git_log;
mod git_runner;
mod macros;
mod models;
mod scanner;
mod service;
mod watcher;
mod web;

use commands::config_commands::*;
use commands::git_commands::*;
use config::ConfigManager;
use std::path::PathBuf;
use tauri::{Emitter, Manager};
use watcher::{EventBus, WatcherState};
use web::WebOptions;

struct Cli {
    web: bool,
    host: String,
    port: u16,
    token: Option<String>,
    config: Option<PathBuf>,
    dist: Option<PathBuf>,
    accelerated: bool,
    help: bool,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let cli = match parse_cli() {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("error: {error}\n\n{}", help_text());
            std::process::exit(2);
        }
    };

    if cli.help {
        print!("{}", help_text());
        return;
    }

    if cli.web {
        if let Err(error) = web::run(WebOptions {
            host: cli.host,
            port: cli.port,
            token: cli.token,
            config_path: cli.config,
            dist_dir: cli.dist,
        }) {
            eprintln!("web server failed: {error}");
            std::process::exit(1);
        }
        return;
    }

    run_desktop(cli);
}

fn run_desktop(cli: Cli) {
    // WebKitGTK negotiates GBM buffer modifiers through Mesa for its DMA-BUF
    // renderer; the NVIDIA proprietary driver rejects them, so allocation
    // fails with `Failed to create GBM buffer ...: Invalid argument` and the
    // window stays blank. Same mitigation as docdeck: switch the path off by
    // default, but let an explicit export or --accelerated opt back in.
    // Must run before the first window exists - WebKitGTK latches these
    // settings while it initialises.
    #[cfg(target_os = "linux")]
    if !cli.accelerated && std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    let config_manager = match cli.config {
        Some(path) => ConfigManager::from_path(path),
        None => ConfigManager::init(),
    };
    let event_bus = EventBus::new();
    let watcher_state = WatcherState::new(event_bus.clone());

    tauri::Builder::default()
        .manage(config_manager)
        .manage(event_bus)
        .manage(watcher_state)
        .setup(|app| {
            let event_bus = app.state::<EventBus>().inner().clone();
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut receiver = event_bus.subscribe();
                while let Ok(event) = receiver.recv().await {
                    match event {
                        crate::events::AppEvent::FsChanged(repo_path) => {
                            let _ = handle.emit("repo:fs-changed", repo_path);
                        }
                        crate::events::AppEvent::GitStep(step) => {
                            let _ = handle.emit("git:step", step);
                        }
                        crate::events::AppEvent::ScanProgress(progress) => {
                            let _ = handle.emit("scan:progress", progress);
                        }
                        crate::events::AppEvent::CommitGen(commit_gen) => {
                            let _ = handle.emit("commitgen:repo", commit_gen);
                        }
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            add_scan_path,
            remove_scan_path,
            save_repo_group,
            delete_repo_group,
            save_onboarding_state,
            discover_repositories,
            refresh_repository,
            list_repo_changes,
            commit_repo_changes,
            execute_group_action,
            switch_group_branch,
            list_group_branches,
            generate_commit_messages
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn parse_cli() -> Result<Cli, String> {
    let mut cli = Cli {
        web: env_flag("WEB"),
        host: env_legacy("HOST").unwrap_or_else(|| "127.0.0.1".to_string()),
        port: env_legacy("PORT")
            .and_then(|value| value.parse().ok())
            .unwrap_or(8787),
        token: env_legacy("TOKEN").filter(|value| !value.is_empty()),
        config: env_legacy("CONFIG").map(PathBuf::from),
        dist: env_legacy("DIST").map(PathBuf::from),
        accelerated: false,
        help: false,
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--web" => cli.web = true,
            "--accelerated" => cli.accelerated = true,
            "--help" | "-h" => cli.help = true,
            "--host" => cli.host = args.next().ok_or("--host requires a value")?,
            "--port" => {
                cli.port = args
                    .next()
                    .ok_or("--port requires a value")?
                    .parse()
                    .map_err(|_| "invalid value for --port".to_string())?;
            }
            "--token" => {
                cli.token = args.next().filter(|value| !value.is_empty());
            }
            "--config" => {
                cli.config = Some(PathBuf::from(
                    args.next().ok_or("--config requires a value")?,
                ));
            }
            "--dist" => {
                cli.dist = Some(PathBuf::from(args.next().ok_or("--dist requires a value")?));
            }
            value if value.starts_with("--host=") => {
                cli.host = value["--host=".len()..].to_string();
            }
            value if value.starts_with("--port=") => {
                cli.port = value["--port=".len()..]
                    .parse()
                    .map_err(|_| "invalid value for --port".to_string())?;
            }
            value if value.starts_with("--token=") => {
                cli.token =
                    Some(value["--token=".len()..].to_string()).filter(|token| !token.is_empty());
            }
            value if value.starts_with("--config=") => {
                cli.config = Some(PathBuf::from(&value["--config=".len()..]));
            }
            value if value.starts_with("--dist=") => {
                cli.dist = Some(PathBuf::from(&value["--dist=".len()..]));
            }
            _ => {}
        }
    }

    Ok(cli)
}

/// Reads `REPOMAN_<suffix>`, falling back to the pre-rename `GIT_REPO_MANAGER_<suffix>`
/// so setups written before the `repoman` rename keep working.
fn env_legacy(suffix: &str) -> Option<String> {
    match std::env::var(format!("REPOMAN_{suffix}")) {
        Ok(value) => Some(value),
        Err(_) => std::env::var(format!("GIT_REPO_MANAGER_{suffix}")).ok(),
    }
}

fn env_flag(suffix: &str) -> bool {
    env_legacy(suffix)
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn help_text() -> &'static str {
    "RepoMan

Usage:
  repoman                 Launch the desktop application
  repoman --web           Serve the browser UI and HTTP API

Options:
  --web                 Run in web server mode (no desktop window)
  --host <HOST>         Bind address for web mode (default: 127.0.0.1)
  --port <PORT>         Port for web mode (default: 8787)
  --token <TOKEN>       Require this access token in web mode
  --config <PATH>       Use a custom config.json location
  --dist <PATH>         Frontend build directory for web mode (default: dist)
  --accelerated         Keep WebKitGTK's DMA-BUF renderer (default: disabled)
  -h, --help            Print this help

Environment variables:
  REPOMAN_WEB=1
  REPOMAN_HOST / REPOMAN_PORT
  REPOMAN_TOKEN / REPOMAN_CONFIG / REPOMAN_DIST
  The legacy GIT_REPO_MANAGER_* names are still accepted.

Web mode example:
  pnpm build
  repoman --web --host 0.0.0.0 --port 8787 --token \"$(openssl rand -hex 16)\"
"
}
