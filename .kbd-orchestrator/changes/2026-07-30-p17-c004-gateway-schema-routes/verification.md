# Verification — p17-c004

## Gate (FFS-001 §8 Phase 3)
Route tests cover 401/401/403/503/200-plan/200-apply/200-alreadyApplied/
409-drift/403-reserved — with the 503-not-404 disabled-state pin.

## Evidence to record on completion
- [ ] `cargo check -p fdb-gateway` clean
- [ ] Disabled deployment returns 503 with FFS-001 §4.1 error body (test name)
- [ ] Drift test mutates live schema between plan and apply and gets 409
- [ ] applied_by lands JWT sub in ledger; grep shows raw_bearer never passed
      beyond require_provisioner
- [ ] OpenAPI group renders (openapi.json contains /schema/v1 paths)

## Implementation notes (2026-07-30)
- Route group lives at `fdb_gateway::schema_api` on the LIBRARY target
  (keto_sync precedent) so integration tests construct the real router;
  bootstrap consumes it. `routes/schema/` path in tasks superseded.
- The plan hash was redefined during this change to cover spec + generated
  DDL: a spec-only hash trivially matches its own re-plan and can never
  detect drift. Regression test: hash_detects_live_schema_drift (fdb-app).
- OpenAPI convention: openapi.json is reflection-compiled only; hand-written
  groups document in runbook §1.4 (three rows added) — matching every other
  hand-written group.

## Adversarial-review dispositions (2026-07-30)
- Replay-path gating: FIXED — require_allowlisted now precedes every status
  branch, so a withdrawn namespace refuses even the informational
  alreadyApplied response. Expiry is deliberately NOT checked for applied
  rows (an applied plan's TTL is meaningless; expiry protects unapplied
  plans from drift, which the hash guard covers anyway).
- clippy allows: cast allow REMOVED (it was needless — no casts in
  epoch_to_iso8601); too_many_lines allow kept with expanded justification
  per the constraints.md justified-comment rule.
- OpenAPI: REFUTED by repo convention — openapi.json is reflection-compiled
  only; no hand-written group (a2ui/mcp/a2a/htmx/agui) appears in it, and
  all document in runbook §1.4, which this change extended. Diverging for
  one group would break the uniform convention.
