# Quickstart: AI-Generated Commit Messages

Validation guide for the finished feature. Details live in
[data-model.md](data-model.md) and [contracts/](contracts/).

## Prerequisites

- Built repoman (`pnpm build` frontend + `cargo build` — desktop) or web mode.
- Any OpenAI-compatible endpoint. Cheapest local loop:
  `ollama serve` + model `qwen3-coder:30b`.
- A credential in the **app process environment** (web: the server's env):
  `export OPENAI_API_KEY=…` (OpenRouter/OpenAI). Keyless local endpoints need
  none — set `"apiKeyEnv": ""` (empty string = no credential required; the
  lookup is skipped, per contract C4 keyless rule).
- Two dirty test repos with clearly different changes, e.g.:

  ```bash
  # repo A: feature change
  echo "fn help() {}" >> /tmp/repoman-demo/api/src/cli.rs
  # repo B: fix + untracked file
  sed -i 's/timestamp/tomstamp/' /tmp/repoman-demo/theme/js/app.js
  echo "new file" > /tmp/repoman-demo/theme/notes.txt
  ```

## Config

Add to repoman's `AppConfig` JSON (`~/.config/repoman*`, see `config show`
equivalent / `ConfigManager::default_config_path`):

```json
"ai": {
  "baseUrl": "http://localhost:11434/v1",
  "model": "qwen3-coder:30b",
  "apiKeyEnv": ""
}
```

Restart repoman so the env var + config snapshot are picked up.

## Automated gates

```bash
cargo test --all-features            # commitgen: render(), truncation, sandbox refusal
cargo clippy --all-targets -- -D warnings && cargo fmt --check
pnpm vitest run                      # repoStore commitgen slice + modal rows
cargo test --all-features -- --ignored   # LIVE test — requires endpoint + key
```

## Manual scenarios → spec coverage

| # | Steps | Expect | Covers |
|---|---|---|---|
| 1 | Remove the `ai` block. Open Save Changes on 2 dirty repos. | "Generate with AI" control present but **disabled**, hint "AI not configured" (DE/EN per language switcher); manual message flow works unchanged | US-5, SC-005 |
| 2 | Restore config, unset `OPENAI_API_KEY` (endpoint requiring it), click Generate. | all rows go error with reason naming the variable; manual typing works | US-5 AC2, contract C4 |
| 3 | Generate for repo A + B together. | two rows spin `generating` concurrently; each turns editable the moment its draft lands; A's subject mentions A's change only | US-1, US-2, SC-001, FR-002 |
| 4 | Drafts ready → edit row A → Commit. | A commits its edited text, B its draft; `git log -1` matches per repo | US-1 AC2/4, SC-002 |
| 5 | Close dialog before committing a completed run; reopen. | no drafts anywhere (session-only); repos untouched by generation (`git log` unchanged) | US-1 AC3, Principle V |
| 6 | Kill the model endpoint mid-run (`ollama stop` for B's model). | A's draft survives; B row shows error + retry icon; retry re-runs **only** B | US-3, SC-004 |
| 7 | Dirty repo with a 5 000-file vendor drop. | run completes (truncated marker in prompt log), row gets a best-effort draft; no hang | Edge "very large change sets", FR-009 |
| 8 | Repo in merge-conflict state. | per-row error "resolve conflicts first"; commit gating unchanged | Edge conflicts, contract C4 |
| 9 | Web mode: `repoman --web`, same selection in a browser. | identical rows/status over SSE; run works with server-side env | Principle II |
| 10 | German and English switcher through all of the above. | every new label/state/hint translated | FR-014 |
| 11 | Point `apiKeyEnv` at a sentinel (`REPOMAN_SENTINEL_KEY=sk-SENTINEL-9f3a`), run once, then grep the AppConfig file and captured tracing output for `sk-SENTINEL-9f3a`. | sentinel appears in **neither** config file, logs, nor UI text | FR-013, SC-006 |

## Exit criteria

All manual rows ✔ + automated gates green ⇒ feature validated against
`spec.md` Success Criteria SC-001…SC-006 (SC-003's 7-of-10 acceptance rate is
measured on real repos once, by the reviewer, with drafts left unedited).
