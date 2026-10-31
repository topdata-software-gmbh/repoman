# Quickstart: Validate Live Scan Progress & Activity Log

Prerequisites: Node 20+ / pnpm 9+, Rust toolchain (`cargo`), a scan root that
actually contains repositories (e.g. `/topdata` or a temp fixture below).

## 1. Automated gates

```bash
cd src-tauri && cargo test          # incl. new scanner sink + throttle + done-always tests
pnpm test                           # vitest: store reducer (started/found/tick/inspected/done/error) + ScanActivityModal
pnpm typecheck                      # vue-tsc clean
```

Expected: all green; scanner results identical to pre-feature tests (sink is
additive), FR coverage mapped in test names (FR-0xx).

## 2. Manual walkthrough — onboarding (desktop)

```bash
pnpm install
mkdir -p /tmp/scan-demo && cd /tmp/scan-demo
for i in $(seq 1 40); do mkdir -p "proj-$i" && git init -q "proj-$i"; done
mkdir -p deep/a/b/c/d && git init -q deep/a/b/c/d/deep-proj
```

1. Start the app with a fresh config (`repoman --config /tmp/scan-demo-config.json` via `pnpm start -- --config …`, or delete the app config) so onboarding runs.
2. In step 1 enter `/tmp/scan-demo`, depth ≥ 3, submit.
   **Expect (US1):** status line shows the directory currently walked and a count
   that ticks up; changes are visible within 2 s (SC-001); "Prüfe …" line never
   freezes for >0.5 s while work happens.
3. Click the "Suche läuft…" chip while still scanning.
   **Expect (US2):** modal opens with header (dir + count + elapsed), 41 rows
   appear/already present, none duplicated (SC-003); rows gain their type badge
   shortly after appearing (all "other" here).
4. Close the modal, let the scan finish.
   **Expect (US3):** wizard advances to the grouping step as before; chip becomes
   the summary and disappears after ≈10 s.

## 3. Refresh flow + multi-root attribution

1. Add two folders (Orderer "+") — `/tmp/scan-demo` and one real folder such as `/topdata/repoman`.
2. Click Refresh. Open the modal from the chip.
   **Expect (FR-010):** each row shows its root folder; header names the root being
   processed; after done, the normal table receives the same repos as today.

## 4. Failure path (SC-006)

1. Add a scan path, then remove the folder from disk just after starting a refresh
   (or point a path at a directory, then revoke read permission: `chmod 000`).
2. **Expect:** running state terminates, the chip/modal show a clear error summary,
   the empty-state spinner in `AllReposPage` is gone — nothing spins forever.

## 5. Web mode parity

```bash
pnpm build
cargo run --manifest-path src-tauri/Cargo.toml -- --web --port 8788
# open http://127.0.0.1:8788 (token if configured)
```

1. Trigger a refresh and open the modal.
   **Expect (FR-011):** identical live behavior over the single `/api/events` SSE
   stream (event name `scan-progress` visible in devtools → Network → EventStream).

## 6. Large-root sanity (SC-005, FR-013)

Scan a large real root (e.g. a home folder with `~/devel`, `~/.cache` excluded by
built-in rules). Observe: elapsed wall time is within ~5% of a pre-feature build
(`git stash` the change and compare), UI stays responsive, and with >500 found
repos the log shows the newest 500 with an "…und N mehr" row while the header
count remains exact.

## References

- Payload contract: [contracts/scan-progress-event.md](contracts/scan-progress-event.md)
- State model & transitions: [data-model.md](data-model.md)
- Mechanism rationale: [research.md](research.md), ADR__261101-1
