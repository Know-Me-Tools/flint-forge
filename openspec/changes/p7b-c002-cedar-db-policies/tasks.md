# p7b-c002 Tasks — Cedar DB Policies

## Tasks

- [ ] Write `migrations/0009_flint_kiln_cedar_policies.sql` — table definition + bootstrap allow-all row
- [ ] Create `crates/fke-server/src/kiln_db_policy.rs` — `DbKilnPolicySource` loading from `flint_kiln.cedar_policies`
- [ ] Declare `mod kiln_db_policy;` in `fke-server/src/main.rs`
- [ ] Replace `AllowAllPolicySource` with `DbKilnPolicySource::new(pool.clone())` in `fke-server/src/main.rs`
- [ ] Keep `kiln_policy.rs` (or rename to `kiln_policy_test.rs`) for unit tests that need an in-memory policy source
- [ ] Unit test: `DbKilnPolicySource::load()` returns error when pool is disconnected (no DB needed — use `connect_lazy`)
- [ ] `cargo clippy -p fke-server -- -D warnings` clean
- [ ] `cargo test -p fke-server` passes
