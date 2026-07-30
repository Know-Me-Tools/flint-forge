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

## CLOSED: SKIPPED-OPTIONAL (2026-07-30, per plan D-P7)
Not attempted this session — no benchmark was run, so per the ship
criterion the change closes without flipping restartRequired. The v1
contract remains the honest `restartRequired: true` + restartNote on every
apply response (D8), verified by the route tests. The delegate remains
specced in this change dir for a future session; its gate (mounted p99
regression <5%, delegated overhead <1ms p99) is unchanged. "An honest flag
beats a slow gateway" — and beats an unbenchmarked delegate too.
