# Phase 1 Data Model: AI-Generated Commit Messages

Session-only drafts (spec Assumptions; cleared when the Save Changes dialog
closes). "Frontend store" = `commitgen` slice in `repoStore.ts`; "Backend" =
Rust types. The **only** persisted structure is `AiSettings` inside the existing
`AppConfig` JSON.

## E1. AiSettings (persisted — `AppConfig.ai`, optional; serde camelCase)

| Field | Type | Default | Rule |
|---|---|---|---|
| `baseUrl` | string | — | OpenAI-compatible endpoint (e.g. `https://openrouter.ai/api/v1`, `http://localhost:11434/v1`) |
| `model` | string | — | model id passed to the provider (e.g. `anthropic/claude-sonnet-4.5`) |
| `apiKeyEnv` | string | `"OPENAI_API_KEY"` | **name** of the environment variable holding the credential; the value is never persisted (FR-013). Empty string `""` ⇒ keyless provider: lookup skipped, no auth header (analyze A1) |
| `maxToolCalls` | u32 | 12 | per-run lookup budget, passed as serdesAI `max_tool_calls` (FR-010) |
| `diffBudgetChars` | usize | 40 000 | hard cap on change-detail sent per repo (FR-009) |

**Validation rules**

- `ai: null`/absent → feature visibly unavailable: generate control disabled + hint (US-5). No error path may trigger.
- `baseUrl` MUST parse as `https?://…`; invalid stored value → treated like unconfigured for runs started after load, with a distinct hint string (`save.ai.badConfig`).
- All rows share one `AiSettings`; changing it mid-run has no effect (snapshot taken at run start).

## E2. CommitDraft (Rust struct + wire object; serdesAI `OutputSchema` output)

| Field | Type | Rule |
|---|---|---|
| `type` | string | Conventional Commit type; schema enum `feat\|fix\|refactor|perf|docs|style|test|build|ci|chore|revert` |
| `scope` | string \| null | optional component scope, lowercase-hyphen |
| `subject` | string | non-empty, ≤ 72 chars after trim, no trailing period (schema-validated; retry on violation) |
| `body` | string \| null | optional wrapped prose explaining *why* |

- Rendering (backend-owned, single function `draft.render()`): `type(scope): subject`, then blank line + `body` when present. The rendered string — not the struct — is what seeds the row textarea and what `commit_repo_changes` receives (unchanged commit path).
- Wire form (events + store): camelCase `{ type, scope, subject, body, rendered }` — `rendered` computed once in Rust so DE/EN or trimming edge cases never diverge between transports.

## E3. CommitGenEvent (Rust payload on the typed bus, camelCase serde like `ScanProgressEvent`)

```text
CommitGenEvent {
  run_id:  String,          // uuid v4, one per batch generation
  path:    String,          // repo being summarized (unique per row)
  phase:   started | done | error,
  draft:   Option<CommitDraft+rendered>,   // phase=done
  error:   Option<String>,                 // phase=error — human-readable reason
}
```

**Ordering guarantees** (transport contract → `contracts/commitgen-events.md`)

- Per `run_id` + `path`: exactly one `started`, then at most one terminal
  (`done` or `error`). Retry emits a **new** `started`/terminal pair for the same
  path under the same `run_id` (FR-008/US3-AC2: only that row resets).
- Events for unknown `run_id` are ignored by the store (late events after close).
- The command itself carries no drafts; the bus is the single result channel for
  both transports (Principle II).

## E4. RepoDraftRow (frontend aggregate, `Map<path, row>` in the store)

| Field | Type | Rule |
|---|---|---|
| `path` | string | key; aligned with `repos` selection of the modal |
| `name` | string | folder name (already in store; row is display-only) |
| `status` | `idle \| generating \| ready \| error` | state machine below |
| `message` | string | textarea model: starts `''`, becomes `draft.rendered` on `done`, **user edits win** — any late event for a row the user has typed into is ignored (`dirty` flag) |
| `dirty` | bool | true once the user edits; only cleared by a fresh `started` for that path (FR-015 overwrite is user-triggered, so it is allowed to clear) |
| `error` | string \| null | last terminal error reason (shown with retry icon) |

**State transitions**

```text
            ┌────────────────────────────────────┐
 idle ─started─▶ generating ──done──▶ ready ─────┤ (user edit keeps status=ready, dirty=true)
   ▲                │                             │
   │                └──error──▶ error ──retry─────┘  (retry ⇒ started, same path)
   └── modal closed (whole slice discarded)
```

**Validation rules**

- Commit confirmation gating (FR-006, unchanged rule, now per row): enabled iff every selected repo has a non-empty `message` and no row is `generating`.
- `error` rows accept manual typing → behave as `ready` once non-empty (US3-AC3).
- Rows exist only for the modal's current selection; de-selecting a repo mid-run drops its row (in-flight results for it are ignored).

## E5. Relationships

- One generation **run** = one command call over N selected repos → N `path`s, each with exactly one active row and one agent task under the batch semaphore (research R6).
- `AiSettings` (persisted, app-wide) → snapshot consumed by every task of a run.
- `CommitDraft` (transient) → rendered text lives on until commit; **the app never persists drafts anywhere** (session-only rule).
