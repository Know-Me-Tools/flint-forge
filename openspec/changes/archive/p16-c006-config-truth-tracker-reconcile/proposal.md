# p16-c006 — Config Truth + Tracker Reconcile

**Phase:** 16 — Production Remediation
**Priority:** P1
**Depends on:** p16-c001 (doc corrections must describe the post-fix RLS behavior)

## What this change delivers

- `ext-flint-hooks`'s `agui_run` webhook target is configurable, not
  hardcoded to `http://localhost:8080`.
- Doc-comments that falsely describe the codebase's security posture are
  corrected.
- `openspec/changes/p9-c*` through `p14-c*` task checkboxes are reconciled
  against what actually shipped.

## Problem

`crates/ext-flint-hooks/sql/flint_hooks.sql:156` hardcodes the `agui_run`
webhook target URL to `http://localhost:8080/agents/v1/...` — this will not
work in any real multi-host deployment.

`crates/fdb-reflection/src/compilers/rest/mod.rs:62` claims "CRUD handlers
remain `todo!()` stubs pending the query-builder landing" — false; they are
fully implemented. `mod.rs:120-122` claims "RLS is enforced by the
connection's GUC context — this handler adds no extra GUC work" — false before
p16-c001, and must be corrected to accurately describe the fix afterward.
These false claims are exactly what let the RLS bypass ship through a prior
"production readiness" phase undetected.

Separately, `openspec/changes/p9-c001` through `p14-c005` `tasks.md` files are
**near-universally unchecked** (`- [ ]`) despite the referenced work having
shipped (verified during the 2026-07-12 audit: docker-compose.prod.yml,
security headers, rate limiting, docs, etc. all exist). A tracker that
contradicts the code cannot be used to gauge readiness — this is a
process-integrity defect independent of security.

## Design

### 1. Configurable `agui_run` target

Add a `flint.webhooks` column or config table entry for the AG-UI base URL
(env-driven at extension init, or a `flint.settings` row), replacing the
hardcoded string. Keep `localhost:8080` as the **default** for local dev, but
make it overridable per-deployment.

### 2. Doc-comment corrections

Audit and rewrite every doc-comment identified in the 2026-07-12 audit
(`docs/audits/2026-07-12-production-readiness.md`) that describes stale or
false behavior — not just the two called out above; grep the flagged crates
for similar drift (search for "GUC context", "todo!()" references in doc
comments, "not yet read by any handler" near `main.rs:56-57`).

### 3. Tracker reconcile

For each `openspec/changes/p9-c*` … `p14-c*`, check the actual code/artifacts
against the `tasks.md` checklist and update checkboxes to reflect reality.
Where a task genuinely wasn't done, leave it unchecked and note it as
still-open debt (don't rubber-stamp). This is a checkpoint pass — a final
reconcile should happen again once p16-c007–c009 land (see caveat in
`plan.md`).

## Verification (gate)

- `grep -r "localhost:8080" crates/ext-flint-hooks/sql/` returns nothing on a
  deployable path (default-with-override is fine; a literal hardcode is not).
- `grep` for the specific false doc-comment strings identified in the audit
  returns nothing.
- Manual review: each `p9`–`p14` change's checkbox state matches a concrete
  artifact/commit; spot-check at least 5 changes across different phases.
