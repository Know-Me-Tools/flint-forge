# AGENTS.md

Guidance for **any** AI coding agent working in this repository (Claude Code, Codex,
Cursor, Roo, Cline, Kilo, Windsurf, OpenCode, and others coordinated via KBD).

**Canonical instructions live in [`CLAUDE.md`](./CLAUDE.md).** Read it in full — it
defines what this repo is, the hexagonal dependency rule, the design contracts, the
CI-enforced quality gates, and the 20 Prometheus Base Rules. This file restates the
cross-cutting **development-management** policy so every tool applies it identically.

Full detail for this policy: [`docs/RUST-DEVELOPMENT-MANAGEMENT.md`](./docs/RUST-DEVELOPMENT-MANAGEMENT.md).

---

## Build & check commands

```bash
cargo check --workspace          # preferred loop — front-end only, ~10× faster than build
cargo check -p <crate>           # scope to one crate while iterating
cargo clippy --workspace -- -D warnings   # the CI gate
cargo fmt --all
cargo test -p <crate>            # phase-boundary action — not during a phase
cargo run -p fdb-gateway         # Quarry gateway
cargo run -p fke-server          # Kiln server
./scripts/ci-check.sh            # CI script
```

pgrx extensions (`ext-flint-*`) are excluded from the default workspace and built
separately with `cargo pgrx` — see `CLAUDE.md`.

---

## Development Management (binding for all agents)

### Integration-First Delivery
Nothing in a plan exists in isolation; a change only has meaning in relation to
everything else getting done. If all the little unit tests pass but the system does not
fit together, we have nothing.

- **Prioritize implementing the entire plan over testing it along the way.** Err toward
  getting MORE code implemented properly. The Base Rules in `CLAUDE.md` already force
  thinking, strong typing, and validation as you write, so the real risk is an
  *unfinished, disconnected* system — not an untested function.
- **Execute the full plan first.** Make every logical connection; leave no gaps and no
  unimplemented load-bearing pieces (no `todo!()` on a live path, no port without an
  adapter, no unmounted handler). **Then** fix all the bugs. **Then, and only then,**
  write integration tests — shaped around code that is *proven* to compile and work. You
  do not know that shape until the end.
- **Favor full integration tests of whole sections** over unit tests that validate
  nothing structurally important.
- **Test at the phase boundary, not during the phase.** Do NOT run the test suite while a
  phase is in flight. Testing code that is not yet certified to deliver value is a large,
  recurring waste: the shape is still moving, the failures are mostly "not wired up yet,"
  and every run costs a full compile. The suite runs **once the phase is
  implementation-complete, immediately before reflection** — that is the checkpoint the
  results are actually for.
  - Implementation-complete = every load-bearing piece present and connected: no `todo!()`
    on a live path, no port without an adapter, no unmounted handler.
  - Further runs within a phase are for *driving a known failure to green* after that
    first run — not for spot-checking new code as it is written.
  - Wanting a mid-phase run means the phase is too big. Split it, and let the boundary
    fall where the test run wanted to be.
  - Record the run count in the phase `progress.json` under `.kbd-orchestrator/`.
- This changes *when* we run the suite — never *whether*. Tests are still written for new
  behavior, and `cargo check` still runs freely throughout: it is a compile check, not a
  test, and it is what makes deferring the suite safe.

### Compile Economy
Compiling Rust costs time, memory, and disk. Compile only when it earns its cost, in the
cheapest form that answers the question.

- **Prefer `cargo check` over `cargo build`.** `check` runs the full front-end (borrow +
  type + trait resolution) but skips codegen and linking. Use it for the ordinary "does
  this hold together?" question.
- **Do not compile after every component.** Batch a coherent slice, then run one
  `cargo check` (scoped with `-p <crate>` when iterating on one crate; `--workspace` only
  to validate cross-crate wiring). A full `cargo build`/`cargo test` is a phase-boundary
  action, not a reflex.
- **`--release` / production builds happen at the end, for production use only.** Never
  run a release build just to see if something works.
- Repository dev-build settings that keep the loop cheap (dev-profile
  `debug = "line-tables-only"`, `opt-level 0` for our crates + `opt-level 1` for deps,
  fast platform linker in `.cargo/config.toml`, `rust-analyzer` on a separate target dir,
  `sccache` opt-in only) are documented in `docs/RUST-DEVELOPMENT-MANAGEMENT.md`.
  `--release` stays fully optimized.

---

## Auditability

Per Base Rule #18, record decisions, file changes, tool calls, and external effects.
KBD state lives under `.kbd-orchestrator/` — keep `progress.json` and the waypoint
current so work done by any tool is visible to the others.
