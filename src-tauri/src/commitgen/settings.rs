//! Optional AI settings persisted inside `AppConfig.ai` (data-model E1).
//!
//! Security rule (FR-013, SC-006): `api_key_env` stores the NAME of the
//! environment variable holding the credential. The credential VALUE is read
//! from the process environment at run start and is never persisted, logged or
//! sent to the UI.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Default environment variable consulted for the provider credential.
pub const DEFAULT_API_KEY_ENV: &str = "OPENAI_API_KEY";
/// Default per-run repository-lookup budget (FR-010, research R3).
pub const DEFAULT_MAX_TOOL_CALLS: u32 = 12;
/// Default hard cap on the change detail sent for one repository (FR-009, research R4).
pub const DEFAULT_DIFF_BUDGET_CHARS: usize = 40_000;

fn default_api_key_env() -> String {
    DEFAULT_API_KEY_ENV.to_string()
}

fn default_max_tool_calls() -> u32 {
    DEFAULT_MAX_TOOL_CALLS
}

fn default_diff_budget_chars() -> usize {
    DEFAULT_DIFF_BUDGET_CHARS
}

/// Application-owned AI configuration (spec Key Entity "AI Settings").
///
/// Absent (`AppConfig.ai == None`) means the feature is visibly unavailable,
/// never an error (US-5). A present-but-invalid block is treated the same way
/// for runs started after load, with a distinct hint string (data-model E1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    /// OpenAI-compatible endpoint, e.g. `https://openrouter.ai/api/v1`.
    pub base_url: String,
    /// Model id passed to the provider, e.g. `anthropic/claude-sonnet-4.5`.
    pub model: String,
    /// NAME of the environment variable holding the credential (never the value).
    #[serde(default = "default_api_key_env")]
    pub api_key_env: String,
    /// Per-run lookup budget, passed into the agent run limits (FR-010).
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls: u32,
    /// Hard cap on the change detail sent per repository (FR-009).
    #[serde(default = "default_diff_budget_chars")]
    pub diff_budget_chars: usize,
}

/// Why an `AiSettings` block cannot be used (rendered as `save.ai.badConfig`).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AiConfigError {
    #[error("base_url must be an http(s) URL, got {0:?}")]
    InvalidBaseUrl(String),
    #[error("model must not be empty")]
    EmptyModel,
}

impl AiSettings {
    /// Validates the block the way the UI and both transports need it
    /// (data-model E1 rule 2): `base_url` must parse as `http(s)://…`.
    pub fn validate(&self) -> Result<(), AiConfigError> {
        let parsed = url::Url::parse(self.base_url.trim())
            .map_err(|_| AiConfigError::InvalidBaseUrl(self.base_url.clone()))?;
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return Err(AiConfigError::InvalidBaseUrl(self.base_url.clone()));
        }
        if self.model.trim().is_empty() {
            return Err(AiConfigError::EmptyModel);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_json(json: &str) -> AiSettings {
        serde_json::from_str(json).expect("valid ai block")
    }

    #[test]
    fn applies_documented_defaults() {
        let settings = from_json(r#"{"baseUrl":"http://localhost:11434/v1","model":"qwen3"}"#);
        assert_eq!(settings.api_key_env, "OPENAI_API_KEY");
        assert_eq!(settings.max_tool_calls, 12);
        assert_eq!(settings.diff_budget_chars, 40_000);
    }

    #[test]
    fn honours_explicit_values() {
        let settings = from_json(
            r#"{"baseUrl":"https://openrouter.ai/api/v1","model":"m","apiKeyEnv":"MY_KEY","maxToolCalls":3,"diffBudgetChars":1000}"#,
        );
        assert_eq!(settings.api_key_env, "MY_KEY");
        assert_eq!(settings.max_tool_calls, 3);
        assert_eq!(settings.diff_budget_chars, 1_000);
    }

    #[test]
    fn serializes_back_as_camel_case() {
        let settings = from_json(r#"{"baseUrl":"http://h/v1","model":"m","apiKeyEnv":"K","maxToolCalls":5,"diffBudgetChars":7}"#);
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("\"apiKeyEnv\":\"K\""), "{json}");
        assert!(json.contains("\"maxToolCalls\":5"), "{json}");
        assert!(json.contains("\"diffBudgetChars\":7"), "{json}");
    }

    #[test]
    fn accepts_http_and_https_base_urls() {
        assert!(from_json(r#"{"baseUrl":"http://localhost:11434/v1","model":"m"}"#)
            .validate()
            .is_ok());
        assert!(
            from_json(r#"{"baseUrl":"https://openrouter.ai/api/v1","model":"m"}"#)
                .validate()
                .is_ok()
        );
    }

    #[test]
    fn rejects_non_http_and_blank_values() {
        assert_eq!(
            from_json(r#"{"baseUrl":"ftp://localhost/x","model":"m"}"#).validate(),
            Err(AiConfigError::InvalidBaseUrl("ftp://localhost/x".into()))
        );
        assert_eq!(
            from_json(r#"{"baseUrl":"not a url","model":"m"}"#).validate(),
            Err(AiConfigError::InvalidBaseUrl("not a url".into()))
        );
        assert_eq!(
            from_json(r#"{"baseUrl":"https://x.dev/v1","model":"  "}"#).validate(),
            Err(AiConfigError::EmptyModel)
        );
    }

    #[test]
    fn round_trips_through_the_config_manager_without_the_credential() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let manager = crate::config::ConfigManager::from_path(path.clone());
        let mut config = manager.snapshot().unwrap();
        config.ai = Some(from_json(
            r#"{"baseUrl":"http://localhost:11434/v1","model":"qwen3","apiKeyEnv":"OLLAMA_API_KEY_PLACEHOLDER"}"#,
        ));
        {
            *manager.config.lock().unwrap() = config;
        }
        manager.save().unwrap();

        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(raw.contains("OLLAMA_API_KEY_PLACEHOLDER"), "{raw}");
        // Only the variable NAME is persisted — nothing looks like a secret value.
        assert!(!raw.contains("sk-"), "{raw}");

        let reloaded = crate::config::ConfigManager::from_path(path)
            .snapshot()
            .unwrap();
        let ai = reloaded.ai.expect("ai block survives reload");
        assert_eq!(ai.model, "qwen3");
        assert_eq!(ai.api_key_env, "OLLAMA_API_KEY_PLACEHOLDER");
    }

    #[test]
    fn absent_ai_block_deserializes_to_none() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"scanPaths":[],"groups":[]}"#).unwrap();
        let config = crate::config::ConfigManager::from_path(path).snapshot().unwrap();
        assert!(config.ai.is_none());
    }
}
