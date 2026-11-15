# Phase 0 Research: Read-Only Change Viewer

**Feature**: `specs/004-diff-viewer` | **Date**: 2026-11-15

No NEEDS CLARIFICATION markers remain from the plan. Spec clarifications already
fixed the preview bounds, accessibility bar, conflict rule, unsaved-change scope,
and the no-hard-cap policy. Every entry below records a design decision validated
against the current codebase, the constitution, and the existing ADRs.

## R1. Diff source: read-only Git output parsed in Rust

- **Decision**: Use only system Git through `GitRunner`:
  - unsaved file inventory: `git status --porcelain=v1 -z --untracked-files=all`
  - saved file inventory: `git diff --name-status -z -M <base> <target>` plus
    `git diff --numstat -z -M <base> <target>`
  - per-file line ranges: `git diff --no-color --no-ext-diff --no-textconv --unified=0 -M <base> <target> -- <path>`
  - file text: `git show <rev>:<path>` for committed revisions and a bounded
    worktree read for the current side; an empty side for additions/deletions
  - commit shape: `git rev-list --parents -n 1 <hash>` to detect a root commit
    and fall back to the empty tree `4b825dc642cb6eb9a060e54bf8d69288fbee4904`
  - Parse porcelain status, numstat, and `@@` hunks in `src-tauri/src/review.rs`
    into typed rows. Rust owns all diff decisions; the frontend only renders.
- **Rationale**: Principle III requires the system CLI and forbids vendoring a
  Git library. Parsing `-U0` hunks against whole old/new text gives exact changed
  ranges without trusting a second diff implementation, and it supports folded
  unchanged stretches, unified display, and huge-file caps from one source of
  truth. `--no-ext-diff`/`--no-textconv` prevent repository-configured drivers
  from executing during a read-only review. `-M` preserves rename pairs so the
  viewer can label *Umbenannt* and compare the old path with the new path.
- **Alternatives considered**: (a) `similar`/`diffy` crate — a second diff engine
  to keep in sync with Git's own hunk boundaries and rename choices, plus a new
  dependency; (b) view only `git diff` output with `-U3` and no full files —
  cannot expand folded regions or show unchanged context on demand; (c) libgit2 /
  `git2` — violates Principle III and loses the user's Git configuration;
  (d) parse `git diff --word-diff` — noisier and not a line diff.

## R2. Request shape: lazy project summary then lazy file preview

- **Decision**: Two read-only requests:
  1. `review_project { path, mode, commitHash? }` returns the project name,
     availability, plain-language file entries, and added/removed/file totals.
     It reads status + numstat only (and operation state for the conflict rule).
  2. `review_file { path, mode, commitHash?, filePath, oldPath? }` returns one
     bounded preview: text rows (plus raw diff for technical view), image data
     URLs and dimensions, or a binary description.
  The viewer fetches the active project on open and one preview per selected
  file; moving to another project fetches that project's summary and restores
  its remembered selected file (fetching that file if needed).
- **Rationale**: SC-002 asks for the first file within 2 s up to 50 files; loading
  50 file bodies up front would fail that goal and increase payload size for no
  benefit. Lazy loading also gives every file a natural loading/empty/error state
  (ui:states) and keeps FR-016's per-file cap local to the file being viewed.
  Multi-project navigation stays one action per hop (SC-006).
- **Alternatives considered**: one request returning every file's full content —
  slow and dangerously large; client-side Git/network reads — violates Principle
  II and direct-frontend-filesystem rule; streaming diff progress over the event
  bus — unnecessary for a synchronous, user-selected read.

## R3. Saved-state reference: capture the commit made by a save

- **Decision**: On a successful commit, `GitRunner::commit_changes` reads
  `git rev-parse HEAD` and returns it in `OperationResult` together with the
  `run_id` used by that commit's Git-step events. `repoStore.commitRepositories`
  stores a session-only `Map<runId, { repoPath, commitHash, savedAt }>`. The
  activity drawer shows *Ansehen* on `commit` runs; it passes the captured
  `commitHash` when present and falls back to the project's current `HEAD` when a
  run predates capture. `review_project`/`review_file` resolve `commitHash`
  (default `HEAD`), verify it with `git rev-parse --verify <hash>^{commit}`, and
  report an `unavailable` project state when the commit or repository cannot be
  resolved.
- **Rationale**: FR-017 requires every save entry to be openable and an
  unavailable state to be explained. Capturing the exact hash makes an activity
  entry show *that* save rather than a later one; the HEAD fallback keeps the
  affordance present for older session entries. The review then uses
  `commit^..commit` (or empty tree for a root commit), which is exactly what the
  save included. Fields are additive and camelCase-serialized, so existing
  Tauri/web JSON contracts keep working.
- **Alternatives considered**: associate only by project path and always read
  current `HEAD` — wrong entry after a second save; persist save references in
  `AppConfig` — unnecessary state and a schema migration; infer the commit from
  the `git commit` output text — brittle and localized.
- **Guard**: the activity log and the save-reference map remain session-only
  (ADR__261030-4). A restarted app has no stale activity entries.

## R4. Image and binary previews: format sniffing and explicit fallbacks

- **Decision**: A file is treated as an image only when its extension is one of
  PNG, JPEG, GIF, WebP, or SVG **and** its bytes parse. Raster dimensions come
  from `imagesize`; SVG dimensions come from `width`/`height` or `viewBox` in the
  leading text. Supported images are read from both sides (worktree/`git show`),
  base64-encoded, and returned as `data:` URLs with dimensions and byte sizes.
  Over-limit (>10 MB or >8,000 px in either dimension), corrupt, or unsupported
  bytes produce a notice state, never a broken `<img>`. Files that are neither
  text nor a supported image return `binary` with old/new sizes and the
  *"Binärdatei geändert"* state.
- **Rationale**: FR-010/FR-011 and the corrupted-image edge case; bounds must be
  checked **before** embedding bytes so the viewer cannot hang or allocate an
  unbounded payload. `imagesize` reads headers only, so a 10 MB image does not
  get decoded into a pixel buffer in the core. Base64 is the only reliable way to
  carry bounded image bytes through both Tauri IPC and axum JSON.
- **Alternatives considered**: `image` crate — decodes pixels we never need;
  browser `File`/URL access — the frontend must not read the filesystem directly;
  a new static image endpoint — extra authenticated surface and path-traversal
  risk for no gain; extension-only detection — corrupt files would still break.

## R5. Text preview rows, folding, and caps

- **Decision**: For a text file the core loads the old and new sides (bounded),
  parses `-U0` hunks, and emits `DiffRow`s with old/new line numbers and text.
  Unchanged runs between hunks become `fold` groups containing their lines; the
  friendly view renders them collapsed and expandable. The core stops at
  **2 MB** of preview text or **5,000 diff lines**, whichever comes first, and
  marks the preview `truncated` with a *rest not displayed* notice. A raw unified
  diff string for the technical view is included only when it fits the same cap.
  Binary detection uses numstat's `-` marker and a NUL-byte check on the leading
  sample, so a misnamed binary is not rendered as text.
- **Rationale**: FR-007/FR-016/FR-009 need exact changed ranges, full context for
  expansion, and a hard stop that never looks like a hang. Emitting rows once in
  the core keeps the unified/raw views derived from the same data and prevents
  frontend re-implementations of diff semantics. Caps are applied while building
  the payload, not after it has grown.
- **Alternatives considered**: send full files and diff client-side — duplicates
  logic and risks unbounded IPC; omit unchanged lines entirely — no expandable
  folds (FR-007); cap only by byte size — one long line could still produce too
  many rows.

## R6. Rename, conflict, and whitespace-only rules

- **Decision**:
  - **Rename**: `status --porcelain -z` rename records and `diff --name-status -z -M`
    yield `oldPath` + `path`; the viewer compares `oldPath` at the base with
    `path` at the target and labels *Umbenannt*.
  - **Conflict**: a path is labelled *Konflikt* only when its porcelain X/Y code
    is unmerged **and** `detect_operation_state` reports `merging`, `rebasing`, or
    `cherry-picking`. Unmerged codes outside an in-progress operation are still
    reviewable but do not claim the conflict label (spec clarification).
  - **Whitespace-only**: for a tracked text file, run
    `git diff --ignore-all-space --ignore-blank-lines --quiet <base> -- <path>`;
    exit code 0 with a non-empty normal diff means *"Nur Leerzeichen geändert"*.
    The file is still shown fully.
- **Rationale**: Matches FR-006/FR-012 and the spec's explicit conflict rule;
  uses Git's own rename/whitespace semantics instead of approximating them.
- **Alternatives considered**: raw status letters in the UI — forbidden by
  FR-006/SC-003; treating every unmerged path as a conflict — broader than the
  clarified rule; a custom whitespace normalizer — risks disagreeing with Git.

## R7. Frontend presentation, colour, and accessibility

- **Decision**:
  - `ReviewViewer.vue` is a `TdModal size="full"` with a translated title,
    *Fertig* button, and Escape close (TdModal already traps focus).
  - The file list is a `role="listbox"` with `role="option"` rows, roving
    `tabindex`, Arrow/Home/End navigation, Enter/Space selection, visible focus,
    and `aria-selected`; project navigation is labelled and announces
    *"Projekt N von M"* via a polite status region.
  - Fold rows are real `<button aria-expanded>` controls; the technical-mode
    toggle is keyboard reachable; no control is hover-only.
  - Added/removed tinting uses `bg-success-soft` / `bg-error-soft` with a text
    marker (`+`/`−` and a translated cell label), so colour is never the only
    signal. Syntax colouring comes from `highlight.js/lib/core` with a fixed
    language registry (plaintext fallback) and token-only CSS for `.hljs-*`;
    tinting stays on the row so it remains dominant (FR-009).
  - Every string comes from `src/lib/i18n/messages.{en,de}.ts`; the default view
    shows no `A`/`M`/`D`, `@@`, `HEAD`, or commit hashes. Raw Git text appears
    only in the explicitly selected technical view.
- **Rationale**: FR-018/FR-019/FR-021 and SC-003/SC-004/SC-009; `TdModal` gives
  the dialog role, focus trap, and Escape handling, so the viewer must not build
  its own modal. A fixed highlight.js core import avoids the full bundle.
- **Alternatives considered**: Shiki/Prism — larger or theme-coupled; custom
  regex highlighting — poor and brittle; colour-only tinting — fails a11y;
  `vue-i18n` — rejected by ADR__261031-1.

## R8. Staleness, read-only proof, and concurrency

- **Decision**: The existing `repo:fs-changed` event only sets a `stale` flag for
  a project currently open in the viewer (and for its summary); it never
  re-fetches or replaces displayed content. A translated *Aktualisieren* prompt
  refetches the current project on demand and clears the flag. All review paths
  run read-only Git commands through `GitRunner`; tests run a review against a
  fixture repository and assert `git status --porcelain` and `HEAD` are unchanged.
  Project/file reads use a bounded semaphore (8) when several paths are read in
  one action; no lock is held across `.await`.
- **Rationale**: FR-001/FR-015/SC-005 and Principle V. Explicit refresh preserves
  the user's scroll/selection and avoids jumping content.
- **Alternatives considered**: auto-refresh on every watcher event — forbidden by
  FR-015 and jarring; disabling the watcher while open — hides legitimate
  staleness; no bound — risks descriptor/task pressure on large selections.

## R9. Dependencies added

- **Decision**: add backend `base64 = "0.22"` and `imagesize = "0.13"`; add
  frontend `highlight.js = "^11"` imported as `highlight.js/lib/core` with only
  the needed language modules.
- **Rationale**: `base64` is the standard, audited way to carry bounded binary
  previews over JSON IPC/HTTP. `imagesize` reads image headers only, keeping
  oversized images from being decoded. `highlight.js` core is the smallest
  mature highlighter that can be limited to a known language list and themed with
  CSS variables.
- **Alternatives considered**: hand-rolled base64 — needless risk; decoding with
  `image` — unnecessary pixel work; no highlighting — violates FR-009; a
  CDN/remote theme — violates the Tauri CSP guidance and offline use.

**Status**: complete — no unresolved unknowns; the plan Constitution Check stands.
