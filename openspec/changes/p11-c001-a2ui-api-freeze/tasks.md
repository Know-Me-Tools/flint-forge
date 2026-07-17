# p11-c001 Tasks — A2UI API Freeze

## Tasks

- [x] Add `#[non_exhaustive]` to `AgUiEvent` — `crates/fdb-domain/src/lib.rs:105`
- [x] Add `#[non_exhaustive]` to `ParseError` — `crates/fdb-app/src/a2ui/design_md_parser.rs:66`
- [x] Add `#[non_exhaustive]` to `ReflectionError` — `crates/fdb-reflection/src/error.rs:2`
- [x] Add `#[non_exhaustive]` to `EndpointKind` — `crates/fdb-reflection/src/passes/endpoint_generation.rs:11`
- [x] Add `#[non_exhaustive]` to `AssemblerError` — `crates/fdb-reflection/src/compilers/a2ui.rs:16`
- [x] Add `#[non_exhaustive]` to `Capability` — `crates/fke-domain/src/lib.rs:11`
- [x] Add `#[non_exhaustive]` to `TargetArch` — `crates/fke-domain/src/lib.rs:30`
- [x] Add `#[non_exhaustive]` to `Decision` — `crates/forge-policy/src/lib.rs:15`
- [x] Add `#[non_exhaustive]` to `PolicyLoadError` — `crates/forge-policy/src/cedar.rs:46`
- [x] Fix any exhaustive `match` arms broken by new `#[non_exhaustive]` attributes — `cargo check --workspace` exits 0
- [x] `mkdir -p docs/api/` — exists (a2ui.md, kiln-abi.md, versioning.md)
- [x] Write `docs/api/a2ui.md` — public API reference (versioning policy, component schema, 10 endpoint contracts, auth, errors) — 541 lines, all required sections present
- [x] Add `FLINT_A2UI_API_VERSION=1` to `.env.example` with comment — `.env.example:115`
- [x] `cargo clippy --workspace -- -D warnings` clean
- [x] `cargo test --workspace` passes
