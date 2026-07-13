# p11-c002 Tasks — Kiln ABI Freeze

## Tasks

- [x] Add `@since(version = 0.1.0)` to `interface db` in `wit/flint/host/world.wit`
- [x] Add `@since(version = 0.1.0)` to `interface llm`
- [x] Add `@since(version = 0.1.0)` to `interface kv`
- [x] Add `@since(version = 0.1.0)` to `interface identity`
- [x] Add `@since(version = 0.1.0)` to `interface secrets`
- [x] Add stability comment block to the `world edge-function` declaration
- [ ] Run `cargo component build -p hello-component` — verify WIT still parses cleanly — OPEN: `cargo-component` toolchain is not installed in this environment; WIT is well-formed by inspection but this build step has no evidence it was ever actually run
- [x] Write `docs/api/kiln-abi.md` — skill-author reference (~200 lines) covering: world overview, wasi:http contract, all 5 interfaces, fuel/epoch limits, Cedar flow, ContentId, store backends, versioning policy — 14KB, all 13 outline sections present
- [x] Add `FLINT_KILN_ABI_VERSION=1` to `.env.example` with comment — `.env.example:104`
- [x] `cargo check --workspace` clean
- [x] `cargo test --workspace` passes
