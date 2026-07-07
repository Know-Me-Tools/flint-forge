# p11-c001 Tasks — A2UI API Freeze

## Tasks

- [ ] Add `#[non_exhaustive]` to `AgUiEvent` — `crates/fdb-domain/src/lib.rs:108`
- [ ] Add `#[non_exhaustive]` to `ParseError` — `crates/fdb-app/src/a2ui/design_md_parser.rs:67`
- [ ] Add `#[non_exhaustive]` to `ReflectionError` — `crates/fdb-reflection/src/error.rs:4`
- [ ] Add `#[non_exhaustive]` to `EndpointKind` — `crates/fdb-reflection/src/passes/endpoint_generation.rs:12`
- [ ] Add `#[non_exhaustive]` to `AssemblerError` — `crates/fdb-reflection/src/compilers/a2ui.rs:18`
- [ ] Add `#[non_exhaustive]` to `Capability` — `crates/fke-domain/src/lib.rs:12`
- [ ] Add `#[non_exhaustive]` to `TargetArch` — `crates/fke-domain/src/lib.rs:30`
- [ ] Add `#[non_exhaustive]` to `Decision` — `crates/forge-policy/src/lib.rs:16`
- [ ] Add `#[non_exhaustive]` to `PolicyLoadError` — `crates/forge-policy/src/cedar.rs:47`
- [ ] Fix any exhaustive `match` arms broken by new `#[non_exhaustive]` attributes
- [ ] `mkdir -p docs/api/`
- [ ] Write `docs/api/a2ui.md` — public API reference (versioning policy, component schema, 10 endpoint contracts, auth, errors)
- [ ] Add `FLINT_A2UI_API_VERSION=1` to `.env.example` with comment
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
