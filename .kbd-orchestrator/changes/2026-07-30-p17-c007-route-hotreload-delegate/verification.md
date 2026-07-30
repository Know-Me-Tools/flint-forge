# Verification — p17-c007 (OPTIONAL)

## Gate (FFS-001 §8 Phase 5)
A table created via /schema/v1/apply is reachable at /{ns}/{table} WITHOUT
restart, with mounted-route p99 regression <5% and delegated overhead <1ms
p99 — or the change closes SKIPPED with the benchmark numbers recorded.

## Evidence to record on completion
- [ ] Benchmark table (before/after, p50/p99, request counts, host/build)
- [ ] Ship / skip decision + rationale
- [ ] If shipped: no-restart reachability test green; require_rls parity test
      green; restartRequired flipped
