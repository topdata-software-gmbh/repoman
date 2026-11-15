# Implementation Plan: Read-Only Change Viewer

**Branch**: `004-diff-viewer` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-diff-viewer/spec.md`

## Summary

Add one shared, strictly read-only change viewer with two entry modes: a
project's **unsaved changes** and the changes included in a **saved state**.
The viewer opens from a project row, from the Save Changes dialog (over every
selected project), and from a save entry in the "What Git did" activity drawer.
It presents a plain-language file list (*Neu*, *Geändert*, *Gelöscht*,
*Umbenannt*, *Konflikt*), a friendly side-by-side before/after view with folded
unchanged stretches, optional unified and raw technical views, syntax colouring,
image before/after previews, and clear notices for binary files, oversized
previews, unreadable projects, and unavailable saved states. It never writes,
stages, discards, or reverts anything.

Technical approach: a new read-only `review` module in the Rust core owns every
Git read and every diff-derived decision. Two new transport-agnostic service
functions — `review_project` (file list + summary) and `review_file` (one
bounded preview) — are exposed through one Tauri command pair and one axum route
pair. Git is still executed only through `GitRunner` (system CLI); diff hunks
are parsed in Rust from `git status`/`git diff` output, so no Git library and no
third-party diff engine are introduced. The Vue viewer is a `TdModal`
(`size="full"`) backed by a session-only Pinia slice that remembers the current
project, the selected file per project, and the display mode. Newly arriving
filesystem changes only mark the open project stale; the snapshot refreshes on
an explicit *Aktualisieren* action. To make a save reviewable from the activity
drawer, a successful commit now reports its `runId` and resulting commit hash in
the existing `OperationResult`, and the viewer resolves that exact saved state.

## Technical Context

**Language/Version**: Rust 2021, MSRV 1.80.0 (`src-tauri/Cargo.toml`) · TypeScript 5.5 + Vue 3.4 (`<script setup>`, Composition API) + Pinia 2

**Primary Dependencies**: existing `tokio`, `serde`, `serde_json`, `uuid`, `axum`, `tauri`, `@tauri-apps/api`, `@topdata-software-gmbh/ui-kit-web-vue3`, `lucide-vue-next` · NEW backend: `base64 = "0.22"` (image data URLs over IPC/JSON) and `imagesize = "0.13"` (PNG/JPEG/GIF/WebP dimensions without decoding pixels) · NEW frontend: `highlight.js = "^11"` (tree-shaken `lib/core` with only the languages the viewer names)

**Storage**: None new. The review session (current project, selected file per project, display mode, stale flags, captured save references) is session-only in Pinia, matching the existing Git-step log retention (ADR__261030-4). No configuration schema change.

**Testing**: `cargo test --manifest-path src-tauri/Cargo.toml` (in-module unit tests plus `tempfile` Git fixtures for status parsing, hunk rows, renames, conflicts, whitespace, caps, image sniffing, saved-state resolution, and read-only assertions) · `pnpm test` (vitest + `@vue/test-utils`: store slice, viewer component states, keyboard/ARIA, i18n parity, token-only guard) · `pnpm typecheck` · existing quality gates from the constitution

**Target Platform**: Tauri v2 desktop window and the same Vue build served by `repoman --web` (axum + SSE) — one core, two thin transports

**Project Type**: Desktop application with an embedded web server (single frontend, shared Rust service)

**Performance Goals**: For ≤50 changed files the first file's comparison is visible within 2 s on local disk (SC-002); the project summary comes from one status call plus one numstat call, and only the selected file's content is loaded; multi-project navigation costs one request per hop; the viewer never blocks on a file beyond its preview cap

**Constraints**: Strictly read-only (no write/stage/discard/revert Git subcommand) · Git only through `GitRunner` (Principle III) · all diff and availability decisions in `review`/`service`, transports stay thin (Principle II) · every UI string in `src/lib/i18n` DE+EN; default view contains no Git shorthand (Principle IV, SC-003) · text preview bounded to the first 2 MB or 5,000 diff lines, images bounded to 10 MB and 8,000×8,000 px, file count unbounded (FR-016, FR-010, FR-022) · no lock held across an await; project/file reads bounded by a semaphore (Principle V) · WCAG 2.1 AA within the viewer (FR-021)

**Scale/Scope**: Typical 1–50 changed files per project and 1–4 projects per review; no hard cap on file count (larger projects open with the same bounded previews). Scope is two new read-only command pairs, one new Rust module, one Pinia slice, one modal plus small child components, activity-drawer and row/Save-Changes entry points, and translation additions.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

Against `.specify/memory/constitution.md` v1.0.0:

- **I. Spec-Driven Change**: **PASS** — this plan is produced from the accepted `specs/004-diff-viewer/spec.md`; implementation follows plan → tasks.
- **II. One Core, Two Frontends**: **PASS** — all Git reads, status parsing, hunk parsing, caps, rename/conflict/whitespace decisions, and saved-state resolution live in `review.rs` behind `service::review_project` / `service::review_file`. The Tauri commands and axum handlers are one-line delegators; DTOs are explicit `serde` types with camelCase renames, and the frontend calls them only through `src/lib/backend.ts`.
- **III. Git Only Through the System CLI**: **PASS** — only read-only Git commands run (`status --porcelain`, `diff`, `show`, `rev-parse`, `rev-list`). `imagesize`/`base64` inspect bytes already read from Git or the worktree and never touch repository metadata. No Git library is added.
- **IV. Beginner-Friendly by Default**: **PASS** — the file list exposes status enums, never porcelain codes; every label, notice, and error is a DE+EN dictionary key; backend errors are stable codes the frontend turns into next-action copy; raw Git text appears only in the explicitly chosen technical view.
- **V. Bounded, Non-Destructive Action**: **PASS** — the feature exposes no mutating operation, caps every preview, bounds concurrent reads with a semaphore, never holds a lock across `.await`, and adds no configuration writes.
- **Security & Configuration Constraints**: **PASS** — no secret or credential path is touched; no filesystem path outside the requested repository is exposed to the frontend beyond paths Git already reports; payload sizes are capped.
- **Dependency justification** (conventions `rust:dependencies`, `ui:performance-perception`): `base64` and `imagesize` are small, focused crates used only by `review`; `highlight.js` is imported through `lib/core` with an explicit language list. Alternatives are recorded in `research.md` R1/R4/R7.

**Gate result: PASS** (pre-research).

## Project Structure

### Documentation (this feature)

```text
specs/004-diff-viewer/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   ├── review-commands.md
│   └── save-reference-and-activity.md
└── tasks.md             # Phase 2 output (/speckit.tasks — NOT this command)
```

### Source Code (repository root)

```text
src-tauri/src/
├── review.rs                    # NEW: public review API + orchestration
├── review/
│   ├── status.rs                # porcelain -z → DiffFileEntry (rename/conflict/untracked)
│   ├── text.rs                  # git diff --unified=0 → side-by-side rows + fold segments
│   ├── saved.rs                 # commit^..commit, root empty-tree fallback, availability
│   ├── image.rs                 # imagesize + SVG width/height parsing → dimensions/notice
│   └── limits.rs                # text 2 MB / 5,000 diff lines; image 10 MB / 8,000 px
├── service.rs                   # MODIFY: review_project / review_file entry points
├── git_runner.rs                # MODIFY: read-only helpers (status -z, numstat, rev-parse,
│                                #   rev-list parents, diff/show bytes) + HEAD hash after commit
├── models.rs                    # MODIFY: add run_id/commit_hash to OperationResult
├── commands/git_commands.rs     # MODIFY: thin review_project / review_file commands
├── web.rs                       # MODIFY: POST /api/repositories/review and /review-file
└── Cargo.toml                   # MODIFY: base64, imagesize

src/
├── lib/backend.ts               # MODIFY: reviewProject / reviewFile transports
├── types/domain.ts              # MODIFY: ReviewMode, ProjectComparison, DiffFileEntry,
│                                #   DiffPreview, SaveRef, OperationResult additions
├── lib/review.ts                # NEW: display helpers (labels, size/line formatting,
│                                #   row→unified/raw presentation, language selection)
├── lib/highlight.ts             # NEW: highlight.js core + explicit language registry
├── stores/repoStore.ts          # MODIFY: session-only review slice + save reference map
├── components/
│   ├── ReviewViewer.vue         # NEW: full modal, project navigation, stale prompt
│   ├── reviews/
│   │   ├── ReviewFileList.vue   # NEW: plain-language list (listbox + roving focus)
│   │   ├── ReviewTextDiff.vue   # NEW: side-by-side/unified rows, folds, colouring
│   │   ├── ReviewImageDiff.vue  # NEW: before/after image + dimensions/size
│   │   └── ReviewNotice.vue     # NEW: empty/unreadable/unavailable/too-large states
│   ├── RepoListTable.vue        # MODIFY: row action "Änderungen ansehen"
│   ├── SaveChangesModal.vue     # MODIFY: "Überprüfen" opens review over selected projects
│   └── GitLogDrawer.vue         # MODIFY: "Ansehen" on commit runs → saved-state review
├── lib/i18n/messages.en.ts      # MODIFY: review.* keys (source of truth)
├── lib/i18n/messages.de.ts      # MODIFY: matching German keys
└── App.vue                      # MODIFY: mount ReviewViewer

src-tauri/macros/git-actions.yaml # UNCHANGED (commit macro still the only save path)

README.md                         # MODIFY: feature summary + new HTTP routes
CHANGELOG.md                      # MODIFY: user-visible entry in the same change
```

**Structure Decision**: Keep the existing single-workspace layout. All
review logic lives in one Rust module so the read-only boundary is visible in
one place and cannot drift between desktop and web. The Vue surface is split into
small presentational children under `components/reviews/`; the modal owns
navigation and lifecycle only. No new event kind is added to `AppEvent`: the
viewer is request/response, and the save reference rides on the existing commit
`OperationResult` plus the existing Git step stream.

## Phase 0 Research Summary

See `research.md`. Decisions: (R1) parse read-only `git status`/`git diff`
output in Rust instead of adding a diff crate; (R2) lazy project summary + file
preview with explicit caps; (R3) capture the post-commit HEAD hash and add it to
`OperationResult` for exact save review; (R4) `imagesize` + SVG header parsing
with corrupt/oversized fallbacks; (R5) whitespace-only via
`git diff --ignore-all-space --ignore-blank-lines --quiet`; (R6) conflict label
only for unmerged paths during merge/rebase/cherry-pick; (R7) highlight.js core
with a fixed language list and token-only diff tints; (R8) staleness is a flag,
never an automatic content replacement.

## Phase 1 Design Summary

See `data-model.md`, `contracts/review-commands.md`,
`contracts/save-reference-and-activity.md`, and `quickstart.md`.

## Post-Design Constitution Re-check

After Phase 1 (research, data model, contracts): **PASS** — the contracts expose
only read-only project/file requests and typed preview payloads; no mutating
Git command, filesystem write, credential, or configuration change is introduced.
The save-reference addition is additive to `OperationResult` and is consumed by
the existing type bus/client. No violations → Complexity Tracking unused.

## Complexity Tracking

> No constitution violations — table unused.
