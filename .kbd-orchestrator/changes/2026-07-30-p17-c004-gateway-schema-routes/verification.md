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
