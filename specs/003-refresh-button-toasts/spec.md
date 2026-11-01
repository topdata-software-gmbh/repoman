# Feature Specification: Refresh Button Feedback Toasts

**Feature Branch**: `003-refresh-button-toasts`

**Created**: 2026-11-01

**Status**: Draft

**Input**: Design handoff `_ai/backlog/active/261101_0159__DESIGN_HANDOFF__refresh-button-toasts.md` — "Show success and error toasts after a manual per-row repository refresh."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See that a manual refresh succeeded (Priority: P1)

A user clicks the refresh button on a project row. When the project was
refreshed successfully, a success toast appears naming the project and showing
its current state summary, so the user knows the click was registered and can
see the fresh status without hunting through the row.

**Why this priority**: The core complaint is that the refresh button gives no
feedback. Without a success confirmation the feature delivers no value.

**Independent Test**: Click refresh on an accessible project and verify one
success toast appears with the project name in the title and a state summary in
the message.

**Acceptance Scenarios**:

1. **Given** an accessible project row, **When** the user clicks refresh and the
   refresh succeeds, **Then** a success toast appears whose title names the
   project and whose message summarizes its branch and status labels.
2. **Given** a project on branch `main` with unsaved changes, **When** its
   refresh succeeds, **Then** the toast message reads like
   `main · Änderungen nicht gespeichert` (the branch label followed by every
   status label, separated by ` · `).
3. **Given** a clean, up-to-date project on branch `main`, **When** its refresh
   succeeds, **Then** the toast message reads like `main · Auf dem neuesten Stand`.

---

### User Story 2 - See that a manual refresh failed (Priority: P1)

A user clicks refresh on a project whose folder is gone or whose refresh fails
for another reason. An error toast appears naming the project and stating the
first line of the failure, so the user knows the refresh did not happen.

**Why this priority**: Silent failure is the second half of the reported
problem; a success-only toast would be misleading.

**Independent Test**: Click refresh on a project that is inaccessible, then on a
project whose backend call rejects, and verify both produce an error toast with
the project name in the title and the failure reason in the message.

**Acceptance Scenarios**:

1. **Given** a project whose folder no longer exists (the refresh returns it as
   inaccessible), **When** the user clicks refresh, **Then** an error toast
   appears titled with the project name and using the existing
   "folder not found" wording as its message.
2. **Given** a refresh call that fails at the transport/backend level, **When**
   the user clicks refresh, **Then** an error toast appears with the project
   name in the title and the first line of the raw error as its message.
3. **Given** an error toast is shown, **When** the user looks at the row, **Then**
   the row still reflects the last known repository data (a failed refresh never
   removes the project from the list).

---

### User Story 3 - Background refreshes stay quiet (Priority: P2)

Operations that refresh repositories without a direct user click on the refresh
button — live file-change events, post-pull/push/commit/branch-switch refreshes
— do not produce refresh toasts. Their existing pull/push/commit feedback
remains the only notification, so the user is not spammed.

**Why this priority**: Prevents a regression that would make the app noisy and
duplicate existing operation toasts; required for the feature to be shippable.

**Independent Test**: Trigger a repository-changed event and a pull; verify no
"refreshed" toast is produced by the internal refresh step, and that the normal
pull toast still appears exactly once.

**Acceptance Scenarios**:

1. **Given** the app receives a live repository-changed event, **When** it
   refreshes that repository in the background, **Then** no refresh toast is
   shown.
2. **Given** a pull/push/commit/branch-switch completes, **When** its internal
   refresh step runs, **Then** no refresh toast is added; only the existing
   operation toast is shown.
3. **Given** a background refresh fails, **When** the failure is handled, **Then**
   no toast is shown and no unhandled error escapes.

---

### Edge Cases

- The user clicks refresh while another refresh is already running: the existing
  per-row spinner guard still prevents concurrent clicks; at most one toast per
  accepted click.
- The repository row is not yet in the list when its refresh resolves: the
  refreshed project is still added to the list and the toast still names it.
- The error text is empty or only whitespace: the message falls back to the
  shared "unknown error" wording rather than an empty toast body.
- The project is accessible but has no remotes: the summary still renders the
  branch label plus the "local only" status label.
- The app language is German or English: both title and summary are fully
  translated.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Clicking the per-row refresh button MUST produce exactly one toast
  for that click: success when the project is refreshed and accessible, error
  otherwise.
- **FR-002**: A success toast title MUST contain the project name, using the
  wording "`{name} aktualisiert`" (German) / "`{name} refreshed`" (English).
- **FR-003**: A success toast message MUST be the project's branch label
  followed by every status label for that project, joined with ` · `, reusing
  the same labels the row displays.
- **FR-004**: An error toast title MUST contain the project name, using the
  wording "`{name} konnte nicht aktualisiert werden`" (German) /
  "`Could not refresh {name}`" (English).
- **FR-005**: An error toast message for an inaccessible project MUST use the
  existing "folder not found" label.
- **FR-006**: An error toast message for a thrown/transport failure MUST be the
  first line of the raw error text, trimmed; when empty it MUST fall back to the
  existing "unknown error" wording.
- **FR-007**: The shared internal refresh operation MUST NOT emit toasts; only
  the button-triggered path may emit them.
- **FR-008**: The shared internal refresh operation MUST NOT throw on failure;
  it MUST report failure to its caller so background callers stay safe.
- **FR-009**: A refresh failure MUST NOT remove or invalidate the existing row
  data.
- **FR-010**: All user-facing toast copy MUST be added to both the German and
  English translation dictionaries; no literals in components.

### Key Entities

- **Repository**: A project shown as a row; has a name, path, accessibility
  flag, and state (branch, clean/dirty, ahead/behind, conflicts, remotes).
- **Refresh Outcome**: The result of refreshing one repository — either the
  updated repository (success) or an error description (failure).
- **Toast**: A transient notification with a kind (success/error), a title, and
  an optional message.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of accepted refresh-button clicks produce exactly one toast
  (success or error).
- **SC-002**: 0 refresh toasts are produced by background/live-update refreshes
  or by the internal refresh step of pull, push, commit, and branch-switch.
- **SC-003**: A user can tell from the toast alone whether a project refreshed
  successfully, and if not, why (project name plus reason).
- **SC-004**: All new copy is present and correct in both German and English.

## Assumptions

- The existing toast infrastructure (toast stack, store action, i18n keys) is
  reused unchanged; no visual redesign or new toast component is needed.
- The refresh operation is already available to the frontend and its behavior
  does not change on the backend.
- The row's existing branch and status label helpers remain the single source
  of truth for the success summary.
- Live-update, pull, push, commit, and branch-switch notification behavior is
  out of scope and must not change.
