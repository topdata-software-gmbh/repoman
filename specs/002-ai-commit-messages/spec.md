# Feature Specification: AI-Generated Commit Messages

**Feature Branch**: `002-ai-commit-messages`

**Created**: 2026-11-01

**Status**: Draft

**Input**: User description: "Add a button for creating commit messages with AI. In the Save Changes dialog that commits several repositories at once, one click generates one Conventional-Commit message per selected repository from its pending changes, shown for review and editing before anything is committed. The generator may inspect the repository (files, history) when the change set alone is ambiguous, within a bounded budget. Model/provider configured via app settings with the credential read from the environment; failures of one repository never block the others."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generate per-repo messages, review, then commit (Priority: P1)

A user selects several dirty repositories and opens the Save Changes dialog. Instead of typing one shared message by hand, they click "Generate with AI". Every selected repository gets its own proposed Conventional Commit message derived from that repository's actual pending changes. The proposals appear in an editable preview list — one row per repository — and the user reviews, edits any of them, and then triggers the commit. Nothing is committed by the generation step itself.

**Why this priority**: This is the core value of the feature: accurate, per-repository messages with zero manual typing for the common case. Without review-then-commit it is unusable as a trustworthy feature.

**Independent Test**: Configure a model, select two repositories with different pending changes, generate, and verify two distinct draft messages appear; edit one and commit; confirm each repository receives exactly its (possibly edited) message.

**Acceptance Scenarios**:

1. **Given** two selected repositories with different pending changes and a configured model, **When** the user clicks the AI generate button, **Then** each repository receives its own draft message (not one shared message) in Conventional Commit form (`type(scope): subject`, optional body).
2. **Given** drafts are displayed, **When** the user edits one draft and confirms the commit, **Then** the edited text is what is committed for that repository.
3. **Given** drafts are displayed, **When** the user closes the dialog without confirming, **Then** no repository has been committed.
4. **Given** a repository with pending changes, **When** its draft is generated, **Then** the message subject describes that repository's own changes, not another repository's.

---

### User Story 2 - Concurrent generation with live per-repo status (Priority: P2)

With several repositories selected, generation starts for all of them at the same time. Each row shows its own state — generating, ready, or failed — and a row becomes editable as soon as its draft arrives, without waiting for slower repositories.

**Why this priority**: Batch use is the primary repoman scenario; sequential waiting for the slowest repository makes the feature feel broken. Status feedback is required for the user to trust the process.

**Independent Test**: Select three repositories where one reliably takes noticeably longer; verify the other two rows become editable while the third still shows "generating".

**Acceptance Scenarios**:

1. **Given** N selected repositories, **When** generation is triggered, **Then** all N generations are in flight simultaneously and each row displays a generating state.
2. **Given** one generation finishes while others are still running, **When** the result arrives, **Then** that row becomes editable immediately without waiting for the rest.
3. **Given** the commit confirmation requires a message for every selected repository, **When** any row is still generating or failed with no message, **Then** committing is blocked until every row has a non-empty message (draft, edited, or manually typed).

---

### User Story 3 - Isolated failure and single-repo retry (Priority: P2)

If generating the message for one repository fails (provider unreachable, model error, empty result), only that row shows the failure with a retry affordance. The user can retry that single repository; all other drafts remain intact and usable.

**Why this priority**: Network/model failures are routine; losing the whole batch because one repository failed would discard good results and erode trust.

**Independent Test**: Generate for three repositories with an invalid model name configured; all three show failed; fix configuration; retry one row and confirm only that row is regenerated.

**Acceptance Scenarios**:

1. **Given** a generation failure for one repository, **When** the failure is displayed, **Then** the row offers a retry and shows a human-readable reason.
2. **Given** a failed row and completed sibling rows, **When** the user retries the failed row, **Then** only that repository is regenerated; the siblings' drafts and edits are untouched.
3. **Given** a failed row, **When** the user types a message manually into it, **Then** it becomes eligible for commit like any ready row.

---

### User Story 4 - Repository inspection for ambiguous changes (Priority: P3)

When the pending change set alone does not explain the change (e.g. a renamed symbol used differently across the codebase, a version bump without context), the generator may inspect the repository: read current file contents, view a file at the previous state, review recent commit messages, and search the code. This inspection is bounded by a per-run limit on the number of lookups so cost and duration stay predictable.

**Why this priority**: Quality differentiator modeled on the proven CLI precedent; the feature ships without it but messages are shallower for large or abstract diffs.

**Independent Test**: For a repository whose diff is a one-line dependency bump, verify the generated message references the actual dependency (information available only via inspection), and verify that the number of lookups performed never exceeds the configured limit.

**Acceptance Scenarios**:

1. **Given** a change set too ambiguous for a confident summary, **When** a draft is generated, **Then** the generator may perform bounded repository lookups (file contents, prior file state, recent history, text search) before answering.
2. **Given** a per-run lookup limit (default 12), **When** the limit is reached, **Then** the generator returns its best-effort draft instead of continuing to search.
3. **Given** any lookup request, **When** the target path lies outside the repository being summarized, **Then** the lookup is refused and does not leak content.

---

### User Story 5 - Graceful degradation without configuration (Priority: P3)

A user without AI settings (or without the referenced environment credential) sees the generate control present but disabled, with a short hint on what is missing. Nothing else in the Save Changes dialog changes behavior; manual messages work as before.

**Why this priority**: The app must remain fully usable for users who never enable AI, and misconfiguration must be diagnosable without crashes.

**Independent Test**: Remove the AI section from the configuration; open the dialog; verify the control is disabled with a hint and the manual flow commits normally.

**Acceptance Scenarios**:

1. **Given** no AI configuration, **When** the Save Changes dialog opens, **Then** the generate control is disabled and a hint explains that AI is not configured.
2. **Given** AI configured but the referenced environment credential is absent at click time, **When** the user triggers generation, **When** the result is displayed, **Then** every row shows a clear "credential missing" failure and manual entry still works.
3. **Given** the UI languages German and English, **When** the dialog is rendered in either, **Then** all new control labels, states, and hints appear in that language.

---

### Edge Cases

- **No pending changes** in a selected repository: generation for that row fails or is skipped with an explanatory message; the user can still type one manually.
- **Very large change sets** (e.g. vendor check-ins): the change summary sent for generation is truncated at a configurable size limit so requests stay bounded; quality may degrade gracefully, the run must not fail or hang.
- **Merge-conflict state**: repository is not committed; generation reports the conflict as a non-retryable-with-AI issue.
- **Sensitive content**: pending changes may contain secrets; the environment credential is never written to disk by this feature, and generated output is never logged with the credential.
- **App closed mid-generation**: nothing has been committed (generation never writes repository state); reopening simply loses the in-flight drafts.
- **Same message expected across repos**: intentionally NOT provided — batch commits get individual messages; users can copy text manually.
- **Provider returns malformed/blank message**: treated as a failure with retry, never committed silently.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Save Changes dialog MUST provide a single control that starts message generation for all currently selected repositories.
- **FR-002**: The system MUST produce one draft message per selected repository from that repository's own pending changes (staged, unstaged, and untracked content), never one shared message for the batch.
- **FR-003**: Every draft MUST follow Conventional Commit structure (`feat:`/`fix:`/… with optional scope, a subject, and optional body) and align its style with the repository's recent commit subjects when available.
- **FR-004**: The system MUST show all drafts in a review list before committing; no repository may be committed as a consequence of generation.
- **FR-005**: Users MUST be able to edit any draft and add or replace text in any row before confirming the commit; confirmed commits use the per-row final text.
- **FR-006**: Commit confirmation MUST require a non-empty message for every selected repository (unchanged existing rule), regardless of whether the text was AI-generated or typed.
- **FR-007**: Generation for all selected repositories MUST run concurrently, and the UI MUST report per-repository state: idle, generating, ready, failed.
- **FR-008**: A failed generation MUST affect only its own row, MUST display a human-readable reason, and MUST offer retry of that single repository.
- **FR-009**: The system MUST bound the amount of repository detail sent for one generation by a configurable size limit, truncating larger change sets rather than failing.
- **FR-010**: The generator MUST be able to consult the repository for additional context — current content of a file, a file's content at the previous revision, recent commit messages, and a bounded text search — subject to a configurable per-run lookup budget (default 12).
- **FR-011**: All repository lookups performed on behalf of generation MUST be restricted to the repository being summarized; paths outside it MUST be refused.
- **FR-012**: AI usage MUST be configured in the application's own settings (provider endpoint, model, credential reference, lookup budget, change-detail size limit); when configuration is absent the feature MUST be visibly unavailable, not error.
- **FR-013**: The access credential MUST be read from the environment at request time and MUST NOT be stored in application settings or output by the app in logs or UI.
- **FR-014**: All new labels, states, hints, and errors MUST be provided in the app's supported UI languages (German and English).
- **FR-015**: Re-triggering generation for a repository MUST replace only that repository's current draft text (whether generated or manually typed); drafts of all other rows MUST remain unchanged.

### Key Entities

- **Draft Message**: The proposal for one repository: type, optional scope, subject, optional body, plus originating repository and created timestamp. Editable by the user before commit.
- **Generation Row State**: Per-repository UI state through the flow: idle → generating → ready (with draft) or failed (with reason, retryable).
- **AI Settings**: Configuration record naming provider endpoint, model identifier, credential environment-variable reference, per-run lookup budget, and change-detail size limit. Optional; its absence disables the feature.
- **Repository Change Set**: The full set of changes that would be committed for one repository (staged, unstaged, untracked), the input the draft is derived from.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For a batch of N selected repositories, total generation wall time is approximately the time of the slowest single repository, not N times the average (concurrency verified).
- **SC-002**: 100% of committed messages equal either the exact generated draft or a user edit of it — the generation step never commits on its own.
- **SC-003**: On a test batch of 10 real dirty repositories with a configured model, at least 7 of 10 drafts are committed with no edits or only minor edits (≤ subject wording), demonstrating the feature removes manual typing in the common case.
- **SC-004**: A single repository's generation failure never prevents drafts for the remaining repositories from completing or being committed.
- **SC-005**: Users with no AI configuration experience zero behavior change in the Save Changes dialog apart from a visibly disabled control with an explanatory hint.
- **SC-006**: No credential value appears anywhere in application config files, logs, or UI text in any scenario.

## Assumptions

- The Save Changes dialog already commits the full working-tree change set of each selected repository; generation summarizes exactly what that commit would contain.
- "Previous revision" means the repository's current HEAD; the generator never needs access to other repositories or the user's home directory.
- The user provides their own access to a hosted or local model endpoint that speaks a mainstream chat/tool-calling API; selecting and provisioning that provider is outside this feature.
- The existing CLI precedent (super-bin `sb gsum`) is the behavioral reference for prompt style, lookup budget, and output format, but repoman's batch-per-repo UI has no equivalent and is new.
- Lookup-budget default (12) and change-detail size default (~40,000 characters) mirror the proven CLI values; both are configuration-adjustable.
- The model may be unavailable or slow; per-row failure plus manual entry is an acceptable complete fallback.
- Out of scope for this feature: committing/pushing driven by the generator itself, learning message style beyond recent history, multi-model fallback chains, a settings UI for AI configuration (editing the settings file is sufficient for v1).
