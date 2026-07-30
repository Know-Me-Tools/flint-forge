# Verification — p17-c001

## Gate (FFS-001 §8 Phase 0)
The existing service_role key authenticates against a protected route and
resolves to role="service_role"; the anon key resolves to role="anon"; the
migration applies cleanly and is idempotent on re-run.

## Evidence to record on completion
- [ ] `cargo check -p fdb-gateway` clean after test addition
- [ ] 0015 applied twice against scratch DB — second run no-op, no errors
- [ ] Auth-proof test compiles; run deferred to phase boundary (c006), run
      record to be added there
- [ ] .env.example + runbook + ANON-SERVICE-ROLE-KEYS.md diffs reviewed for
      accuracy against forge-identity source (no aspirational claims)
