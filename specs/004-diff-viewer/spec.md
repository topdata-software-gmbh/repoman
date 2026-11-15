# Feature Specification: Read-Only Change Viewer

**Feature Branch**: `004-diff-viewer`

**Created**: 2026-11-14

**Status**: Draft

**Input**: User description: "Add a diff viewer similar to JetBrains products, but made for a web designer who does not like Git and finds it too complicated. One shared, read-only viewer with two modes: review a project's unsaved changes before saving, and review what was saved last time. Show a plain-language file list, before/after comparison, code coloring and image previews, and never expose raw Git jargon or destructive actions."

## Clarifications

### Session 2026-11-15

- Q: Where should the viewer draw the line between "previewable" and "too large," and which image formats count as "supported"? → A: Text preview capped at the first 2 MB or 5,000 diff lines (whichever comes first); supported images are PNG, JPEG, GIF, WebP, and SVG up to 10 MB and 8,000×8,000 px.
- Q: What level of keyboard and screen-reader support must the viewer meet? → A: Keyboard fully operable with visible focus and screen-reader labels/roles, meeting WCAG 2.1 AA within the viewer.
- Q: When exactly should a file be labelled *Konflikt*? → A: Only when the repository reports the file as unmerged during an in-progress merge, rebase, or cherry-pick.
- Q: Which changes count as "unsaved changes" when the viewer opens? → A: All uncommitted work: modified, deleted, or renamed files (staged or unstaged) plus untracked files; ignored files excluded.
- Q: What should happen when a project has far more changes than the 50-file, 2-second target? → A: No hard cap; the 2-second target applies up to 50 files, and larger projects still open with the same bounded previews.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Review one project's unsaved changes (Priority: P1)

A shop operator edits files in one project and wants to check what she is about to save before pressing save. She opens "Änderungen ansehen" on that project's row. A large panel shows the project name, how many files changed, and a running total of added and removed lines. On the left she sees a list of files described in plain words (*Neu*, *Geändert*, *Gelöscht*, *Umbenannt*, *Konflikt*). Selecting a file shows the previous and current version side by side, with changed lines clearly tinted and unchanged stretches folded away. She closes the panel with *Fertig*. Nothing is saved, reverted, or staged by looking.

**Why this priority**: This is the reassurance loop that motivates the feature. Without it, the rest is decoration.

**Independent Test**: On a project with at least one modified, one new, and one deleted file, open the viewer from the row, confirm all three appear with the correct plain-language label, open each, verify before/after content and tinting, and close. Confirm the project's files on disk are unchanged.

**Acceptance Scenarios**:

1. **Given** a project with unsaved changes, **When** she opens the viewer from its row, **Then** she sees the project name, a file count, added/removed totals, and every changed file labelled in plain words.
2. **Given** the viewer is open on a modified file, **When** she selects a file, **Then** the previous and current versions appear side by side with changed lines tinted and unchanged regions collapsed.
3. **Given** the viewer is open, **When** she closes it with *Fertig* or Escape, **Then** it closes and no project file or saved state has changed.
4. **Given** a project with no unsaved changes, **When** she opens the viewer, **Then** it shows an empty state saying there is nothing to review.

---

### User Story 2 - Review several projects before saving them together (Priority: P1)

She has edited three projects and opens the shared Save Changes dialog. Before saving, she picks "Überprüfen" and the viewer opens over all selected projects, starting with the first. A compact line reads *"Projekt 2 von 4"* with previous/next; switching projects keeps her place in each. She reviews each project, then closes and saves as usual.

**Why this priority**: The bulk-save flow is where an error is most costly, so it is the second half of the core value.

**Independent Test**: Select three projects with differing changes, open review from Save Changes, step through all three with previous/next, verify each project's own files and summary appear, return to the first, and confirm the previously selected file is still selected.

**Acceptance Scenarios**:

1. **Given** several projects are selected in Save Changes, **When** she opens review, **Then** the viewer lists those projects and starts on the first with a *"Projekt N von M"* indicator.
2. **Given** the viewer is showing one project, **When** she moves to the next or previous project, **Then** that project's file list and summary load and the position indicator updates.
3. **Given** she reviewed project two and returns to it later, **Then** her previously selected file in that project is still selected.
4. **Given** one selected project cannot be read, **When** she reaches it, **Then** only that project shows a plain-language error state and the other projects remain reviewable.

---

### User Story 3 - See what was saved last time (Priority: P2)

After saving, the activity panel "Was Git gemacht hat" lists the save. She clicks "Ansehen" on that entry and sees exactly what was included in that save, using the same viewer. If the save is no longer available, the panel tells her so in plain language instead of failing silently.

**Why this priority**: Reassurance after the fact; valuable, but the pre-save review already covers the riskiest moment.

**Independent Test**: Make a save, open the activity panel, click "Ansehen" on that save's entry, verify each file included in the save appears with its before/after comparison, and close. Separately verify that an entry whose saved state is unavailable shows the inline notice.

**Acceptance Scenarios**:

1. **Given** a successful save, **When** she opens the activity panel and chooses "Ansehen" on that save, **Then** the viewer shows the files and differences included in that save, read-only.
2. **Given** the saved state can no longer be found, **When** she chooses "Ansehen", **Then** a plain-language notice appears in the viewer saying the change is no longer available.

---

### User Story 4 - Understand images and non-text changes (Priority: P2)

Her projects contain images. Selecting an image file shows the previous and current image side by side with pixel dimensions and file size, so she can judge the change visually. Files that cannot be previewed are still listed with a clear description of what happened to them.

**Why this priority**: Web work is heavily visual; a viewer that only understands code would miss a large share of her changes.

**Independent Test**: Change an image and a non-previewable file, open the viewer, confirm the image shows before/after with dimensions and size, and confirm the other file shows a clear *"Binärdatei geändert"* description with a size change.

**Acceptance Scenarios**:

1. **Given** a changed supported image, **When** she selects it, **Then** the previous and current image appear side by side with dimensions and file size.
2. **Given** an image larger than the preview limit, **When** she selects it, **Then** she sees a plain-language notice that the image is too large to preview instead of a broken or endless load.
3. **Given** a changed file that is neither text nor a supported image, **When** she selects it, **Then** it shows *"Binärdatei geändert"* with its size change and no preview.

---

### User Story 5 - Switch to a technical view (Priority: P3)

A developer borrowing the machine, or the operator following written instructions, can switch the comparison to a single-column or raw technical form, and back, without leaving the viewer.

**Why this priority**: Power convenience; the beginner-friendly view must remain the default and is fully useful without it.

**Independent Test**: Open any text change, switch to the technical form, verify the same change is represented there, then switch back and confirm the friendly view returns with the same file selected.

**Acceptance Scenarios**:

1. **Given** the friendly side-by-side view, **When** she enables the technical view, **Then** the same change is shown in a raw single-column form and the default view can be restored.
2. **Given** the technical view is enabled, **When** she changes files, **Then** the chosen display mode is preserved for the rest of the review.

---

### Edge Cases

- **Project with no saved history yet**: every file is treated as *Neu*; no previous version is shown.
- **Deleted file**: only the previous version is shown, with a *Gelöscht* label.
- **Renamed file**: shown as *Umbenannt*, comparing the old path's content with the new path's content.
- **Conflicts**: a file the repository reports as unmerged during a merge, rebase, or cherry-pick is labelled *Konflikt*; it remains viewable and the viewer never blocks on it.
- **Whitespace-only change**: the file is shown fully but carries a *"Nur Leerzeichen geändert"* label so an apparently empty change is explained.
- **Very large text file**: the first portion is shown with a clear notice that the rest is not displayed; the viewer never hangs.
- **Unreadable or missing project**: only that project shows an error state naming the project and what to do next.
- **Unsupported or corrupted image**: falls back to a plain description rather than a broken preview.
- **Changes arrive while the viewer is open**: the content does not jump; a quiet *"Änderungen haben sich geändert · Aktualisieren"* prompt lets her refresh on demand.
- **Last-save entry that can no longer be resolved**: an inline notice explains it is no longer available.
- **No changes at all**: an empty state, not an error.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The viewer MUST be strictly read-only. It MUST NOT save, stage, discard, revert, or otherwise modify project files or history from any control within it.
- **FR-002**: The viewer MUST open on a project's unsaved changes from a project row action. Unsaved changes are all uncommitted work — modified, deleted, or renamed files whether staged or unstaged, plus untracked files — excluding ignored files.
- **FR-003**: The viewer MUST open over all projects selected in the shared Save Changes dialog, and MUST provide previous/next navigation between them with a *"Projekt N von M"* indicator.
- **FR-004**: The viewer MUST support a second mode that shows the differences included in the most recent save of a project, opened from that save's entry in the activity panel.
- **FR-005**: The viewer MUST show, for the current project, the project name, the number of changed files, and the total added and removed lines.
- **FR-006**: The file list MUST label each file in plain language as one of *Neu*, *Geändert*, *Gelöscht*, *Umbenannt*, or *Konflikt*, and MUST NOT expose single-letter status codes. A file MUST be labelled *Konflikt* only when the repository reports it as unmerged during an in-progress merge, rebase, or cherry-pick.
- **FR-007**: For text files the viewer MUST show the previous and current content side by side by default, with changed lines visually distinguished and unchanged stretches collapsed; collapsed stretches MUST be expandable.
- **FR-008**: The viewer MUST offer an alternative unified (single-column) display and a raw technical display, selectable without leaving the viewer, with the friendly side-by-side view as the default.
- **FR-009**: Text code MUST be colorized by language in a way that keeps added/removed tinting dominant.
- **FR-010**: Supported image changes MUST show a before/after preview with dimensions and file size. Supported formats are PNG, JPEG, GIF, WebP, and SVG, up to 10 MB and 8,000×8,000 pixels; images beyond that bound MUST show a clear notice instead of a preview.
- **FR-011**: Files that are neither previewable text nor supported images MUST be listed as *"Binärdatei geändert"* with a size change.
- **FR-012**: Files whose only change is whitespace MUST be displayed fully and labelled *"Nur Leerzeichen geändert"*.
- **FR-013**: The viewer MUST treat a project that cannot be read as an isolated error state naming the project, without preventing review of other projects in the same session.
- **FR-014**: The viewer MUST show an empty state when there are no changes to review.
- **FR-015**: While the viewer is open, newly arriving changes MUST NOT replace the displayed comparison automatically; the viewer MUST instead offer an explicit *Aktualisieren* prompt.
- **FR-016**: Very large text files MUST be limited to the first 2 MB or 5,000 diff lines, whichever comes first, with a clear notice that the rest is not displayed; the viewer MUST never appear to hang.
- **FR-017**: Each save shown in the activity panel MUST be associated with a way to open its differences in the viewer, and an unavailable saved state MUST produce a plain-language inline notice.
- **FR-018**: Every user-facing string MUST be translated in both German and English; German is the primary voice and no Git jargon may appear in the default view.
- **FR-019**: The viewer MUST be closable with a *Fertig* control and with Escape; it MUST contain no action that could be mistaken for saving or discarding.
- **FR-020**: Requesting to review a single project MUST show that project immediately; requesting several MUST start on the first project without requiring a separate selection step.
- **FR-021**: The viewer MUST be fully operable by keyboard with visible focus, and MUST expose screen-reader labels and roles for the panel, file list, project navigation, and collapsed-stretch controls, meeting WCAG 2.1 AA within the viewer.
- **FR-022**: The viewer MUST NOT impose a hard cap on the number of changed files; the 2-second first-file target applies up to 50 changed files, and larger projects MUST still open with the same bounded previews and notices.

### Key Entities *(include if feature involves data)*

- **Review session**: One open viewer instance. Attributes: mode (unsaved changes or last save), the ordered set of projects being reviewed, the current project, and the currently selected file per project.
- **Project comparison**: The reviewable state of one project. Attributes: project identity, availability (ready, unreadable, empty), the list of changed files, and the added/removed/file-count summary.
- **File comparison**: One changed file within a project. Attributes: path, plain-language status, kind (text, image, binary), whether the change is whitespace-only, change statistics, and the previous and current content or an explanation of why content cannot be shown.
- **Saved-state reference**: A pointer from an activity-panel save entry to the state it recorded. Attributes: project, save message, time, and whether the referenced state is still available.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a usability check with five people who do not use Git, at least four can open the viewer and correctly answer "what changed in this project?" without help.
- **SC-002**: For a project with up to 50 changed files, the first file's changes are visible within 2 seconds of opening the viewer on local disk.
- **SC-003**: The default view of a change contains zero occurrences of Git shorthand (`A`, `M`, `D` status codes, `@@`, `HEAD`, commit hashes).
- **SC-004**: 100% of visible viewer strings exist in both German and English with no missing translations.
- **SC-005**: A review of any kind results in zero modifications to project files or repository history (verified before/after).
- **SC-006**: Moving between projects in a multi-project review takes exactly one action per hop, and returning to a project restores its previously selected file.
- **SC-007**: Every defined edge case (empty project, unreadable project, deleted/renamed file, conflict, oversized image, oversized text file, unavailable last save) reaches a labelled state with no unhandled error dialog.
- **SC-008**: At least 90% of reviewed sessions end with the user closing the viewer via *Fertig* or Escape rather than navigating away or reloading, indicating the flow is understandable.
- **SC-009**: 100% of interactive viewer controls are reachable and operable by keyboard, show a visible focus indicator, and have an accessible name, with no WCAG 2.1 AA violations detected in the viewer.

## Assumptions

- The primary user is a non-technical web designer who treats Git as a save button; German is the default language and English is required for parity.
- The viewer is read-only by deliberate product choice; saving and any future reverting remain in existing, separate flows.
- A full commit history browser is out of scope for this version; only the most recent save per project is reviewable from the activity panel.
- Projects are local repositories already managed by the app and reachable by the same discovery and save paths that exist today.
- The supported image types and preview bounds recorded in FR-010 and FR-016 are product defaults chosen to protect performance; they may be tuned later without changing the user experience described here.
- Code colorization and display-mode preferences are conveniences; the feature remains fully useful if they are unavailable.
- Notifications about newly arriving changes are advisory; the viewer deliberately never refreshes content underneath the user.
