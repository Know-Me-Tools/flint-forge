# Tasks — p3-c012-forge-policy-cedar

- [ ] 1. Add `cedar_policies` table SQL (new migration in `crates/ext-flint-meta/sql/`)
- [ ] 2. Add `cedar-policy = "4"` to `[workspace.dependencies]` and `forge-policy/Cargo.toml`
- [ ] 3. Implement `CedarPolicyEngine` in `forge-policy/src/cedar.rs` (< 500 lines)
- [ ] 4. Implement `PolicyLoader` (privileged pool, ArcSwap cache, hot-reload hook)
- [ ] 5. Wire `CedarPolicyEngine` as `Pep` impl; fail-closed on load/compile/eval error
- [ ] 6. Add `#[tracing::instrument(skip(self, who))]` — never log principal/policy body
- [ ] 7. Unit tests: allow, deny, malformed-policy-deny, missing-table-deny
- [ ] 8. Inject `Arc<dyn Pep>` into `fdb-app` mutation use-cases
- [ ] 9. `cargo check --workspace` + clippy + `cargo test -p forge-policy -p fdb-app`
