# p11-c002 Tasks — Kiln ABI Freeze

## Tasks

- [ ] Add `@since(version = 0.1.0)` to `interface db` in `wit/flint/host/world.wit`
- [ ] Add `@since(version = 0.1.0)` to `interface llm`
- [ ] Add `@since(version = 0.1.0)` to `interface kv`
- [ ] Add `@since(version = 0.1.0)` to `interface identity`
- [ ] Add `@since(version = 0.1.0)` to `interface secrets`
- [ ] Add stability comment block to the `world edge-function` declaration
- [ ] Run `cargo component build -p hello-component` — verify WIT still parses cleanly
- [ ] Write `docs/api/kiln-abi.md` — skill-author reference (~200 lines) covering: world overview, wasi:http contract, all 5 interfaces, fuel/epoch limits, Cedar flow, ContentId, store backends, versioning policy
- [ ] Add `FLINT_KILN_ABI_VERSION=1` to `.env.example` with comment
- [ ] `cargo check --workspace` clean
- [ ] `cargo test --workspace` passes
