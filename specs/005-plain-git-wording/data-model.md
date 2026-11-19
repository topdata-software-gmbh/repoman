# Data Model: Personal Wording Preference — Plain Language vs Git Terminology

**Feature**: `005-plain-git-wording` | **Date**: 2026-11-19

This feature adds no server-side data and no `AppConfig` field. It adds one
browser-local preference and one in-memory terminology overlay that is compiled
into the frontend.

## E1 — Wording preference

The person's choice between plain language and Git terminology on this device
and browser.

| Field | Type | Rules |
|---|---|---|
| `value` | `'plain' \| 'git'` | Default `plain` when nothing valid is stored |
| `storageKey` | string literal `topdata.wording.v1` | Fixed; never part of project config |
| `scope` | device + browser | One value per webview/browser profile; no repository/group dimension |

**Persistence**: A synchronous `localStorage` read at startup and write on
change. Storage failures (disabled, full, hardened webview) are swallowed; the
in-memory value remains usable and defaults to plain.

**Independence**: The wording value is never read from or written to
`topdata.language.v1`, `AppConfig`, repository caches, or the server. The
language and wording preferences are combined only at render time.

**Validation**

- **VR-001**: A missing stored value resolves to `plain`.
- **VR-002**: A stored value not in `WORDINGS` resolves to `plain`.
- **VR-003**: A malformed or unavailable storage backend resolves to `plain`
  and does not throw.
- **VR-004**: `setWording()` accepts only `'plain'` or `'git'`; other values are
  ignored (with a development warning) and do not overwrite storage.

**State transitions**

```text
[startup: no key]        -> plain
[startup: "plain"]       -> plain
[startup: "git"]         -> git
[startup: invalid/error] -> plain
plain --user selects Git--> git   (writes topdata.wording.v1="git")
git   --user selects Plain-> plain (writes topdata.wording.v1="plain")
```

The transition is synchronous and affects only label resolution. It never
starts, stops, or changes a Git operation.

## E2 — Terminology entry

A single message key with a plain base value and an optional Git overlay value.

| Field | Type | Rules |
|---|---|---|
| `key` | `MessageKey` | Must exist in the English catalogue |
| `plain.en` / `plain.de` | string | Already in `messages.en.ts` / `messages.de.ts`; the source of truth |
| `git.en` / `git.de` | string? | Optional; non-empty when present; canonical English Git term in both languages |
| `category` | enum | `toolbar`, `table`, `status`, `filter`, `dialog`, `glossary`, `onboarding`, `action`, `insight`, `toast` |
| `surfaces` | list of UI locations | Control, confirmation/failure message, dialog title, badge, glossary name |

**Rules**

- **VR-005**: The plain catalogue stays complete for both languages; the overlay
  never replaces it as the source of truth.
- **VR-006**: Every overlay key must exist in the English catalogue; a
  misspelled key is a compile error because the overlay is typed by
  `MessageKey`.
- **VR-007**: `en` and `de` overlay key sets must be identical (parity test), so
  a missing translation is caught in review. The resolver still tolerates a
  missing or blank value by falling back to plain (FR-010).
- **VR-008**: Overlay values must be short labels, not explanatory sentences.
  Hints, error guidance, and the Git log remain outside this entity.
- **VR-009**: An operation concept's toolbar/table label, action label, dialog
  title, toast title and glossary name must use the same Git term.
- **VR-010**: A term without a Git counterpart (for example a branch name or
  “Review”) has no overlay entry and renders identically in both modes.

**Canonical terms**

| Concept | Git label | Hint suffix | Overlay keys (examples) |
|---|---|---|---|
| Commit | `Commit` | `(git commit)` | `toolbar.saveToGit`, `table.save`, `save.title`, `save.submit`, `save.savedTitle`, `help.word.save.name`, `onboarding.ready.saveTitle`, `insight.dirty.action*` |
| Fetch | `Fetch` | `(git fetch)` | `toolbar.checkForUpdates`, `action.fetch`, `toast.fetch*` |
| Pull | `Pull` | `(git pull)` | `toolbar.getLatest`, `table.getLatestFor`, `help.word.pull.name`, `onboarding.ready.pullTitle`, `action.pull`, `toast.pull*` |
| Push | `Push` | `(git push)` | `toolbar.sendChanges`, `table.sendFor`, `help.word.push.name`, `onboarding.ready.pushTitle`, `action.push`, `toast.push*` |
| Stash | `Stash` | `(git stash)` | `toolbar.setAside`, `action.stash`, `toast.stash*` |
| Stash pop | `Stash pop` | `(git stash pop)` | `toolbar.bringBack`, `action.stashPop`, `toast.stashPop*` |

## E3 — Language preference (existing, unchanged)

The existing German/English choice. It remains independent and is combined with
wording at render time.

| Field | Type | Rules |
|---|---|---|
| `value` | `'de' \| 'en'` | Existing `topdata.language.v1` behavior, default `de` |
| `storageKey` | string literal `topdata.language.v1` | Unchanged |
| `scope` | device + browser | Unchanged |

## E4 — Resolved message (in-memory)

The value returned by `t(key, params)` after combining the three layers.

| Field | Type | Rules |
|---|---|---|
| `template` | string \| function | From the Git overlay, else the active-language plain catalogue, else English |
| `params` | record | Interpolated after selection |
| `wording` | `'plain' \| 'git'` | Determines whether the overlay is consulted |
| `language` | `'de' \| 'en'` | Determines the plain catalogue and the overlay language map |

**Resolution precedence**

```text
if wording == "git":
    gitOverlay[language][key]        (if present and non-blank)
    else plainCatalogue[language][key]
    else plainCatalogue.en[key]
else:
    plainCatalogue[language][key]
    else plainCatalogue.en[key]
```

**Invariants**

- **VR-011**: The resolved label is never an empty string, a placeholder, or a
  raw key when a plain catalogue entry exists.
- **VR-012**: Explanatory/hint/error keys have no overlay entry, so their
  resolved text is identical in both wording modes.
- **VR-013**: `operationHint(key, term)` never changes the base hint; it only
  appends ` (git <term>)` in plain mode.

## Relationships

```text
localStorage "topdata.wording.v1" --read/write--> Wording preference
localStorage "topdata.language.v1" --read/write--> Language preference (existing)

Wording preference --+
Language preference --+--> t(key) --> Resolved message --> every label surface
Terminology overlay --+

Operation control --title/aria-describedby--> operationHint(key, term)
                                             └─> base hint + optional "(git <term>)"
```

## Surface map

| Surface | Entity used | Example in plain | Example in Git |
|---|---|---|---|
| Toolbar button | E2 toolbar entry | Save to Git | Commit |
| Row action | E2 table entry | Save / Get latest / Send changes | Commit / Pull / Push |
| Save dialog title + submit | E2 dialog entry | Save to Git | Commit |
| Save confirmation toast | E2 toast/dialog entry | Saved to Git | Committed |
| Operation toast | E2 toast entry | Latest changes downloaded | Pull complete |
| Status badge | E2 status entry | Changes not saved | Uncommitted changes |
| Status filter | E2 filter entry | Updates available | Behind |
| Help glossary name | E2 glossary entry | Get latest | Pull |
| Onboarding heading | E2 onboarding entry | Send changes | Push |
| Insight action button | E2 action/insight entry | Save all to Git | Commit all |
| Hints / descriptions | no overlay | explained in plain words | unchanged, no suffix |

## Non-goals

- No persisted terminology table; the overlay is bundled frontend code.
- No per-repository, per-group, or server-synced wording.
- No auto-detection of the user's Git experience.
- No change to Git output, commit messages, branch names, file names, or any
  data sent to repositories.
