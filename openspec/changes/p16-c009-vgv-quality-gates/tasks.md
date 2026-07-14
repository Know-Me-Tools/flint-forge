# p16-c009 Tasks — VGV Enterprise Quality Gates

## Tasks

- [x] Add `cargo-llvm-cov` to CI with ≥90% threshold on changed crates
- [x] Track workspace-wide coverage as a visible metric (even if not gated at 90% yet)
- [x] Add `deny.toml` (licenses + advisories) matching the project's MIT posture
- [x] Add `cargo-deny check` as a CI step alongside existing `cargo audit`
- [x] Add `#![deny(missing_docs)]` to `forge-domain` (start smallest/most stable) + fill in `///` docs
- [x] Roll `#![deny(missing_docs)]` out to remaining library crates incrementally, one PR per crate or small group — completed via 9 parallel review agents, each verifying its assigned crate(s) with `cargo check`/`cargo clippy --all-targets -- -D warnings` before moving on: `fdb-domain`, `fdb-ports`, `fdb-auth`, `fdb-app`, `fdb-query`, `fdb-postgres`, `fdb-realtime`, `fdb-reflection`, `fdb-gateway` (library target only — its large `routes/`/`bootstrap`/`handlers` modules live in the binary target, which `missing_docs` doesn't gate), `fke-domain`, `fke-ports`, `fke-registry`, `fke-runtime`, `fke-sign-did`, `fke-sign-cosign`, `fke-store-{fs,ipfs,oci,s3}`, `forge-identity`, `forge-policy`, `flint-skill`. All 22 crates now enforce the lint; several agents also ran `cargo doc --no-deps` to catch and fix broken intra-doc links beyond the lint's own requirements.
- [x] Add `# Errors` rustdoc sections to fallible public functions as `missing_docs` is enabled per crate — done for every crate above as part of the same pass; each fallible public function's `# Errors` section enumerates real failure paths read from the implementation, not generic boilerplate. `# Panics` sections were added where genuinely applicable (e.g. `fke-runtime`'s poisoned-mutex/engine-init paths).
- [x] Run `cargo tree --workspace --duplicates`; align duplicate versions via `[workspace.dependencies]` where compatible
- [x] Document any unresolvable duplicate dependency with a reason
- [x] Re-run `unsafe` grep across non-pgrx crates after p16-c001–c008 land (recount from the 19 baseline)
- [x] Add `// SAFETY:` justification comment to every `unsafe` block
- [x] Replace any `unsafe` block that has a safe alternative
- [x] Classify each of the 47 `.ok()`/`let _ =` error-swallow sites: fire-and-forget (comment), should-propagate (fix), or should-log (add tracing)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
