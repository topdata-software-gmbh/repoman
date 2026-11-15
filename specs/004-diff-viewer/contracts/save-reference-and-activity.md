# Contract: Save references and the activity drawer

The activity drawer ("What Git did") already lists Git runs from the typed step
stream. This contract adds a session-only association between a successful save
run and the exact commit it produced, so *Ansehen* opens the right saved state.

## C1. `OperationResult` extension (additive)

Existing payload:

```json
{
  "repoPath": "/srv/projects/theme",
  "success": true,
  "output": "…",
  "error": null
}
```

Commit payload after this feature:

```json
{
  "repoPath": "/srv/projects/theme",
  "success": true,
  "output": "…",
  "error": null,
  "runId": "uuid v4",
  "commitHash": "9f2c…"
}
```

- `runId` is set by `execute_macro` and matches the `runId` carried by that
  invocation's `GitStepEvent`s **for direct commit calls** (the Save Changes
  path). It may differ for group actions whose observer rewrites step run ids;
  group actions never produce `commitHash`.
- `commitHash` is set only by `commit_changes` after a successful commit, from
  `git rev-parse HEAD`.
- Both fields are optional/default-`null`, camelCase, and ignored by existing
  consumers. No existing field changes meaning.

## C2. Capture rule

1. `service::commit_repo_changes` → `GitRunner::commit_changes`.
2. The commit macro runs as today (`add -A`, `commit -m <message>`).
3. On success, the core reads `git rev-parse HEAD` and returns it as
   `commitHash` with the run's `runId`.
4. On failure, both new fields may be `null`; no save reference is stored.
5. The core performs no extra write and no history rewrite.

`repoStore.commitRepositories` records, for each successful result carrying both
fields:

```text
saveRefs.set(runId, {
  runId,
  repoPath,
  commitHash,
  savedAt: new Date().toISOString()
})
```

The map is session-only and is not cleared when the viewer closes.

## C3. Activity drawer mapping

- `GitLogDrawer.vue` continues to render runs from `store.gitSteps`.
- A run with `macroId === "commit"` gets a translated **Ansehen** action.
- If `saveRefs` has the run's `runId`, the action opens saved-state review with
  that `commitHash`.
- If no reference exists (e.g. a run recorded before hash capture), the action
  falls back to saved-state review for the project's current `HEAD`. If no
  `HEAD` exists, the viewer shows the unavailable notice.
- Opening the review does not close the drawer permanently; closing the modal
  returns to the drawer with its expanded state intact.

## C4. Saved-state resolution

For `mode: "save"` the core resolves:

1. `commitHash` supplied by the request, otherwise `HEAD`.
2. Verify with `git rev-parse --verify <hash>^{commit}`.
3. Read the commit subject/time with `git show -s --format=%s%x00%ct <hash>`.
4. Determine the parent with `git rev-list --parents -n 1 <hash>`.
   - A root commit diffs against the empty tree
     `4b825dc642cb6eb9a060e54bf8d69288fbee4904`.
5. Build the inventory from `<parent>..<hash>`; the "before" side is the parent,
   the "after" side is the commit.
6. The same resolved pair is used for the summary and every file preview.

## C5. Unavailable states

| Condition | `review_project` result | UI copy |
|---|---|---|
| Hash no longer resolves (rewritten/garbage-collected history) | `state: "unavailable"`, `error.code: "save-not-found"` | plain-language "This saved state is no longer available." |
| Repository has no commits (`HEAD` unborn) | `state: "unavailable"`, `error.code: "head-not-found"` | plain-language "There is nothing saved to review yet." |
| Repository path missing/unreadable | command error or `state: "unreadable"` | project error naming the project and what to do next |
| File existed in the summary but not in the resolved commit | `review_file` returns `notice: "missing"` | inline "This file is no longer available." |

No unavailable case throws an unhandled error dialog (SC-007).

## C6. Compatibility and security

- Existing `commit_repo_changes` callers keep working; the extra fields are
  ignored until the store reads them.
- Web JSON and Tauri IPC use the same `OperationResult` shape (Principle II).
- `commitHash` is a public revision identifier, not a secret; no credential,
  environment variable, or file content is added to the result.
- The activity log and save references remain in memory only (ADR__261030-4);
  nothing is written to `AppConfig` or disk.
