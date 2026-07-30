# Verification — p17-c002

## Gate (FFS-001 §8 Phase 1)
cargo test -p fdb-app green at the phase boundary, including DDL snapshots and
negative tests for every injection shape; clippy pedantic clean.

## Evidence to record on completion
- [ ] `cargo check -p fdb-domain -p fdb-app` clean
- [ ] Injection corpus enumerated in test names (each shape its own test)
- [ ] Snapshot files reviewed: RLS block verbatim vs FFS-001 §5 template
- [ ] Hash stability test covers field reordering AND table/column order
- [ ] No public item undocumented (missing_docs deny compiles)
