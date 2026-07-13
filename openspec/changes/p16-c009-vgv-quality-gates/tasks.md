# p16-c009 Tasks — VGV Enterprise Quality Gates

## Tasks

- [ ] Add `cargo-llvm-cov` to CI with ≥90% threshold on changed crates
- [ ] Track workspace-wide coverage as a visible metric (even if not gated at 90% yet)
- [ ] Add `deny.toml` (licenses + advisories) matching the project's MIT posture
- [ ] Add `cargo-deny check` as a CI step alongside existing `cargo audit`
- [ ] Add `#![deny(missing_docs)]` to `forge-domain` (start smallest/most stable) + fill in `///` docs
- [ ] Roll `#![deny(missing_docs)]` out to remaining library crates incrementally, one PR per crate or small group
- [ ] Add `# Errors` rustdoc sections to fallible public functions as `missing_docs` is enabled per crate
- [ ] Run `cargo tree --workspace --duplicates`; align duplicate versions via `[workspace.dependencies]` where compatible
- [ ] Document any unresolvable duplicate dependency with a reason
- [ ] Re-run `unsafe` grep across non-pgrx crates after p16-c001–c008 land (recount from the 19 baseline)
- [ ] Add `// SAFETY:` justification comment to every `unsafe` block
- [ ] Replace any `unsafe` block that has a safe alternative
- [ ] Classify each of the 47 `.ok()`/`let _ =` error-swallow sites: fire-and-forget (comment), should-propagate (fix), or should-log (add tracing)
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
