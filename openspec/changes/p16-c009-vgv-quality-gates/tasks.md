# p16-c009 Tasks — VGV Enterprise Quality Gates

## Tasks

- [x] Add `cargo-llvm-cov` to CI with ≥90% threshold on changed crates
- [x] Track workspace-wide coverage as a visible metric (even if not gated at 90% yet)
- [x] Add `deny.toml` (licenses + advisories) matching the project's MIT posture
- [x] Add `cargo-deny check` as a CI step alongside existing `cargo audit`
- [x] Add `#![deny(missing_docs)]` to `forge-domain` (start smallest/most stable) + fill in `///` docs
- [ ] Roll `#![deny(missing_docs)]` out to remaining library crates incrementally, one PR per crate or small group — deliberately left open: the proposal itself is explicit ("Do not do this in one giant PR — it will generate enormous, low-review-value diffs; batch by crate"). `forge-domain` (task above) is the first crate; every other library crate (`fdb-domain`, `fdb-ports`, `fke-domain`, `fke-ports`, `forge-identity`, `forge-policy`, etc.) still needs its own incremental pass. Tracked as ongoing follow-up work, not silently dropped.
- [ ] Add `# Errors` rustdoc sections to fallible public functions as `missing_docs` is enabled per crate — N/A so far (`forge-domain`'s only public function, `is_safe_identifier`, returns `bool`, not `Result`); applies as each future crate's `missing_docs` pass (above) lands on a crate with fallible public functions.
- [x] Run `cargo tree --workspace --duplicates`; align duplicate versions via `[workspace.dependencies]` where compatible
- [x] Document any unresolvable duplicate dependency with a reason
- [x] Re-run `unsafe` grep across non-pgrx crates after p16-c001–c008 land (recount from the 19 baseline)
- [x] Add `// SAFETY:` justification comment to every `unsafe` block
- [x] Replace any `unsafe` block that has a safe alternative
- [x] Classify each of the 47 `.ok()`/`let _ =` error-swallow sites: fire-and-forget (comment), should-propagate (fix), or should-log (add tracing)
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
