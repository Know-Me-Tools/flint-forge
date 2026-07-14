# p14-c001 Tasks — sqlx 0.9 Upgrade + Pool Prometheus Metrics

## Tasks

- [x] Bump `sqlx = "0.8"` → `sqlx = "0.9"` in `[workspace.dependencies]` of `Cargo.toml`
- [x] Run `cargo check --workspace`; fix any API changes from 0.8→0.9
- [x] Run `cargo update generic-array`; confirm pgvector traits still unify
- [x] Run `cargo test -p fdb-reflection`; confirm vector RPC tests pass
- [x] Add `metrics = "0.24"` to `crates/fdb-gateway/Cargo.toml`
- [x] Add `spawn_pool_metrics(pool)` function to `crates/fdb-gateway/src/telemetry.rs`
- [x] Call `telemetry::spawn_pool_metrics(pool.clone())` in `main()` after pool creation
- [x] Verify: `cargo check -p fdb-gateway`; `metrics::gauge!` calls compile
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
- [x] `cargo audit` clean

## Execution notes (2026-07-14, p16-c006 reconcile)

Part A (the sqlx bump itself) had never actually been executed — only Part B
(pool metrics) had shipped, despite this file showing all boxes unchecked.
This pass performed Part A for real:

- Bumped `sqlx = "0.8"` → `"0.9"` in the workspace `[workspace.dependencies]`
  (`Cargo.toml`) and in `crates/fdb-app/Cargo.toml`'s dev-dependency (which
  pinned its own version instead of using `workspace = true`).
- `crates/fdb-app`'s dev-dependency also used the combined
  `runtime-tokio-native-tls` feature flag, which sqlx 0.9 no longer exposes as
  a single feature; split into `runtime-tokio` + `tls-native-tls`.
- The 0.8→0.9 bump introduced one real breaking change beyond the version
  number: a new `SqlSafeStr` trait bound on `sqlx::query`/`sqlx::raw_sql` that
  requires every dynamic (non-`'static`) SQL string to be explicitly wrapped
  in `sqlx::AssertSqlSafe(..)`, auditing it as injection-safe. This is a
  compile-time SQL-injection guard sqlx added upstream in 0.9, not
  Flint-Forge-specific breakage. Fixed at every call site it flagged:
  - `crates/forge-cli/src/main.rs` (`hook_add`): the DDL trigger-name string
    previously spliced `schema`/`table` into `CREATE TRIGGER` unescaped and
    unvalidated — genuinely nearer to a real SQL-injection gap than the
    others below. Added `is_safe_identifier` validation (matching the
    chokepoint already used throughout `fdb-query`/`fdb-reflection`) before
    building the DDL string, then wrapped with `AssertSqlSafe`. Added
    `forge-domain` as a `forge-cli` dependency for this.
  - `crates/fdb-reflection/src/compilers/rest/{mutations.rs,rpc.rs,mod.rs}`
    (5 call sites): all already validated schema/table/column identifiers via
    `is_safe_identifier` before interpolation and bound every value as `$n`
    per their own pre-existing `SECURITY:` doc comments — wrapped with
    `AssertSqlSafe` with a `SAFETY:` comment citing the prior validation.
  - `crates/fdb-reflection/tests/{embedding_live_pg.rs,rest_typed_columns_live_pg.rs}`
    and `crates/fdb-app/tests/meta_listener.rs` (9 call sites total): test-only
    SQL built from hardcoded literal schema names or a numeric epoch-suffixed
    table name — wrapped with `AssertSqlSafe`.
- `cargo update -p generic-array` resolves to the same 0.14.7 with no
  transitive conflict — the proposal's `generic-array` concern does not
  reproduce with sqlx 0.9 in this workspace's current dependency graph.
- `cargo test -p fdb-reflection`: all 5 `tests/pgvector_rpc.rs` tests pass
  (vector-arg reflection, JSON↔vector binding, extension-version check).
  `cargo test --workspace`: 74 test binaries, all `ok`, 0 failed
  (`DATABASE_URL` was set in this environment, so the live-Postgres-gated
  suites — including `rest_typed_columns_live_pg.rs` — ran for real, not
  skipped).
- `cargo audit`: 0 advisories (only unrelated `spin` yanked-crate warnings).
  The sqlx 0.9 bump dropped the `rsa` crate from `sqlx-mysql`'s dependency
  tree entirely, which was the sole source of the allowlisted
  RUSTSEC-2023-0071. Verified by re-running `cargo audit` with that entry
  excluded from the ignore list — still zero advisories, confirming the
  advisory doesn't just go unreported, it no longer applies. Removed the
  `"RUSTSEC-2023-0071"` entry from `.cargo/audit.toml`'s `[advisories].ignore`
  (comment left in place explaining the removal). The other three allowlisted
  advisories (quick-xml/object_store, tokio-tar, rustls-pemfile) are unrelated
  to sqlx and remain.
