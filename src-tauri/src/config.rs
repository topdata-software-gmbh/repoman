use crate::models::AppConfig;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: Mutex<AppConfig>,
}

impl ConfigManager {
    pub fn init() -> Self {
        Self::from_path(default_config_path(&dirs_fallback()))
    }

    pub fn from_path(config_path: PathBuf) -> Self {
        if let Some(parent) = config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let initial_config = if config_path.exists() {
            fs::read_to_string(&config_path)
                .ok()
                .and_then(|data| serde_json::from_str::<AppConfig>(&data).ok())
                .unwrap_or_default()
        } else {
            AppConfig::default()
        };

        Self {
            config_path,
            config: Mutex::new(initial_config),
        }
    }

    pub fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    pub fn snapshot(&self) -> Result<AppConfig, String> {
        self.config
            .lock()
            .map(|cfg| cfg.clone())
            .map_err(|e| e.to_string())
    }

    pub fn group_ids_by_path(&self) -> Result<HashMap<String, Vec<String>>, String> {
        let cfg = self.config.lock().map_err(|e| e.to_string())?;
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        for group in &cfg.groups {
            for path in &group.repo_paths {
                map.entry(path.clone()).or_default().push(group.id.clone());
            }
        }
        Ok(map)
    }

    pub fn save(&self) -> Result<(), String> {
        let serialized = {
            let config = self.config.lock().map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&*config).map_err(|e| e.to_string())?
        };

        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Write through a temp file so a crash or full disk cannot truncate the config.
        let temp_path = self.config_path.with_extension("json.tmp");
        fs::write(&temp_path, serialized).map_err(|e| e.to_string())?;

        #[cfg(target_os = "windows")]
        {
            if self.config_path.exists() {
                fs::remove_file(&self.config_path).map_err(|e| e.to_string())?;
            }
        }

        fs::rename(&temp_path, &self.config_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Config lives in `<config root>/repoman/config.json`. Installs made before the
/// `repoman` rename kept it under `git-repo-manager/`, so that file is copied over
/// once instead of silently presenting an empty workspace.
fn default_config_path(root: &std::path::Path) -> PathBuf {
    let current = root.join("repoman").join("config.json");
    let legacy = root.join("git-repo-manager").join("config.json");

    if !current.exists() && legacy.exists() {
        if let Some(parent) = current.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Copy (not move) so the previous install stays recoverable.
        if fs::copy(&legacy, &current).is_ok() {
            return current;
        }
    }

    current
}

fn dirs_fallback() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            #[cfg(target_os = "windows")]
            {
                std::env::var_os("APPDATA").map(PathBuf::from)
            }
            #[cfg(not(target_os = "windows"))]
            {
                std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config"))
            }
        })
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{GroupSettings, RepoGroup, ScanPath};

    #[test]
    fn saves_and_reloads_configuration_atomically() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        let manager = ConfigManager::from_path(config_path.clone());

        {
            let mut cfg = manager.config.lock().unwrap();
            cfg.scan_paths.push(ScanPath {
                id: "scan-1".to_string(),
                path: "/tmp/repos".to_string(),
                max_depth: 3,
                enabled: true,
                exclude_patterns: vec![],
            });
            cfg.groups.push(RepoGroup {
                id: "group-1".to_string(),
                name: "Services".to_string(),
                color: "#38bdf8".to_string(),
                repo_paths: vec!["/tmp/repos/api".to_string()],
                settings: GroupSettings {
                    sync_branching: true,
                    auto_stash_on_switch: true,
                    default_base_branch: "main".to_string(),
                },
            });
        }

        manager.save().unwrap();
        assert!(!config_path.with_extension("json.tmp").exists());

        let raw = fs::read_to_string(&config_path).unwrap();
        assert!(raw.contains("\"scanPaths\""));
        assert!(raw.contains("\"repoPaths\""));

        let reloaded = ConfigManager::from_path(config_path);
        let snapshot = reloaded.snapshot().unwrap();
        assert_eq!(snapshot.scan_paths.len(), 1);
        assert_eq!(snapshot.groups.len(), 1);
        assert_eq!(snapshot.groups[0].settings.default_base_branch, "main");

        let group_map = reloaded.group_ids_by_path().unwrap();
        assert_eq!(group_map["/tmp/repos/api"], vec!["group-1".to_string()]);
    }

    #[test]
    fn loads_legacy_config_without_onboarding() {
        let temp = tempfile::tempdir().unwrap();
        let config_path = temp.path().join("config.json");
        fs::write(&config_path, r#"{"scanPaths":[],"groups":[]}"#).unwrap();

        let manager = ConfigManager::from_path(config_path);
        let snapshot = manager.snapshot().unwrap();

        assert!(!snapshot.onboarding.started);
        assert!(!snapshot.onboarding.completed);
        assert_eq!(snapshot.onboarding.current_step, 0);
        assert_eq!(snapshot.onboarding.owner_names, Vec::<String>::new());
    }

    #[test]
    fn migrates_the_pre_rename_config_directory() {
        let temp = tempfile::tempdir().unwrap();
        let legacy_dir = temp.path().join("git-repo-manager");
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::write(
            legacy_dir.join("config.json"),
            r#"{"scanPaths":[],"groups":[]}"#,
        )
        .unwrap();

        let path = default_config_path(temp.path());

        assert_eq!(path, temp.path().join("repoman").join("config.json"));
        assert!(path.exists());
        assert!(legacy_dir.join("config.json").exists());

        let manager = ConfigManager::from_path(path);
        assert!(manager.snapshot().unwrap().scan_paths.is_empty());
    }

    #[test]
    fn keeps_the_new_directory_when_it_already_exists() {
        let temp = tempfile::tempdir().unwrap();
        let current_dir = temp.path().join("repoman");
        let legacy_dir = temp.path().join("git-repo-manager");
        fs::create_dir_all(&current_dir).unwrap();
        fs::create_dir_all(&legacy_dir).unwrap();
        fs::write(
            current_dir.join("config.json"),
            r#"{"scanPaths":[],"groups":[]}"#,
        )
        .unwrap();
        fs::write(legacy_dir.join("config.json"), "stale").unwrap();

        assert_eq!(
            default_config_path(temp.path()),
            current_dir.join("config.json")
        );
        assert_eq!(
            fs::read_to_string(legacy_dir.join("config.json")).unwrap(),
            "stale"
        );
    }
}
