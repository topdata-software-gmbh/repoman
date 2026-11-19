# Feature Specification: Personal Wording Preference — Plain Language vs Git Terminology

**Feature Branch**: `005-plain-git-wording`

**Created**: 2026-11-19

**Status**: Draft

**Input**: User description: "Let's have a setting to switch between 'git jargon' and 'mac user language'. These simplified names confuse me as a git power user, but Sandra gets confused by git jargon instead." Derived from a design discussion: two independent axes (language and wording), plain as the default and source of truth, a small terminology overlay, short labels only, a personal device-local preference, and a control next to the existing language choice.

## Clarifications

### Session 2026-11-19

- Q: In German Git wording, should labels use canonical English Git command words or German-translated jargon? → A: Canonical English Git words in both languages; the German plain form stays German.
- Q: Which operation concepts must get a Git form — every operation the app exposes or only the five named? → A: Every operation concept the app currently exposes that has a Git counterpart (not just the five named).
- Q: Must the plain-mode Git term be revealed to keyboard and assistive-technology users, or is hover enough? → A: Reveal it on keyboard focus and announce it to assistive technology, not hover-only.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Keep plain words, still see the real Git term (Priority: P1)

A non-technical user keeps the app's plain wording (Save to Git, Get latest,
Send changes) and can reveal the corresponding Git term when they want to learn
it or talk to a developer, without changing the interface.

**Why this priority**: Preserves the beginner-first voice that the product is
built on, while making the gap to real Git terminology visible instead of
hidden. This alone delivers value with no risk to existing users.

**Independent Test**: With no stored preference, open the app and confirm every
operation label is plain; hover or focus an operation control and confirm the Git
term appears alongside the existing explanation, and that assistive technology
announces it.

**Acceptance Scenarios**:

1. **Given** no stored preference, **When** the user opens the app, **Then** all
   operation labels are in plain language and no Git jargon is shown as a label.
2. **Given** plain wording, **When** the user points at "Send changes", **Then**
   the hint explains what happens and also names the Git term "push".
3. **Given** plain wording, **When** the user reads the glossary, **Then** the
   glossary entries use the plain names.

---

### User Story 2 - A Git power user switches to real terminology (Priority: P2)

A Git-experienced user switches the wording preference to Git terminology and
immediately sees Commit, Fetch, Pull, Push and Stash on every surface where an
operation is named.

**Why this priority**: Removes the daily friction for the second audience. It
depends on the same mechanism as Story 1 and is independently demonstrable.

**Independent Test**: Switch to Git terminology and verify that every operation
label, confirmation message, dialog title and glossary name uses Git terms, with
no reload.

**Acceptance Scenarios**:

1. **Given** plain wording, **When** the user selects Git terminology, **Then**
   all operation labels update immediately without restarting the app.
2. **Given** Git terminology, **When** the user triggers an operation, **Then**
   the confirmation message names the same operation with the same Git term as
   the control that started it.
3. **Given** Git terminology, **When** the user opens the glossary, **Then** the
   glossary entries use the Git names.

---

### User Story 3 - The preference is personal, persistent and language-independent (Priority: P3)

The choice belongs to the person and the device, survives restarts, does not
follow the user's projects or the shared configuration, and combines freely with
German or English.

**Why this priority**: Makes the switch trustworthy and safe for mixed teams and
shared machines; it is what turns a cosmetic toggle into a usable preference.

**Independent Test**: Set a wording, restart the app, change the interface
language, and confirm both choices survive independently.

**Acceptance Scenarios**:

1. **Given** Git terminology and English, **When** the app is restarted, **Then**
   the app opens in Git terminology and English.
2. **Given** Git terminology, **When** the user switches the language to German,
   **Then** all labels are German Git terms and the wording choice is unchanged.
3. **Given** a stored wording value that is not recognised, **When** the app
   opens, **Then** it falls back to plain language.
4. **Given** a second device or browser, **When** the user opens the app there,
   **Then** that device's own wording preference applies and is not overwritten.

---

### Edge Cases

- A term has no meaningful Git counterpart (for example a branch name) → the
  label is identical in both modes and no misleading hint appears.
- A Git variant is missing for one language → the plain label is shown; no
  empty, truncated or placeholder label ever appears.
- The user switches wording while a confirmation dialog or a running operation
  is visible → labels update live and the operation itself is unaffected.
- The user switches wording and language in quick succession → every visible
  label reflects the final combination; no mixed-register or mixed-language
  leftovers.
- Explanatory sentences, hints, error guidance and the record of executed
  commands remain plain in Git mode.
- The control is reachable and operable by keyboard and assistive technology,
  and announces its current state.
- A first run with no stored preference, including browser access from another
  machine, starts in plain language.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Users MUST be able to choose between plain wording and Git
  terminology.
- **FR-002**: The default wording MUST be plain when no preference is stored.
- **FR-003**: The wording preference MUST persist across restarts on the same
  device and MUST NOT be shared through the project's configuration or follow
  the user's repositories or groups.
- **FR-004**: Wording MUST be independent of the interface language; every
  combination of German/English and plain/Git MUST be coherent.
- **FR-005**: In plain wording, operation labels MUST be plain and their hints
  MUST also name the corresponding Git term. The hint MUST be reachable by
  keyboard focus and announced to assistive technology, not only on mouse hover.
- **FR-006**: In Git wording, operation labels MUST use the Git term — the
  canonical English Git command word in every supported language — and the
  existing explanations MUST remain unchanged.
- **FR-007**: Every operation concept the app currently exposes that has a Git
  counterpart MUST use one consistent term on all surfaces where it is named,
  including the control, its confirmation or failure message, related dialog
  titles, and the glossary.
- **FR-008**: Explanatory copy, hints, error guidance, and the record of
  executed commands MUST NOT be reworded by the preference.
- **FR-009**: Switching wording MUST take effect immediately without a reload.
- **FR-010**: If a Git variant is unavailable for the active language, the
  system MUST show the plain label and MUST NOT show an empty or placeholder
  label.
- **FR-011**: Terms without a Git counterpart MUST render identically in both
  modes and MUST NOT gain a terminology hint.
- **FR-012**: The wording control MUST be discoverable next to the existing
  language choice, MUST be labelled, and MUST be operable by keyboard and
  assistive technology, exposing its current state.
- **FR-013**: Status badges that name an operation state (for example unsaved
  changes or changes waiting to be sent) MUST follow the selected wording, while
  remaining short labels.
- **FR-014**: The preference MUST NOT alter Git output, commit contents, branch
  names, file names, or any data sent to repositories.

### Key Entities *(include if feature involves data)*

- **Wording preference**: The person's choice between plain and Git terminology.
  Belongs to a person and a device; independent of projects, repositories, groups
  and interface language.
- **Terminology entry**: A short operation label with a plain form and an
  optional Git form. The plain form is language-specific; the Git form is the
  same canonical English Git command word in every language. Explanatory text
  is explicitly outside this entity.
- **Language preference**: The existing German/English choice, with which wording
  combines freely.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A Git-experienced user can switch to Git terminology and see
  consistent terms on every operation surface in under 30 seconds, without
  restarting the app.
- **SC-002**: After switching, 100% of operation concepts show the same term in
  their control and in the confirmation or failure message they produce.
- **SC-003**: Returning to plain wording restores every label exactly; zero
  mixed-register labels remain visible.
- **SC-004**: The preference survives an app restart in 100% of attempts on the
  same device.
- **SC-005**: Across all four language-and-wording combinations, zero operation
  labels are empty, truncated to a placeholder, or untranslated.
- **SC-006**: A first-time non-technical user completes a save or send operation
  with no additional steps compared with today, and sees plain wording by
  default.

## Assumptions

- Plain wording is the default and the authoritative copy; no automatic
  detection of the user's experience level is attempted.
- The preference is stored per device and browser, matching how the interface
  language already behaves; it is not part of the shared project configuration.
- Scope is limited to short labels: operation names, short status badges,
  confirmation titles and glossary names. Explanations, hints and error guidance
  stay plain by design.
- The interface already offers a language choice in a persistent place, and the
  wording control can live beside it without introducing a new settings area.
- The record of executed commands and raw Git output remain technical and are
  not translated or reworded.
- German and English remain the only supported languages for this feature.
