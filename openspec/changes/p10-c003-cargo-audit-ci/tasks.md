# p10-c003 Tasks — Dependency CVE Remediation + `cargo audit` CI Gate

## Tasks

- [x] Bump `wasmtime = "46"`, `wasmtime-wasi = "46"`, `wasmtime-wasi-http = "46"` in `[workspace.dependencies]` of `Cargo.toml`
- [x] Run `cargo check -p fke-runtime -p fke-server`; fix all compilation errors from wasmtime API changes in `fke-runtime/src/lib.rs`
- [x] Bump `object_store = "0.14"` in `[workspace.dependencies]`; run `cargo check -p fke-store-s3`; fix any API changes
- [x] Run `cargo audit`; confirm CVSS ≥ 7.0 advisories are gone — p16-c006 reconcile note: NOT fully accurate as stated (see still-open item below); wasmtime's advisories are genuinely gone, but two CVSS-7.5 quick-xml advisories remain, allowlisted rather than eliminated
- [x] Write `.cargo/audit.toml` with justified allowlist for remaining no-fix advisories (`RUSTSEC-2023-0071` rsa/sqlx-mysql; any unmaintained with no security impact)
- [x] Add `Security audit` step to `.github/workflows/ci.yml` after `test` step
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes

## Still-open debt (p16-c006 reconcile, 2026-07-13)

- [ ] `RUSTSEC-2026-0194`/`RUSTSEC-2026-0195` (quick-xml, CVSS 7.5, pulled in transitively via `object_store`) are still present in `Cargo.lock` (`quick-xml 0.40.1`) and are suppressed via `.cargo/audit.toml`'s allowlist ("Fix blocked on object_store releasing with quick-xml >=0.41.0"), not eliminated as the original task implied. Re-check when `object_store` ships a release pulling in `quick-xml >= 0.41.0`.

<!-- p16-c006 reconcile (2026-07-13): verified against Cargo.toml/Cargo.lock, .cargo/audit.toml, .github/workflows/ci.yml, and cargo check -p fke-runtime -p fke-server -p fke-store-s3. Corrected the "CVSS >= 7.0 advisories are gone" claim to match the allowlist reality rather than rubber-stamping it. -->
