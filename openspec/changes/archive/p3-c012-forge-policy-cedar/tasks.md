# Tasks — p3-c012-forge-policy-cedar

- [x] 1. Add `cedar_policies` table SQL (new migration in `crates/ext-flint-meta/sql/`)
- [x] 2. Add `cedar-policy = "4"` to `[workspace.dependencies]` and `forge-policy/Cargo.toml`
- [x] 3. Implement `CedarPolicyEngine` in `forge-policy/src/cedar.rs` (< 500 lines)
- [x] 4. Implement `PolicyLoader` (privileged pool, ArcSwap cache, hot-reload hook)
- [x] 5. Wire `CedarPolicyEngine` as `Pep` impl; fail-closed on load/compile/eval error
- [x] 6. Add `#[tracing::instrument(skip(self, who))]` — never log principal/policy body
- [x] 7. Unit tests: allow, deny, malformed-policy-deny, missing-table-deny
- [x] 8. Inject `Arc<dyn Pep>` into `fdb-app` mutation use-cases
- [x] 9. `cargo check --workspace` + clippy + `cargo test -p forge-policy -p fdb-app`
