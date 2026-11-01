//! The structured draft the model must produce (data-model E2, research R2).
//!
//! The LLM framework at the pinned version does not send a JSON schema to the
//! provider, so the shape is enforced twice: spelled out in the system prompt
//! (see `json_schema_hint`) and re-checked by a validator that makes the agent
//! re-ask the model (T021/T022).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A Conventional Commit proposal for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitDraft {
    /// Conventional Commit type, serialized on the wire as `type`.
    #[serde(rename = "type")]
    pub commit_type: String,
    /// Optional component scope, lowercase-hyphen.
    #[serde(default)]
    pub scope: Option<String>,
    /// Non-empty, ≤ 72 chars after trim, no trailing period.
    pub subject: String,
    /// Optional prose explaining *why*.
    #[serde(default)]
    pub body: Option<String>,
}

/// Wire form used by the event bus and the store: camelCase with the
/// backend-rendered text precomputed (data-model E2, contracts §C2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDraftWire {
    #[serde(rename = "type")]
    pub commit_type: String,
    pub scope: Option<String>,
    pub subject: String,
    pub body: Option<String>,
    pub rendered: String,
}

/// Maximum subject length after trimming (data-model E2, schema-validated).
pub const MAX_SUBJECT_CHARS: usize = 72;

/// A schema violation the model gets a chance to repair (FR-003).
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DraftValidationError {
    #[error("type must be one of feat|fix|refactor|perf|docs|style|test|build|ci|chore|revert, got {0:?}")]
    UnknownType(String),
    #[error("subject must not be empty")]
    EmptySubject,
    #[error("subject must be at most {MAX_SUBJECT_CHARS} characters, got {0}")]
    SubjectTooLong(usize),
    #[error("subject must not end with a period")]
    SubjectTrailingPeriod,
    #[error("scope must be lowercase-hyphen, got {0:?}")]
    BadScope(String),
}

impl CommitDraft {
    /// Normalizes the fields the way the schema requires, without rejecting
    /// anything: trims, lower-cases the scope, drops an empty scope to `None`.
    pub fn normalized(mut self) -> Self {
        self.commit_type = self.commit_type.trim().to_ascii_lowercase();
        self.scope = Self::normalize_scope(self.scope);
        self.subject = collapse_spaces(self.subject.trim());
        self.body = self
            .body
            .as_ref()
            .map(|body| body.trim().to_string())
            .filter(|body| !body.is_empty());
        self
    }

    fn normalize_scope(scope: Option<String>) -> Option<String> {
        let raw = scope?.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return None;
        }
        let normalized = collapse_spaces(&raw.replace(['_', ' '], "-"));
        let normalized: String = normalized
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let normalized = normalized.trim_matches('-').to_string();
        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    /// Validates the (already normalized) draft against data-model E2.
    pub fn validate(&self) -> Result<(), DraftValidationError> {
        if !super::CONVENTIONAL_COMMIT_TYPES.contains(&self.commit_type.as_str()) {
            return Err(DraftValidationError::UnknownType(self.commit_type.clone()));
        }
        let subject = self.subject.trim();
        if subject.is_empty() {
            return Err(DraftValidationError::EmptySubject);
        }
        if subject.chars().count() > MAX_SUBJECT_CHARS {
            return Err(DraftValidationError::SubjectTooLong(subject.chars().count()));
        }
        if subject.ends_with('.') {
            return Err(DraftValidationError::SubjectTrailingPeriod);
        }
        if let Some(scope) = &self.scope {
            if !scope
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                return Err(DraftValidationError::BadScope(scope.clone()));
            }
        }
        Ok(())
    }

    /// Renders the final commit message (research R2): `type(scope): subject`,
    /// then a blank line and the body when present. The rendered string — not
    /// the struct — seeds the row textarea and reaches `commit_repo_changes`.
    pub fn render(&self) -> String {
        let head = match &self.scope {
            Some(scope) => format!("{}({}): {}", self.commit_type, scope, self.subject),
            None => format!("{}: {}", self.commit_type, self.subject),
        };
        match &self.body {
            Some(body) if !body.trim().is_empty() => format!("{head}\n\n{}", body.trim()),
            _ => head,
        }
    }

    /// Backend-computed wire form (single renderer keeps transports identical).
    pub fn to_wire(&self) -> CommitDraftWire {
        CommitDraftWire {
            commit_type: self.commit_type.clone(),
            scope: self.scope.clone(),
            subject: self.subject.clone(),
            body: self.body.clone(),
            rendered: self.render(),
        }
    }

    /// JSON schema spelled into the system prompt (the framework cannot send it).
    pub fn json_schema_hint() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "type": { "type": "string", "enum": super::CONVENTIONAL_COMMIT_TYPES },
                "scope": { "type": ["string", "null"], "description": "lowercase-hyphen component, or null" },
                "subject": { "type": "string", "maxLength": MAX_SUBJECT_CHARS, "description": "imperative, no trailing period" },
                "body": { "type": ["string", "null"], "description": "optional prose explaining why, or null" }
            },
            "required": ["type", "subject"]
        })
    }
}

impl CommitDraftWire {
    /// Round-trips back into the typed draft.
    pub fn to_draft(&self) -> CommitDraft {
        CommitDraft {
            commit_type: self.commit_type.clone(),
            scope: self.scope.clone(),
            subject: self.subject.clone(),
            body: self.body.clone(),
        }
    }
}

fn collapse_spaces(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(commit_type: &str, scope: Option<&str>, subject: &str) -> CommitDraft {
        CommitDraft {
            commit_type: commit_type.to_string(),
            scope: scope.map(str::to_string),
            subject: subject.to_string(),
            body: None,
        }
    }

    #[test]
    fn accepts_every_documented_type() {
        for commit_type in super::super::CONVENTIONAL_COMMIT_TYPES {
            let candidate = draft(commit_type, None, "do the thing").normalized();
            assert!(candidate.validate().is_ok(), "{commit_type} must be valid");
        }
        assert_eq!(super::super::CONVENTIONAL_COMMIT_TYPES.len(), 11);
    }

    #[test]
    fn rejects_unknown_types_even_after_casing() {
        let candidate = draft("Feature", None, "do the thing").normalized();
        assert_eq!(
            candidate.validate(),
            Err(DraftValidationError::UnknownType("feature".into()))
        );
    }

    #[test]
    fn enforces_subject_rules() {
        let empty = draft("feat", None, "   ").normalized();
        assert_eq!(empty.validate(), Err(DraftValidationError::EmptySubject));

        let trailing = draft("feat", None, "do the thing.").normalized();
        assert_eq!(
            trailing.validate(),
            Err(DraftValidationError::SubjectTrailingPeriod)
        );

        let exactly_72 = "a".repeat(72);
        assert!(draft("feat", None, &exactly_72)
            .normalized()
            .validate()
            .is_ok());

        let too_long = "a".repeat(73);
        assert_eq!(
            draft("feat", None, &too_long).normalized().validate(),
            Err(DraftValidationError::SubjectTooLong(73))
        );
    }

    #[test]
    fn counts_subject_length_in_characters_not_bytes() {
        let umlauts = "ä".repeat(72);
        assert!(draft("fix", None, &umlauts).normalized().validate().is_ok());
        let too_many = "ä".repeat(73);
        assert!(matches!(
            draft("fix", None, &too_many).normalized().validate(),
            Err(DraftValidationError::SubjectTooLong(73))
        ));
    }

    #[test]
    fn normalizes_scope_to_lowercase_hyphen() {
        let candidate = draft("feat", Some("  Save_Modal  "), "do it").normalized();
        assert_eq!(candidate.scope.as_deref(), Some("save-modal"));

        let dots = draft("feat", Some("a..b"), "do it").normalized();
        assert_eq!(dots.scope.as_deref(), Some("ab"));

        let blank = draft("feat", Some("   "), "do it").normalized();
        assert_eq!(blank.scope, None);
    }

    #[test]
    fn renders_with_and_without_scope_and_body() {
        assert_eq!(
            draft("feat", Some("modal"), "add AI commit message generation").render(),
            "feat(modal): add AI commit message generation"
        );
        assert_eq!(draft("fix", None, "avoid crash").render(), "fix: avoid crash");

        let with_body = CommitDraft {
            body: Some("  Because the modal lost drafts.  ".to_string()),
            ..draft("fix", None, "keep drafts on close")
        };
        assert_eq!(
            with_body.render(),
            "fix: keep drafts on close\n\nBecause the modal lost drafts."
        );
    }

    #[test]
    fn wire_form_carries_the_rendered_text_and_uses_type_key() {
        let wire = draft("chore", Some("ci"), "pin serdesai").normalized().to_wire();
        let json = serde_json::to_value(&wire).unwrap();
        assert_eq!(json["type"], "chore");
        assert_eq!(json["scope"], "ci");
        assert_eq!(json["rendered"], "chore(ci): pin serdesai");

        let parsed: CommitDraftWire =
            serde_json::from_value(json).expect("wire round trip is deserializable");
        assert_eq!(parsed.to_draft().render(), "chore(ci): pin serdesai");
    }

    #[test]
    fn deserializes_provider_shaped_json() {
        let parsed: CommitDraft = serde_json::from_str(
            r#"{"type":"refactor","scope":"modal","subject":"split the save dialog","body":null}"#,
        )
        .unwrap();
        assert_eq!(parsed.commit_type, "refactor");
        assert_eq!(parsed.scope.as_deref(), Some("modal"));
        assert_eq!(parsed.body, None);
        assert!(parsed.normalized().validate().is_ok());
    }

    #[test]
    fn schema_hint_lists_the_enum_and_the_limit() {
        let schema = CommitDraft::json_schema_hint();
        assert_eq!(
            schema["properties"]["type"]["enum"].as_array().unwrap().len(),
            11
        );
        assert_eq!(schema["properties"]["subject"]["maxLength"], 72);
        assert_eq!(schema["required"].as_array().unwrap().len(), 2);
    }
}
