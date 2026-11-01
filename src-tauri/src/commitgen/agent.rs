//! Builds and runs the per-repository agent (research R2/R7/R8).
//!
//! The LLM framework is confined to this file and `tools.rs`; services,
//! transports and UI never see it (plan §Structure Decision). At the pinned
//! version the framework does not send an output schema to the provider and does
//! not retry provider errors, so the seam spells the schema into the system
//! prompt, re-checks it through an output validator, and wraps the run in its
//! own bounded exponential-backoff loop.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serdes_ai::agent::{AgentBuilder, OutputValidationError, UsageLimits};
use serdes_ai::agent::{AgentRunError, RunContext};
use serdes_ai::models::OpenAIChatModel;
use serdes_ai::retries::ExponentialBackoff;

use super::diff::ChangeDetail;
use super::draft::{CommitDraft, DraftValidationError};
use super::settings::AiSettings;

/// Provider failures are retried with exponential backoff (research R8).
const MAX_PROVIDER_ATTEMPTS: u32 = 3;
/// The model gets a bounded number of chances to repair a schema violation.
const MAX_OUTPUT_RETRIES: u32 = 2;

/// Agent dependencies: exactly one repository, nothing else (FR-011).
#[derive(Debug)]
pub struct RepoCtx {
    /// Canonicalized repository root; every lookup is prefix-checked against it.
    pub path: PathBuf,
    /// Folder name shown to the model and used in the prompt header.
    pub repo_name: String,
    /// Lookups already performed in this attempt (budget enforcement, FR-010).
    pub tool_calls_used: AtomicU32,
    /// Per-attempt lookup budget from `AiSettings::max_tool_calls`.
    pub lookup_limit: u32,
}

impl RepoCtx {
    pub fn new(path: PathBuf, repo_name: impl Into<String>) -> Self {
        Self {
            path,
            repo_name: repo_name.into(),
            tool_calls_used: AtomicU32::new(0),
            lookup_limit: 12,
        }
    }

    /// Sets the per-attempt lookup budget (default 12).
    pub fn with_lookup_limit(mut self, limit: u32) -> Self {
        self.lookup_limit = limit;
        self
    }

    /// Claims one lookup from the per-run budget.
    pub fn claim_lookup(&self, limit: u32) -> bool {
        self.tool_calls_used
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
                if used < limit {
                    Some(used + 1)
                } else {
                    None
                }
            })
            .is_ok()
    }

    /// Diagnostics and tests; the budget itself is enforced by `claim_lookup`.
    #[allow(dead_code)]
    pub fn lookups_used(&self) -> u32 {
        self.tool_calls_used.load(Ordering::SeqCst)
    }
}

/// Why one repository's draft could not be produced (FR-008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitGenError {
    /// Referenced environment variable is not set; carries the NAME only (FR-013).
    MissingCredential { env_var: String },
    /// Endpoint unreachable / HTTP failure.
    Provider(String),
    /// Model rejected the request or produced an error finish.
    Model(String),
    /// Provider answered with nothing usable.
    EmptyResult,
    /// Provider answered but the draft violates the schema.
    MalformedDraft(String),
    /// Repository has no pending changes.
    NothingToSummarize,
    /// Repository is mid-merge; AI cannot help there.
    Conflicts,
    /// Filesystem or git read failure.
    Io(String),
}

impl CommitGenError {
    /// Non-retryable with AI (contract C4 rows 3–4).
    pub fn is_retryable(&self) -> bool {
        !matches!(self, Self::NothingToSummarize | Self::Conflicts)
    }

    /// Stable, human-readable reason carried in the `error` event (FR-008).
    /// Never contains a credential or a full payload (SC-006).
    pub fn user_reason(&self) -> String {
        match self {
            Self::MissingCredential { env_var } => format!("missing-credential:{env_var}"),
            Self::Provider(detail) => format!("provider-unreachable:{detail}"),
            Self::Model(detail) => format!("model-error:{detail}"),
            Self::EmptyResult => "empty-result".to_string(),
            Self::MalformedDraft(detail) => format!("malformed-draft:{detail}"),
            Self::NothingToSummarize => "nothing-to-summarize".to_string(),
            Self::Conflicts => "merge-conflicts-resolve-first".to_string(),
            Self::Io(detail) => format!("repository-unreadable:{detail}"),
        }
    }
}

/// Input for exactly one repository within one run.
pub struct DraftRequest<'a> {
    pub settings: &'a AiSettings,
    pub credential: Option<String>,
    pub ctx: Arc<RepoCtx>,
    pub change_detail: &'a ChangeDetail,
}

/// System prompt template — golden-file tested (T022) so wording changes show up
/// in review diffs. Placeholders are filled per repository.
pub const SYSTEM_PROMPT: &str = r#"You are RepoMan's commit message writer. You summarize the pending changes of exactly ONE repository: {repo_name}.

Produce one Conventional Commit message for the change detail below. Rules:

1. Format: `type(scope): subject`, optionally followed by a blank line and a body.
2. `type` must be one of: feat, fix, refactor, perf, docs, style, test, build, ci, chore, revert.
3. `subject` is imperative, at most 72 characters, no trailing period, and it must describe THIS repository's own changes.
4. `scope` is the lowercase-hyphen component name, or null when it is not obvious.
5. `body` is optional prose explaining WHY the change was made, never a file list.
6. Match the style of the recent commit subjects when they exist.
7. Use the inspection tools (read_file, file_at_head, recent_history, grep) ONLY when the change detail is not enough to be confident. Inspect sparingly: you have a strict per-run lookup budget.
8. Never invent files, symbols or behaviour that the change detail does not show.
9. Answer with a single JSON object and nothing else, matching this schema:
{output_schema}

Recent commit subjects:
{style_subjects}
"#;

/// Fills the golden prompt template for one repository.
pub fn build_system_prompt(ctx: &RepoCtx, detail: &ChangeDetail) -> String {
    let style = if detail.style_subjects.is_empty() {
        "(no previous commits — this looks like a new repository)".to_string()
    } else {
        detail
            .style_subjects
            .iter()
            .map(|subject| format!("- {subject}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let schema = serde_json::to_string_pretty(&CommitDraft::json_schema_hint())
        .unwrap_or_else(|_| "{}".to_string());
    SYSTEM_PROMPT
        .replace("{repo_name}", &ctx.repo_name)
        .replace("{output_schema}", &schema)
        .replace("{style_subjects}", &style)
}

/// Generates one repository's draft. Errors are per repository by construction:
/// this function never touches another repository (FR-008, Principle V).
pub async fn generate_for_repo(request: DraftRequest<'_>) -> Result<CommitDraft, CommitGenError> {
    let credential = request
        .credential
        .ok_or_else(|| CommitGenError::MissingCredential {
            env_var: request.settings.api_key_env.clone(),
        })?;

    let model = OpenAIChatModel::new(request.settings.model.clone(), credential)
        .with_base_url(request.settings.base_url.clone());

    let limits = UsageLimits::new()
        .tool_calls(request.settings.max_tool_calls)
        // Hard backstop: an unparseable answer must never loop forever
        // (edge case "malformed provider output", "run must not hang").
        .requests(request.settings.max_tool_calls.saturating_add(6));
    let budget = request.settings.max_tool_calls;

    let mut builder = AgentBuilder::<RepoCtx, String>::new(model)
        .system_prompt(build_system_prompt(&request.ctx, request.change_detail))
        .output_type::<CommitDraft>()
        .output_validator_fn(move |draft: CommitDraft, _ctx: &RunContext<RepoCtx>| {
            validate_draft(draft, budget)
        })
        .usage_limits(limits)
        // exact lookup accounting: a parallel batch is charged up front by the framework
        .parallel_tool_calls(false)
        .max_output_retries(MAX_OUTPUT_RETRIES);

    for (definition, executor) in crate::commitgen::tools::tool_executors() {
        builder = builder.tool_with_executor(definition, executor);
    }
    let agent = builder.build();

    run_with_backoff(
        &agent,
        request.ctx.path.clone(),
        request.ctx.repo_name.clone(),
        request.settings.max_tool_calls,
        request.change_detail.prompt.clone(),
    )
    .await
}

/// Retries retryable provider failures with exponential backoff (research R8).
async fn run_with_backoff(
    agent: &serdes_ai::agent::Agent<RepoCtx, CommitDraft>,
    repo_path: std::path::PathBuf,
    repo_name: String,
    lookup_limit: u32,
    prompt: String,
) -> Result<CommitDraft, CommitGenError> {
    let backoff = ExponentialBackoff::new();
    let mut attempt = 0_u32;

    loop {
        attempt += 1;
        // Fresh dependencies (and a fresh lookup budget) per attempt.
        let deps =
            RepoCtx::new(repo_path.clone(), repo_name.clone()).with_lookup_limit(lookup_limit);
        match agent.run(prompt.clone(), deps).await {
            Ok(result) => {
                let draft = result.into_output().normalized();
                if draft.subject.trim().is_empty() {
                    return Err(CommitGenError::EmptyResult);
                }
                return Ok(draft);
            }
            Err(error) => {
                let mapped = map_run_error(error);
                let retryable = mapped.is_retryable_attempt() && mapped.error.is_retryable();
                if !retryable || attempt >= MAX_PROVIDER_ATTEMPTS {
                    return Err(mapped.into_error());
                }
                tokio::time::sleep(backoff.calculate_delay(attempt)).await;
            }
        }
    }
}

fn validate_draft(
    draft: CommitDraft,
    _budget: u32,
) -> std::result::Result<CommitDraft, OutputValidationError> {
    let draft = draft.normalized();
    match draft.validate() {
        Ok(()) => Ok(draft),
        Err(error) => Err(match error {
            DraftValidationError::UnknownType(value) => {
                OutputValidationError::field_failed("type", format!("unknown type {value:?}"))
            }
            DraftValidationError::EmptySubject => {
                OutputValidationError::field_failed("subject", "subject must not be empty")
            }
            DraftValidationError::SubjectTooLong(length) => OutputValidationError::field_failed(
                "subject",
                format!("subject is {length} characters, maximum is 72"),
            ),
            DraftValidationError::SubjectTrailingPeriod => {
                OutputValidationError::field_failed("subject", "subject must not end with a period")
            }
            DraftValidationError::BadScope(scope) => OutputValidationError::field_failed(
                "scope",
                format!("scope must be lowercase-hyphen, got {scope:?}"),
            ),
        }),
    }
}

/// Wrapper that distinguishes "retry the provider" from "report the row".
struct MappedError {
    error: CommitGenError,
    retry_attempt: bool,
}

impl MappedError {
    fn is_retryable_attempt(&self) -> bool {
        self.retry_attempt
    }

    fn into_error(self) -> CommitGenError {
        self.error
    }
}

fn map_run_error(error: AgentRunError) -> MappedError {
    match error {
        AgentRunError::Model(model_error) => MappedError {
            error: CommitGenError::Provider(model_error.to_string()),
            retry_attempt: model_error.is_retryable(),
        },
        AgentRunError::OutputValidationFailed(validation) => MappedError {
            error: CommitGenError::MalformedDraft(validation.to_string()),
            retry_attempt: false,
        },
        AgentRunError::NoOutput | AgentRunError::UnexpectedStop => MappedError {
            error: CommitGenError::EmptyResult,
            retry_attempt: true,
        },
        AgentRunError::UsageLimitExceeded(limits) => MappedError {
            error: CommitGenError::Model(limits.to_string()),
            retry_attempt: false,
        },
        AgentRunError::Tool(tool_error) => MappedError {
            error: CommitGenError::Io(tool_error.to_string()),
            retry_attempt: false,
        },
        AgentRunError::Serialization(serialization) => MappedError {
            error: CommitGenError::MalformedDraft(serialization.to_string()),
            retry_attempt: true,
        },
        other => MappedError {
            error: CommitGenError::Model(other.to_string()),
            retry_attempt: other.is_retryable(),
        },
    }
}

/// Reads the credential from the process environment at run start (FR-013).
/// The value never leaves this function: callers pass it straight to the model.
pub fn read_credential(settings: &AiSettings) -> Option<String> {
    std::env::var(&settings.api_key_env)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Backoff helper used by the tests below (and documented for the live test).
#[allow(dead_code)]
pub(crate) fn retry_delay(attempt: u32) -> Duration {
    ExponentialBackoff::new().calculate_delay(attempt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serdes_ai::core::{ModelResponse, ModelSettings};

    fn detail(prompt: &str) -> ChangeDetail {
        ChangeDetail {
            prompt: prompt.to_string(),
            style_subjects: vec!["feat(api): add the endpoint".to_string()],
            truncated: false,
            untracked_count: 0,
        }
    }

    fn ctx() -> RepoCtx {
        RepoCtx::new(PathBuf::from("/repo/api"), "api")
    }

    fn settings() -> AiSettings {
        AiSettings {
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen3".to_string(),
            api_key_env: "REPOMAN_TEST_KEY".to_string(),
            max_tool_calls: 12,
            diff_budget_chars: 40_000,
        }
    }

    #[test]
    fn prompt_template_is_pinned_by_a_golden_file() {
        let golden = include_str!("testdata/system_prompt.golden");
        assert_eq!(SYSTEM_PROMPT, golden);
    }

    #[test]
    fn rendered_prompt_names_the_repo_the_style_and_the_schema() {
        let rendered = build_system_prompt(&ctx(), &detail("whatever"));
        assert!(rendered.contains("api"));
        assert!(rendered.contains("- feat(api): add the endpoint"));
        assert!(rendered.contains("\"maxLength\": 72"));
        assert!(rendered.contains("feat, fix, refactor"));
        assert!(rendered.contains("lookup budget"));
    }

    #[test]
    fn rendered_prompt_handles_a_repository_without_history() {
        let empty = ChangeDetail {
            prompt: "x".into(),
            style_subjects: Vec::new(),
            truncated: false,
            untracked_count: 0,
        };
        let rendered = build_system_prompt(&ctx(), &empty);
        assert!(rendered.contains("no previous commits"));
    }

    #[test]
    fn validator_accepts_a_wellformed_draft_and_normalizes_it() {
        let draft = CommitDraft {
            commit_type: "Feat".into(),
            scope: Some("API".into()),
            subject: "add the endpoint".into(),
            body: None,
        };
        let accepted = validate_draft(draft, 12).unwrap();
        assert_eq!(accepted.commit_type, "feat");
        assert_eq!(accepted.scope.as_deref(), Some("api"));
    }

    #[test]
    fn validator_rejects_a_schema_violation_with_a_repairable_message() {
        let draft = CommitDraft {
            commit_type: "bugfix".into(),
            scope: None,
            subject: "fix it.".into(),
            body: None,
        };
        let error = validate_draft(draft, 12).unwrap_err();
        assert!(error.to_string().contains("unknown type"));
    }

    #[tokio::test]
    async fn missing_credential_is_reported_before_any_request() {
        std::env::remove_var("REPOMAN_TEST_KEY");
        let error = generate_for_repo(DraftRequest {
            settings: &settings(),
            credential: None,
            ctx: Arc::new(ctx()),
            change_detail: &detail("diff"),
        })
        .await
        .unwrap_err();
        assert_eq!(
            error,
            CommitGenError::MissingCredential {
                env_var: "REPOMAN_TEST_KEY".to_string()
            }
        );
        assert_eq!(error.user_reason(), "missing-credential:REPOMAN_TEST_KEY");
    }

    #[tokio::test]
    async fn parses_a_structured_answer_from_a_stub_model() {
        let model =
            serdes_ai::models::FunctionModel::new(|_requests, _settings: &ModelSettings| {
                ModelResponse::text(
                    r#"{"type":"feat","scope":"api","subject":"add the help command","body":null}"#,
                )
            });
        let agent = AgentBuilder::<RepoCtx, String>::new(model)
            .system_prompt("stub")
            .output_type::<CommitDraft>()
            .build();

        let result = agent.run("diff", ctx()).await.expect("stub run");
        assert_eq!(
            result.into_output().render(),
            "feat(api): add the help command"
        );
    }

    /// Manual gate: `OPENAI_API_KEY=… cargo test --all-features -- --ignored`.
    #[tokio::test]
    #[ignore = "live provider: needs OPENAI_API_KEY (and optionally REPOMAN_LIVE_BASE_URL / REPOMAN_LIVE_MODEL)"]
    async fn live_provider_returns_a_schema_valid_draft() {
        let Some(credential) = std::env::var("OPENAI_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        else {
            return;
        };

        let settings = AiSettings {
            base_url: std::env::var("REPOMAN_LIVE_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            model: std::env::var("REPOMAN_LIVE_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".to_string()),
            api_key_env: "OPENAI_API_KEY".to_string(),
            max_tool_calls: 4,
            diff_budget_chars: 4_000,
        };
        let change =
            detail("--- composer.json ---\n- \"foo/bar\": \"^1.0\"\n+ \"foo/bar\": \"^1.2\"\n");
        let ctx = Arc::new(RepoCtx::new(PathBuf::from("/tmp/live-repo"), "live-repo"));

        let draft = generate_for_repo(DraftRequest {
            settings: &settings,
            credential: Some(credential),
            ctx,
            change_detail: &change,
        })
        .await
        .expect("live draft");

        assert!(!draft.commit_type.trim().is_empty());
        assert!(!draft.subject.trim().is_empty());
        assert!(draft.subject.chars().count() <= crate::commitgen::draft::MAX_SUBJECT_CHARS);
    }

    #[tokio::test]
    async fn a_schema_violation_makes_the_agent_re_ask_and_then_succeed() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let attempts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&attempts);
        let model = serdes_ai::models::FunctionModel::new(move |_, _| {
            let attempt = counter.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                // Unknown Conventional Commit type — the validator must reject it.
                ModelResponse::text(r#"{"type":"bugfix","subject":"repair the thing"}"#)
            } else {
                ModelResponse::text(r#"{"type":"fix","subject":"repair the thing"}"#)
            }
        });
        let agent = AgentBuilder::<RepoCtx, String>::new(model)
            .system_prompt("stub")
            .output_type::<CommitDraft>()
            .output_validator_fn(|draft: CommitDraft, _ctx: &RunContext<RepoCtx>| {
                validate_draft(draft, 12)
            })
            .max_output_retries(2)
            .build();

        let result = agent
            .run("diff", ctx())
            .await
            .expect("second answer is valid");
        assert_eq!(result.into_output().render(), "fix: repair the thing");
        assert_eq!(
            attempts.load(Ordering::SeqCst),
            2,
            "the model was re-asked once"
        );
    }

    #[test]
    fn unusable_provider_answers_map_to_typed_errors_not_panics() {
        assert_eq!(
            map_run_error(AgentRunError::NoOutput).into_error(),
            CommitGenError::EmptyResult
        );
        assert_eq!(
            map_run_error(AgentRunError::UnexpectedStop).into_error(),
            CommitGenError::EmptyResult
        );
        let provider = map_run_error(AgentRunError::Provider("offline".into()));
        assert_eq!(
            provider.into_error(),
            CommitGenError::Model("Provider error: offline".into())
        );
    }

    #[test]
    fn lookup_budget_is_claimed_atomically_and_capped() {
        let ctx = RepoCtx::new(PathBuf::from("/repo/theme"), "theme");
        for expected in 0..12 {
            assert!(ctx.claim_lookup(12), "lookup {expected} must be allowed");
        }
        assert_eq!(ctx.lookups_used(), 12);
        assert!(!ctx.claim_lookup(12), "the 13th lookup must be refused");
        assert_eq!(ctx.lookups_used(), 12);
    }

    #[test]
    fn reasons_never_leak_a_credential() {
        let error = CommitGenError::MissingCredential {
            env_var: "OPENAI_API_KEY".to_string(),
        };
        assert_eq!(error.user_reason(), "missing-credential:OPENAI_API_KEY");
        assert!(error.is_retryable());

        for reason in [
            CommitGenError::NothingToSummarize.user_reason(),
            CommitGenError::Conflicts.user_reason(),
        ] {
            assert!(!reason.contains("sk-"), "{reason}");
        }
        assert!(!CommitGenError::NothingToSummarize.is_retryable());
        assert!(!CommitGenError::Conflicts.is_retryable());
    }
}
