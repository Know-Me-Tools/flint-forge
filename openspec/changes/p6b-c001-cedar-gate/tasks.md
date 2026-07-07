# p6b-c001 Tasks — Cedar Capability Gate

## Tasks

- [ ] Add `crates/forge-policy/src/kiln.rs` — `KILN_RESOURCE`, `KILN_INVOKE`, `request()` helper, `KilnPep` convenience trait (mirrors `a2ui.rs`)
- [ ] Export `kiln` module from `crates/forge-policy/src/lib.rs`
- [ ] Add `pep: Option<Arc<dyn Pep>>` field to `EdgeRuntime`; add `with_pep()` builder method
- [ ] In `EdgeRuntime::handle()`: call `pep.check(who, &kiln::request(KILN_INVOKE))` before `check_capabilities()`; return `Err(anyhow!("policy denied"))` on `Decision::Deny`
- [ ] Add `forge-policy` as a dep to `fke-runtime/Cargo.toml` (already a workspace dep)
- [ ] Update `fke-server/src/main.rs`: construct `CedarPolicyEngine::new(policy_source)` and pass via `runtime.with_pep(pep)`
- [ ] Unit test: `pep = None` → capability list comparison only (backward-compatible)
- [ ] Unit test: `pep = Some(deny-all)` → `handle()` returns error before instantiation
- [ ] `cargo clippy -p fke-runtime -p forge-policy -- -D warnings` clean
