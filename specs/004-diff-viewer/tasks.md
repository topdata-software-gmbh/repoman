---
description: "Task list for the read-only change viewer"
---

# Tasks: Read-Only Change Viewer

**Input**: Design documents from `/specs/004-diff-viewer/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/review-commands.md, contracts/save-reference-and-activity.md, quickstart.md

**Organization**: Tasks are grouped by user story so each story can be implemented, tested, and delivered independently once the shared foundation is in place.

**Tests**: Research, plan, and the project constitution require tests for new behavior and error paths. Rust tests run in-module with `tempfile` Git fixtures; frontend tests use vitest + `@vue/test-utils`. Write each story's tests first and confirm they fail before implementing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US5 from spec.md; omitted for Setup, Foundational, and Polish
- Every task names the exact file it touches

## Path Conventions

Single Tauri workspace: Rust core in `src-tauri/src/`, Vue app in `src/`, feature docs in `specs/004-diff-viewer/`.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Declare dependencies and create the module/type skeletons every story builds on.

- [X] T001 Add `base64 = "0.22"` and `imagesize = "0.13"` to `[dependencies]` in `src-tauri/Cargo.toml` (no Git library; read-only previews only).
- [X] T002 Add `highlight.js = "^11"` to `dependencies` in `package.json` and run `pnpm install` so `pnpm-lock.yaml` records it.
- [X] T003 Declare `mod review;` in `src-tauri/src/main.rs` and create the module skeleton: `src-tauri/src/review.rs` plus `src-tauri/src/review/limits.rs`, `src-tauri/src/review/status.rs`, `src-tauri/src/review/text.rs`, `src-tauri/src/review/saved.rs`, `src-tauri/src/review/image.rs` (each declared from `review.rs`).
- [X] T004 Add the review types from data-model E2–E8 to `src/types/domain.ts`: `ReviewMode`, `ReviewOrigin`, `DisplayMode`, `ProjectComparison`, `ProjectComparisonState`, `DiffFileEntry`, `DiffFileStatus`, `DiffFileKind`, `DiffPreview`, `DiffLine`, `DiffRow`, `DiffFold`, `TextPreview`, `ImagePreview`, `BinaryPreview`, `SaveRef`, `ReviewError`; extend `OperationResult` with `runId: string | null` and `commitHash: string | null`.
- [X] T005 [P] Add the base `review.*` keys (`review.title`, `review.description`, `review.close`, `review.loading`, `review.loadingFile`) to `src/lib/i18n/messages.en.ts` and the matching German entries in `src/lib/i18n/messages.de.ts` (German is the primary voice).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared transport, read-only Git helpers, session slice, and viewer shell required by every story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 Extend `OperationResult` in `src-tauri/src/models.rs` with `#[serde(default)] pub run_id: Option<String>` and `#[serde(default)] pub commit_hash: Option<String>` under the existing `rename_all = "camelCase"` (additive only; existing fields keep meaning).
- [X] T007 [P] Add read-only helpers to `src-tauri/src/git_runner.rs`: `run_git_bytes` (raw stdout), `status_porcelain_z` (`git status --porcelain=v1 -z --untracked-files=all`), `diff_name_status_z` and `diff_numstat_z` (`git diff --name-status -z -M <base> <target>` / `--numstat -z -M`), `diff_unified0` (`--no-color --no-ext-diff --no-textconv --unified=0 -M`), `show_bytes`, `rev_parse`, `rev_list_parents`, `diff_quiet_ignore_space` (`--ignore-all-space --ignore-blank-lines --quiet`). No mutating subcommand.
- [X] T008 [P] Define the caps and truncation helpers in `src-tauri/src/review/limits.rs`: `TEXT_MAX_BYTES = 2_097_152` (2 MB), `TEXT_MAX_DIFF_LINES = 5_000`, `IMAGE_MAX_BYTES = 10_485_760` (10 MB), `IMAGE_MAX_DIMENSION = 8_000`; expose `truncation_reason(bytes, diff_lines) -> Option<'bytes' | 'lines'>` and `image_exceeds(bytes, width, height)`.
- [X] T009 [P] Define the serde DTOs in `src-tauri/src/review.rs` mirroring data-model E2–E8: `ProjectComparison`, `DiffFileEntry`, `FilePreview` (exactly one of `text`/`image`/`binary`), `DiffLine` (`DiffRow`/`DiffFold`), `SavedCommit`, `ReviewError`, request structs `ReviewProjectRequest`/`ReviewFileRequest`, and the enums for mode/state/status/kind/notice. Use `#[serde(rename_all = "camelCase")]` and omit-lowercase enum values matching the contracts.
- [X] T010 Add `pub async fn review_project(request: ReviewProjectRequest) -> Result<ProjectComparison, String>` and `pub async fn review_file(request: ReviewFileRequest) -> Result<FilePreview, String>` to `src-tauri/src/service.rs` as thin calls into `review.rs`; keep `path` validation (`GitRunner::resolve_git_dir`) and the "filePath must be in the inventory" rejection (`unknown-file`) in the core.
- [X] T011 Add thin delegating Tauri commands `review_project` and `review_file` to `src-tauri/src/commands/git_commands.rs`, registered through the existing `invoke_handler` in `src-tauri/src/main.rs`.
- [X] T012 Add `POST /api/repositories/review` and `POST /api/repositories/review-file` to `src-tauri/src/web.rs` with `ReviewProjectRequest`/`ReviewFileRequest` deserializers, delegating to `service::review_project` / `service::review_file`, inside the existing `require_token` route layer.
- [X] T013 Add `reviewProject(request)` and `reviewFile(request)` to the `backend` object in `src/lib/backend.ts`, wired for both Tauri (`review_project` / `review_file`) and HTTP (`/api/repositories/review` / `/api/repositories/review-file`).
- [X] T014 Add the session-only review slice to `src/stores/repoStore.ts`: `reviewOpen`, `reviewMode`, `reviewOrigin`, `reviewProjects`, `reviewActiveIndex`, `reviewDisplayMode`, `reviewComparisons`, `reviewPreviews`, `reviewSelectedFileByProject`, `reviewStaleProjects`, `reviewLoadingProject`, `reviewLoadingFile`, `reviewSaveRefs`, plus `openReview`, `closeReview`, and the exported state/actions. Closing clears viewer maps but keeps `reviewSaveRefs`.
- [X] T015 Add display helpers to `src/lib/review.ts`: status → i18n key (`new`→`Neu`, `modified`→`Geändert`, `deleted`→`Gelöscht`, `renamed`→`Umbenannt`, `conflict`→`Konflikt`), preview notice/error code → i18n key maps, and byte/line formatting helpers. This module never formats raw porcelain letters.
- [X] T016 Create `src/components/ReviewViewer.vue` as a `TdModal size="full"` shell with translated title, loading/empty/error placeholders, a *Fertig* button, and Escape close; create `src/components/reviews/ReviewNotice.vue` for reused empty/unreadable/unavailable/too-large notices; mount `<ReviewViewer />` in `src/App.vue`.
- [X] T017 [P] Add foundational Rust tests in `src-tauri/src/review/limits.rs` and `src-tauri/src/review.rs`: cap boundaries (2 MB, 5,000 lines, 10 MB, 8,000 px), camelCase serialization of the DTOs, and a `tempfile` fixture asserting a review leaves `git status --porcelain` and `HEAD` unchanged.
- [X] T018 [P] Add frontend tests in `src/lib/__tests__/review.test.ts` and `src/stores/__tests__/review-session.test.ts` for status/error mapping and open/close lifecycle (closing clears maps, save references survive).

**Checkpoint**: Foundation ready — the viewer can open an empty shell, both transports compile, and no layer outside `review`/`service` holds Git or diff logic.

---

## Phase 3: User Story 1 — Review one project's unsaved changes (Priority: P1) 🎯 MVP

**Goal**: Open the viewer from a project row, see every uncommitted change labelled in plain words, and compare text files side by side with folded unchanged stretches; nothing is modified.

**Independent Test**: On a project with one modified, one new, and one deleted file, open from the row, confirm the three labels, open each file to verify before/after content and tinting, expand a fold, close with *Fertig* and Escape, and confirm `git status --porcelain` and `HEAD` are unchanged.

### Tests for User Story 1 (write first, confirm they fail) ⚠️

- [X] T019 [P] [US1] Add parsing tests in `src-tauri/src/review/status.rs` with `tempfile` repos for modified/new/deleted/renamed/untracked entries, ignored files excluded, rename pairs carrying `oldPath`, and `conflict` labelled **only** when porcelain is unmerged **and** `detect_operation_state` is `merging`, `rebasing`, or `cherry-picking`.
- [X] T020 [P] [US1] Add diff tests in `src-tauri/src/review/text.rs` for `-U0` hunks producing `same`/`changed`/`added`/`removed` rows with old/new numbers, unchanged runs forming `fold` groups, deleted/new files having one null side, NUL bytes classified as binary, and truncation at the first of 2 MB or 5,000 diff lines.
- [X] T021 [P] [US1] Add `src/components/__tests__/review-file-list.test.ts` covering plain-language labels (no `M`/`A`/`D`), listbox roles, roving tabindex Arrow/Home/End, Enter/Space selection, `aria-selected`, and visible focus.
- [X] T022 [P] [US1] Add `src/components/__tests__/review-text-diff.test.ts` covering side-by-side rows, added/removed tinting plus text markers, collapsed folds, `aria-expanded` fold buttons, and expansion revealing the unchanged rows.
- [X] T023 [P] [US1] Add `src/stores/__tests__/review-session.test.ts` cases for opening from a row, loading one project summary, caching one preview per file, and the empty-project state.

### Implementation for User Story 1

- [X] T024 [P] [US1] Implement `src-tauri/src/review/status.rs`: parse `git status --porcelain=v1 -z --untracked-files=all` into `DiffFileEntry` rows (untracked → `new`, deleted → `deleted`, rename pairs → `oldPath` + `renamed`); apply the conflict rule from data-model E3 verbatim; classify `kind` from the numstat binary marker (`-`), a NUL-byte leading sample, and image extension; set `whitespaceOnly` only when the normal diff is non-empty and `git diff --ignore-all-space --ignore-blank-lines --quiet` is empty; compute `addedLines`/`removedLines` from numstat (binary/unknown contribute 0).
- [X] T025 [US1] Implement `src-tauri/src/review/text.rs`: load bounded old/new sides (`git show <base>:<path>` / bounded worktree read, empty side for additions/deletions), parse `-U0` hunks, emit `DiffRow`s and `DiffFold`s in document order, build a bounded `raw` unified string, and set `truncated` + `truncationReason` (`'bytes'` or `'lines'`).
- [X] T026 [US1] Implement unsaved-mode orchestration in `src-tauri/src/review.rs`: `review_project` returns the data-model E2 shape (`ready`/`empty`/`unreadable`; `summary` always present, zeros for non-ready; rename counts once; files ordered by case-insensitive display path), and `review_file` returns the text variant with language `null`/`plaintext` for now.
- [X] T027 [US1] Implement `openReviewFromRow` plus summary/preview fetch actions in `src/stores/repoStore.ts`, using `backend.reviewProject`/`reviewFile`, caching per project/file, and exposing loading flags and the empty state.
- [X] T028 [P] [US1] Implement `src/components/reviews/ReviewFileList.vue` as a `role="listbox"` with `role="option"` rows, plain-language labels, per-file added/removed counts, roving `tabindex`, Arrow/Home/End/Enter/Space handling, `aria-selected`, and visible focus.
- [X] T029 [US1] Implement `src/components/reviews/ReviewTextDiff.vue` side-by-side friendly mode: old/new columns, changed-row tinting via `bg-success-soft`/`bg-error-soft` plus `+`/`−` text markers and translated cell labels, and fold rows as `<button aria-expanded>` controls that reveal exactly the folded rows.
- [X] T030 [US1] Complete `src/components/ReviewViewer.vue` for one project: header with project name, file count, and added/removed totals; file-list + preview layout; loading, empty, and unreadable states; *Fertig* and Escape close; no control that stages, saves, discards, or reverts.
- [X] T031 [US1] Add the row action "Änderungen ansehen" in `src/components/RepoListTable.vue` that calls the store's row-review opener, including an accessible name and disabled handling for inaccessible projects.
- [X] T032 [US1] Add all US1 DE+EN keys (status labels, summary, empty state, file list labels, fold labels, row action) to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts`.
- [X] T033 [US1] Add a read-only integration test in `src-tauri/src/review.rs` that runs `review_project` and `review_file` against a `tempfile` fixture with modified/new/deleted files and asserts `git status --porcelain` and `git log -1 --format=%H` are byte-identical before and after.

**Checkpoint**: US1 is fully functional and independently testable; the viewer proves the read-only promise.

---

## Phase 4: User Story 2 — Review several projects before saving them together (Priority: P1)

**Goal**: Open the same viewer over every project selected in Save Changes, move with previous/next (*"Projekt N von M"*), and keep each project's selected file.

**Independent Test**: Select three projects with different changes, choose *Überprüfen* in Save Changes, step through all three, return to the first, and confirm its previously selected file is still selected; make one project unreadable and confirm only it shows an error.

### Tests for User Story 2 (write first, confirm they fail) ⚠️

- [X] T034 [P] [US2] Extend `src/stores/__tests__/review-session.test.ts` with multi-project cases: ordered/deduplicated projects, exactly one active-index change per previous/next hop, per-project selected-file restoration, and cached preview reuse.
- [X] T035 [P] [US2] Add `src/components/__tests__/review-viewer.test.ts` cases for the *"Projekt N von M"* polite status region, previous/next controls, one isolated unreadable project, and the stale-change *Aktualisieren* prompt not replacing content automatically.

### Implementation for User Story 2

- [X] T036 [US2] Extend the review slice in `src/stores/repoStore.ts` with `openReviewFromSaveChanges(paths)`, `nextReviewProject()`, `prevReviewProject()`, `setReviewSelectedFile(projectPath, filePath)`, per-project comparison/preview caches, and stale-project tracking; `activeIndex` must always index a valid project while open.
- [X] T037 [US2] Add the *Überprüfen* control to `src/components/SaveChangesModal.vue` that opens review over `repos.value` (the selected projects) without saving, leaving the dialog state intact underneath.
- [X] T038 [US2] Extend `src/components/ReviewViewer.vue` with previous/next project navigation, the translated *"Projekt N von M"* indicator in a polite status region, per-project loading, and an isolated error state naming only the unreadable project (FR-013, US2-AC4).
- [X] T039 [US2] Wire `repo:fs-changed` handling in `src/App.vue` / `src/stores/repoStore.ts` to mark only the open project stale, and render the translated *"Änderungen haben sich geändert · Aktualisieren"* prompt + refresh handler in `src/components/ReviewViewer.vue`; content must never be replaced automatically (FR-015).
- [X] T040 [US2] Add all US2 DE+EN keys (project navigation, position indicator, unreadable-project copy, stale prompt, refresh action) to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts`.

**Checkpoint**: Both P1 stories work; the bulk-save review is deliverable.

---

## Phase 5: User Story 3 — See what was saved last time (Priority: P2)

**Goal**: Open a save entry from the activity drawer and review exactly the files and differences included in that commit; explain unavailable saved states in plain language.

**Independent Test**: Make a save, use *Ansehen* on its drawer entry, verify the commit's files and before/after content; make a second save and confirm the first entry still opens the first commit; simulate a garbage-collected hash and confirm the unavailable notice.

### Tests for User Story 3 (write first, confirm they fail) ⚠️

- [X] T041 [P] [US3] Add resolution tests in `src-tauri/src/review/saved.rs` with `tempfile` repos: parent..commit inventory, root commit diffing against `4b825dc642cb6eb9a060e54bf8d69288fbee4904`, unresolvable hash → `unavailable`/`save-not-found`, unborn `HEAD` → `unavailable`/`head-not-found`.
- [X] T042 [P] [US3] Extend `src/stores/__tests__/review-session.test.ts` to assert a `SaveRef` is stored only for successful results carrying both `runId` and `commitHash`, failed commits create none, and the drawer fallback uses current `HEAD`.

### Implementation for User Story 3

- [X] T043 [P] [US3] Add saved-state helpers to `src-tauri/src/git_runner.rs`: `rev_parse_verify_commit` (`rev-parse --verify <hash>^{commit}`), `show_commit_subject_and_time` (`show -s --format=%s%x00%ct`), `rev_list_parents`, post-commit `rev_parse_head`, and the empty-tree constant.
- [X] T044 [US3] Implement save-reference capture in `src-tauri/src/git_runner.rs` / `src-tauri/src/service.rs`: on a successful `commit_changes`, read `git rev-parse HEAD` and return `commitHash` with the run's `runId`; on failure leave both `None`; perform no extra write and no history rewrite.
- [X] T045 [US3] Implement `src-tauri/src/review/saved.rs`: resolve the requested `commitHash` (absent = `HEAD`), verify it, read subject/time, find the parent or empty-tree fallback, and return the resolved `<base>..<target>` pair plus `SavedCommit`; map failures to the stable codes `save-not-found` / `head-not-found`.
- [X] T046 [US3] Implement saved-mode orchestration in `src-tauri/src/review.rs`: `review_project` returns `state: "unavailable"` with the stable error code when resolution fails and otherwise builds the inventory from the same resolved pair used for every file; `review_file` returns `notice: "missing"` when a summarized file is absent from the resolved commit.
- [X] T047 [US3] Implement the frontend save-reference path: add `reviewSaveRefs` capture in `commitRepositories` in `src/stores/repoStore.ts`, and add the translated *Ansehen* action for `macroId === "commit"` runs in `src/components/GitLogDrawer.vue` with the captured hash, current-`HEAD` fallback, and drawer state preserved behind the modal.
- [X] T048 [US3] Add all US3 DE+EN keys (saved-review title, saved metadata labels, unavailable notice, *Ansehen*) to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts`.

**Checkpoint**: Post-save review works and every save entry either resolves or explains itself.

---

## Phase 6: User Story 4 — Understand images and non-text changes (Priority: P2)

**Goal**: Show supported images before/after with dimensions and size, and describe every other non-text change as *"Binärdatei geändert"* with a size change.

**Independent Test**: Change a supported image and a binary file, select each, confirm the image shows before/after with dimensions/size and the binary shows *"Binärdatei geändert"* with its size change; select an oversized or corrupt image and confirm a plain notice.

### Tests for User Story 4 (write first, confirm they fail) ⚠️

- [X] T049 [P] [US4] Add `src-tauri/src/review/image.rs` tests: PNG/JPEG/GIF/WebP dimensions via `imagesize`, SVG dimensions from `width`/`height` or `viewBox`, the **10 MB and 8,000×8,000 px** bounds checked before encoding, corrupt/unsupported bytes → notice state, and a missing/deleted side → `null`.
- [X] T050 [P] [US4] Add `src/components/__tests__/review-image-diff.test.ts` covering side-by-side images with dimensions and byte sizes, the too-large/corrupt notice, a missing side, and the binary *"Binärdatei geändert"* + size-change state.

### Implementation for User Story 4

- [X] T051 [US4] Implement `src-tauri/src/review/image.rs`: treat a file as an image only for PNG/JPEG/GIF/WebP/SVG whose bytes parse; read raster dimensions with `imagesize`; parse SVG `width`/`height`/`viewBox`; check bounds before base64-encoding; return `data:` URLs with `{ dataUrl, width, height, bytes }` per side or a `too-large`/`corrupt`/`unsupported`/`missing` state.
- [X] T052 [US4] Extend `src-tauri/src/review.rs` file orchestration: choose the image variant for supported images, return `binary.oldBytes`/`binary.newBytes` with `notice: "unsupported"` for other non-text files, and keep exactly one of `text`/`image`/`binary` present matching `kind`.
- [X] T053 [P] [US4] Implement `src/components/reviews/ReviewImageDiff.vue`: before/after images with pixel dimensions and file sizes, a plain too-large/corrupt notice instead of a broken image, and only the available side for added/deleted files.
- [X] T054 [US4] Extend `src/components/ReviewViewer.vue` to render the image and binary preview variants; binary renders *"Binärdatei geändert"* with old→new byte size and no preview.
- [X] T055 [US4] Add all US4 DE+EN keys (image preview labels, dimensions, size, too-large/corrupt/unsupported, binary-changed) to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts`.

**Checkpoint**: Visual and non-text changes are reviewable without breaking the friendly default.

---

## Phase 7: User Story 5 — Switch to a technical view (Priority: P3)

**Goal**: Offer unified single-column and raw technical forms alongside the friendly side-by-side default, with syntax colouring that never overrides add/remove tinting.

**Independent Test**: Open a text change, switch to unified then raw and back to side-by-side, confirm the same change appears in each form, and confirm the chosen mode persists while changing files.

### Tests for User Story 5 (write first, confirm they fail) ⚠️

- [X] T056 [P] [US5] Add tests in `src/lib/__tests__/review.test.ts` for unified-row derivation in document order, raw presentation selection (`text.raw` when available, rows when truncated), and extension → highlight.js language selection with a `plaintext` fallback.
- [X] T057 [P] [US5] Add `src/components/__tests__/review-text-diff.test.ts` cases for unified and raw rendering, the accessible mode toggle, and display-mode preservation across file changes.

### Implementation for User Story 5

- [X] T058 [P] [US5] Implement `src/lib/highlight.ts` using `highlight.js/lib/core` plus an explicit language registry (JavaScript, TypeScript, Vue, HTML/XML, CSS/SCSS, JSON, PHP, Rust, Bash, YAML, Markdown) with `plaintext` fallback and token-only `.hljs-*` styling; no full-bundle import and no raw palette colours.
- [X] T059 [US5] Implement unified/raw helpers and language selection in `src/lib/review.ts` from the same `DiffLine` rows, so all display modes derive from one core-produced source of truth.
- [X] T060 [US5] Extend `src/components/reviews/ReviewTextDiff.vue` with accessible side-by-side/unified/raw switching, syntax-highlighted code in all modes, and row tinting that remains dominant.
- [X] T061 [US5] Persist `reviewDisplayMode` in `src/stores/repoStore.ts` and honour it in `src/components/ReviewViewer.vue` across file and project changes for the rest of the session, defaulting to `sideBySide`.
- [X] T062 [US5] Add all US5 DE+EN keys (display-mode labels, technical-view warning/description) to `src/lib/i18n/messages.en.ts` and `src/lib/i18n/messages.de.ts`.

**Checkpoint**: All five stories are independently functional; the default remains beginner-friendly.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, accessibility verification, and the constitution's quality gates.

- [X] T063 [P] Update `README.md` with the review feature summary and the two new HTTP routes, and add a user-visible entry to `CHANGELOG.md` in the same change.
- [X] T064 [P] Perform a keyboard/screen-reader audit of `src/components/ReviewViewer.vue`, `src/components/reviews/ReviewFileList.vue`, `src/components/reviews/ReviewTextDiff.vue`, and `src/components/reviews/ReviewImageDiff.vue` against WCAG 2.1 AA (focus visible, names/roles, fold `aria-expanded`, polite project position) and fix any gaps.
- [X] T065 [P] Verify the i18n parity gate (`src/lib/__tests__/i18n.test.ts`) and the token-only styling gate (`src/lib/__tests__/token-only.test.ts`) pass with every new viewer component; remove any raw palette class or hex colour.
- [X] T066 Run every scenario in `specs/004-diff-viewer/quickstart.md` (US1–US5, all edge cases including >50 files, 2 MB/5,000-line and 10 MB/8,000 px caps, stale prompt, unavailable save) and record the read-only before/after `git status --porcelain` + `HEAD` proof.
- [X] T067 Run the Rust gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test --all-features --manifest-path src-tauri/Cargo.toml`; fix all failures.
- [X] T068 Run the frontend gates: `pnpm typecheck` and `pnpm test`; fix all failures.
- [X] T069 Cross-check `src-tauri/src/review.rs` and its submodules against `specs/004-diff-viewer/contracts/review-commands.md` and `contracts/save-reference-and-activity.md`: all states, error codes, caps, camelCase fields, and the read-only invariant are implemented, with no transport holding business logic.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup — blocks all stories.
- **US1 (Phase 3)**: Depends on Foundational; independent thereafter.
- **US2 (Phase 4)**: Depends on Foundational and integrates US1's viewer/file-list/text components.
- **US3 (Phase 5)**: Depends on Foundational and US1's viewer; uses the T006/T043 model/helper surface.
- **US4 (Phase 6)**: Depends on Foundational and US1's viewer/file list; independent of US2/US3.
- **US5 (Phase 7)**: Depends on Foundational and US1's text rows/viewer.
- **Polish (Phase 8)**: Depends on all stories being complete.

### User Story Dependencies

- **US1 (P1)**: No story dependency — the MVP.
- **US2 (P1)**: Requires US1's viewer/list/text implementation (integration), but its own navigation/memory/isolation behavior is independently testable.
- **US3 (P2)**: Requires US1; no dependency on US2 or US4.
- **US4 (P2)**: Requires US1; no dependency on US2 or US3.
- **US5 (P3)**: Requires US1; no dependency on US2–US4 except that all modes render in the same component.

### Within Each User Story

- Write and fail the tests first.
- Backend parsing/resolution before orchestration.
- Models/DTOs before service behavior.
- Core behavior before Vue rendering.
- Story behavior before integration touch-points.
- Run the story checkpoint before starting the next priority.

### Parallel Opportunities

- Setup: T002/T004/T005 can proceed alongside T001/T003 (different files).
- Foundational: T007, T008, T009, T017, T018 are `[P]`.
- US1: all test tasks (T019–T023) are `[P]`; T024 (`status.rs`) and T025 (`text.rs`) touch different files; T028 (`ReviewFileList.vue`) can run alongside T029 (`ReviewTextDiff.vue`).
- US2: T034 and T035 are `[P]`; T037 (`SaveChangesModal.vue`) can run alongside T038 (`ReviewViewer.vue`) once the store contract (T036) is fixed.
- US3: T041/T042/T043 are `[P]`; T045 (`saved.rs`) and T047 (store/drawer) are on different files.
- US4: T049/T050 are `[P]`; T051 (`image.rs`) and T053 (`ReviewImageDiff.vue`) can run in parallel.
- US5: T056/T057 are `[P]`; T058 (`highlight.ts`) and T059 (`review.ts`) are `[P]`.
- Polish: T063/T064/T065 are `[P]`; run T067/T068 after the audits.

---

## Parallel Example: User Story 1

```bash
# Launch the story's tests together:
Task: "Add status parsing tests in src-tauri/src/review/status.rs"
Task: "Add diff row/fold/cap tests in src-tauri/src/review/text.rs"
Task: "Add file-list tests in src/components/__tests__/review-file-list.test.ts"
Task: "Add text-diff tests in src/components/__tests__/review-text-diff.test.ts"

# Then split backend implementation:
Task: "Implement porcelain -z parsing in src-tauri/src/review/status.rs"
Task: "Implement -U0 row/fold parsing in src-tauri/src/review/text.rs"

# And frontend presentation:
Task: "Implement the listbox in src/components/reviews/ReviewFileList.vue"
Task: "Implement side-by-side rendering in src/components/reviews/ReviewTextDiff.vue"
```

## Parallel Example: User Story 4

```bash
Task: "Add image/binary tests in src-tauri/src/review/image.rs"
Task: "Add image/binary component tests in src/components/__tests__/review-image-diff.test.ts"
Task: "Implement image sniffing/caps in src-tauri/src/review/image.rs"
Task: "Implement before/after previews in src/components/reviews/ReviewImageDiff.vue"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 (Setup).
2. Complete Phase 2 (Foundational) — this blocks every story.
3. Complete Phase 3 (US1).
4. **STOP and VALIDATE**: run the US1 independent test plus the read-only proof (T033).
5. Demo the single-project unsaved-change review.

### Incremental Delivery

1. Setup + Foundational → the viewer shell and both transports exist.
2. US1 → single-project unsaved review (MVP).
3. US2 → multi-project Save-Changes review (completes the P1 core).
4. US3 → saved-state review from the activity drawer.
5. US4 → images and binary changes.
6. US5 → unified/raw technical view and syntax colouring.
7. Polish → docs, a11y, quickstart, and all quality gates.

### Parallel Team Strategy

After Foundational:

- Developer A: US1 (file inventory + text diff + row entry).
- Developer B: waits for US1's viewer contract, then US2 navigation/memory.
- Developer C: waits for US1's viewer contract, then US3 saved-state mode and save references.
- Developer D: waits for US1's file list, then US4 image/binary previews; US5 after text rows stabilize.

---

## Notes

- `[P]` means different files and no dependency on an incomplete task.
- `[USn]` maps every story task to its spec story for traceability.
- The viewer is strictly read-only: no task may add `add`, `commit`, `reset`, `checkout`, `restore`, `clean`, `stash`, or `apply` to any review path.
- All Git calls go through `GitRunner`; all diff/cap/rename/conflict decisions stay in `review.rs`/`service.rs`; both transports stay one-line delegators.
- Every user-facing string lands in both `messages.en.ts` and `messages.de.ts`; no porcelain letters, `@@`, `HEAD`, or hashes in the default view.
- Commit after each task or logical group, and stop at each checkpoint to validate the story independently.
