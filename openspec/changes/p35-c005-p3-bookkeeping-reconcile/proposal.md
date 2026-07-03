# p35-c005 — Reconcile p3-auth-rls-keto bookkeeping

## Change ID
`p35-c005-p3-bookkeeping-reconcile`

## Phase
`p3.5-ci-postgres-hardening`

## Goal Mapping
**G5** — record delivered work; resolve superseded/overlapping changes.

## Depends on
None (docs/state only; can run any time — sequenced last so it captures final state).

## Problem
`p3-auth-rls-keto/progress.json` tracks c010–c018 with c017/c018 `pending`, but the
delivered-and-merged work landed as **untracked** c019 (PostgREST engine), c020 (LISTEN
source), and the G4 subscription seam. c017 (FRF reconnect stub) is **superseded** by
c020's alternative backend; c018 (introspection-verify) overlaps already-merged work.
`openspec/changes/p3-c019-postgrest-query-engine/` is unarchived.

## Scope (bookkeeping only — no product code)
- In `p3-auth-rls-keto/progress.json`: add c019 + c020 as delivered (`qa_passed`,
  `completed_by: claude-code`) with a note that they were the actual G3/G4/G5/G7-alt
  deliverables; update `changes_completed`/`changes_total` accordingly.
- Mark **c017** `superseded` (by `p3-c020` / the in-process LISTEN backend); mark
  **c018** `resolved` (introspection merge shipped + verified via the c016 gate + G4 seam).
- Archive `openspec/changes/p3-c019-postgrest-query-engine/` to `openspec/changes/archive/`.
- (c020 had no openspec dir — it was branch/commit-only; optionally backfill an archive
  proposal for the record, or note it's commit-tracked only.)

## Out of Scope
- Any code change. Any change to the p3 reflection (already written).

## Acceptance Criteria
- [ ] `p3-auth-rls-keto/progress.json` reflects c019/c020 as delivered and c017/c018 resolved/superseded.
- [ ] `openspec/changes/p3-c019-*` archived.
- [ ] The p3 phase can be read as coherently closed (delivered set matches `main`).
