# Feature Specification: Live Scan Progress & Activity Log

**Feature Branch**: `001-scan-progress-log`

**Created**: 2026-11-01

**Status**: Draft

**Input**: User description: "It should give a bit more feedback about the progress .. and maybe also writing which directory it is processing right now .. and some 'log view' of all found repos. Refresh shows a clickable spinner; clicking it opens a live 'Scan activity' modal (current dir + found count header, append-only log of found repos with type badges), and the same modal works during onboarding. On finish the modal shows a summary and stays open until closed."

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Watch a scan make progress during onboarding (Priority: P1)

A new user points RepoMan at a large folder (e.g. a whole development drive) during first-run onboarding. Instead of a silent spinner that reads like a hang, the user sees the app actively working: the directory being examined right now and a count of repositories found so far, both updating live.

**Why this priority**: This is the core complaint being fixed — long searches currently give zero feedback and look broken. Without this, nothing else matters.

**Independent Test**: Start an onboarding scan on a folder containing hundreds of directories and dozens of git repositories; verify the current directory text and the found-count change while the scan runs.

**Acceptance Scenarios**:

1. **Given** an onboarding scan of a large folder is running, **When** the search examines directories, **Then** the status line shows the directory currently being processed and updates at least twice per second.
2. **Given** an onboarding scan is running, **When** a new repository is discovered, **Then** the "found" count increases within one second of the discovery.
3. **Given** a scan is running, **When** the user looks away and back, **Then** both the current directory and the found count let them tell that the search is alive and roughly where it is.

---

### User Story 2 - Inspect the live activity log via a modal (Priority: P2)

While any scan runs (onboarding or a later refresh), the user clicks the "search running" indicator and a "Scan activity" modal opens: a header with the current directory, found count, and elapsed time, and below it a running log with one row per repository found in this scan, its location and (as soon as known) its type — Plugin, Theme, or other.

**Why this priority**: The user explicitly asked for a "log view of all found repos"; it turns opaque waiting into visible, checkable work, but it builds on the same progress stream as US1.

**Independent Test**: Start a scan, open the modal from the indicator, and verify rows appear live, append in discovery order, auto-scroll, and that type badges fill in shortly after each row appears.

**Acceptance Scenarios**:

1. **Given** any scan is running, **When** the user clicks the "search running" indicator, **Then** the Scan activity modal opens and immediately shows the progress so far.
2. **Given** the modal is open while a scan runs, **When** another repository is found, **Then** a new row appears at the bottom and the view follows it automatically.
3. **Given** a found repository's type is not yet determined, **When** determination completes, **Then** the existing row is updated in place with the type badge rather than a duplicate row appearing.
4. **Given** the modal is open during onboarding, **When** the user closes it, **Then** the onboarding flow continues uninterrupted and the modal can be reopened.

---

### User Story 3 - See the result and stay in control when the scan ends (Priority: P3)

When the scan finishes, the modal does not vanish with the answer: its header becomes a summary ("Scan complete — 47 repositories in 2:14"), the log stays readable until the user closes it, and the finished indicator remains clickable briefly. A failed or interrupted scan reports a clear error instead of spinning forever. Clicking Refresh on the main screen produces the same feedback loop, and found repositories flow into the normal project table as before.

**Why this priority**: Completion and failure handling make the feedback trustworthy; without US1/US2 there is nothing to finish, so it ranks last.

**Independent Test**: Complete a scan with the modal open, verify summary, elapsed time, and that the log stays until closed; then simulate an interrupted scan (e.g. remove a folder mid-scan) and verify the error state.

**Acceptance Scenarios**:

1. **Given** a scan completes, **When** it was still running when the modal was opened, **Then** the header changes to a summary with total count and elapsed time and the log remains readable until the user closes the modal.
2. **Given** a scan completes, **When** the modal is closed, **Then** the finished indicator stays available for a short window (~10 seconds) to reopen that run's summary, then disappears.
3. **Given** a scan fails or is interrupted, **When** the failure occurs, **Then** the modal/indicator shows a clear error message and the "running" state always terminates — the UI never waits indefinitely.
4. **Given** the user starts a new scan while an old (finished) run's log is still displayed, **When** the new scan begins, **Then** the log and counts reset so the modal always shows exactly one run.

---

### Edge Cases

- Scan of a very large tree (10,000+ repositories): the log display stays responsive — the newest entries are shown with an explicit "…and N more" notice, while header counts remain exact.
- Very long directory paths: the current-directory text is shortened in the middle and the full path remains available on hover.
- Multiple folders scanned in one refresh: log rows identify which root folder they came from, and the header names the root currently being processed.
- Scan finishes while the modal is open and the user has scrolled back through the log: auto-scroll stops for that user action and never yanks the view mid-read.
- Scan started from onboarding and the app window resized/re-focused: progress text and modal stay consistent (no lost or duplicated rows).
- The scan's underlying step transition (onboarding step change) is never affected: it still happens only after the scan fully finishes.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST, during any repository scan, show the directory currently being processed, updating continuously while the scan runs.
- **FR-002**: System MUST show the number of repositories found so far, incremented within one second of each discovery.
- **FR-003**: The "search running" indicator MUST be clickable whenever a scan is active and MUST open the Scan activity modal.
- **FR-004**: Scan activity modal MUST list one row per repository found in the current run, in discovery order, newest at the bottom, auto-following new rows unless the user has scrolled away.
- **FR-005**: Each log row MUST show the repository name and location, and MUST gain its type badge (Plugin / Theme / other) as soon as the type is determined, updating the existing row in place.
- **FR-006**: Modal header MUST show current directory, total found count, and elapsed time for the running scan.
- **FR-007**: On completion, modal MUST convert its header into a summary (total repositories, elapsed time) and MUST remain open and readable until the user closes it.
- **FR-008**: On failure or interruption, modal and indicator MUST present a clear error and MUST terminate the running state; no scan feedback may hang indefinitely.
- **FR-009**: Starting a new scan MUST reset the progress state so modal and counts always reflect exactly one scan run.
- **FR-010**: For refreshes covering multiple folders, every progress surface MUST attribute repositories to the root folder being processed.
- **FR-011**: System MUST offer the same live feedback in both entry flows — first-run onboarding scan and refresh from the main view — driven by a single progress stream.
- **FR-012**: All new user-facing text MUST be available in German and English via the existing language switch, with no untranslated strings.
- **FR-013**: With more than 500 repositories found in one run, the displayed log MUST cap at the newest 500 rows plus an "…and N more" notice, while the shown counts remain exact.
- **FR-014**: Long current-directory values MUST be shortened in the middle visually, with the full path available on hover.
- **FR-015**: After a scan completes with the modal closed, the indicator MUST remain clickable showing that run's summary for approximately 10 seconds before disappearing.

### Key Entities

- **Scan Run**: one execution of a repository search; has a start time, one or more root folders, a current directory, a found count, a state (running / complete / failed), and an optional error.
- **Found Repository Entry**: one discovered repository within a run; has name, location, originating root folder, discovery time, and an optional type (Plugin / Theme / other) resolved after discovery.
- **Progress Update**: a single live signal from the running scan carrying any of: current directory, newly found repository, resolved type of a found repository, run completion or failure.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Within 2 seconds of starting any scan, the user sees live feedback (current directory and/or found count), and that feedback changes at least twice per second thereafter for the rest of the scan.
- **SC-002**: During a scan of a folder with 1,000+ subfolders, a user can state, at any moment, which part of the tree the app is working on.
- **SC-003**: A user who opens the activity modal mid-scan sees every repository found so far, and no repository appears twice.
- **SC-004**: In a usability check with 5 users scanning a large directory, 0 report the app as "stuck or crashed" while a scan is running (today's baseline is the app looking frozen for up to a minute).
- **SC-005**: Adding progress feedback does not slow the scan itself: end-to-end scan duration grows by no more than 5% versus today's silent scan on the same folder.
- **SC-006**: 100% of failed or interrupted scans end with a visible error or cleared indicator — zero hanging "running" states in testing.

## Assumptions

- The repositories discovered by the scan still populate the normal project table exactly as today; the activity log is a view of the run, not a separate inventory.
- The activity log is kept only for the current application session (like the existing Git-action log); it is not persisted and has no history beyond the most recent run.
- The onboarding flow still advances to the grouping step only after the scan completes; this feature changes what is shown during the wait, not when the wait ends.
- Behavior applies equally to the desktop window and the browser view, reusing the existing notification channel shared by both.
- Type badges reflect the same repository-type rules the app already uses (Shopware plugin via composer manifest, theme via theme manifest, everything else "other").
