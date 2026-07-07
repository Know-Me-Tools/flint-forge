# p10-c003 Tasks — Dependency CVE Remediation + `cargo audit` CI Gate

## Tasks

- [ ] Bump `wasmtime = "46"`, `wasmtime-wasi = "46"`, `wasmtime-wasi-http = "46"` in `[workspace.dependencies]` of `Cargo.toml`
- [ ] Run `cargo check -p fke-runtime -p fke-server`; fix all compilation errors from wasmtime API changes in `fke-runtime/src/lib.rs`
- [ ] Bump `object_store = "0.14"` in `[workspace.dependencies]`; run `cargo check -p fke-store-s3`; fix any API changes
- [ ] Run `cargo audit`; confirm CVSS ≥ 7.0 advisories are gone
- [ ] Write `.cargo/audit.toml` with justified allowlist for remaining no-fix advisories (`RUSTSEC-2023-0071` rsa/sqlx-mysql; any unmaintained with no security impact)
- [ ] Add `Security audit` step to `.github/workflows/ci.yml` after `test` step
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
