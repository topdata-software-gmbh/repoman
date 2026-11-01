# Contract: `generate_commit_messages` command / route

Starts one AI generation run for a batch of repositories. Returns immediately —
results arrive exclusively on the typed event bus
([commitgen-events.md](commitgen-events.md)). No other behavior of the Save
Changes flow changes; the existing commit command still receives final row text
per repo.

## C1. Transport mapping

| Transport | Surface | Shape |
|---|---|---|
| Desktop (Tauri) | `invoke("generate_commit_messages", { repoPaths })` in `commands/git_commands.rs` | thin delegator → `service::generate_commit_messages` |
| Web (axum) | `POST /api/repositories/generate-commit-messages` | same handler core; sits behind existing `require_token` middleware, same as `/api/repositories/commit` |
| Frontend helper | `generateCommitMessages(paths): Promise<string /* runId */>` in `src/lib/backend.ts` | one function, two transports behind it |

**Rule (Principle II)**: neither transport may inspect `AiSettings`, spawn
tasks, or format messages — all of that lives in `service`/`commitgen`.

## C2. Request

```json
{ "repoPaths": ["string", "…"] }
```

- Every path MUST be a discovered repository known to the store; unknown or
  duplicate paths are rejected before any task spawns.
- Empty list → 400 / `Err("no repositories selected")` (cannot happen from the
  modal: the control is enabled only with ≥1 dirty repo).

## C3. Response (success)

```json
{ "runId": "uuid v4" }
```

- `runId` is emitted in the first `started` events; the client MUST treat the
  POST/invoke return as "run accepted", never as results.
- HTTP status `202 Accepted` on web (long-running side effect reported via SSE).

## C4. Refusals (pre-run, no events emitted)

| Condition | Response | UI mapping |
|---|---|---|
| `config.ai` absent or invalid | `Err("ai-not-configured")` | control already disabled (US-5); error string only if invoked programmatically |
| Credential env var missing at run start | run is accepted; **every** row gets `phase=error` with reason naming the variable (per data-model E3) | per-row error + hint `save.ai.missingKey` with `{envVar}` |
| Repo has no pending changes | per-row `phase=error`, reason "nothing to summarize" | row error + manual typing still possible |
| Repo in conflict state | per-row `phase=error`, reason "merge conflicts — resolve first" | row error; commit gating unchanged |

**Keyless providers (analyze A1 decision)**: an `apiKeyEnv` of `""` explicitly
means "no credential required" — the env lookup is skipped, no auth header is
sent, and the missing-credential refusal above does NOT apply. A *non-empty*
variable name that is unset still triggers it.

**Security invariants**

- `apiKeyEnv` value is read from the server process environment at run start
  and MUST NOT appear in any response, event payload, log line, or UI string
  (FR-013, SC-006).
- The run performs **no writes** to any repository or to `AppConfig` (Principle V).
