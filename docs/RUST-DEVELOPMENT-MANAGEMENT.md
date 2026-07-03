# Rust Development Management

**Status:** Normative. Applies to all crates in the Flint Forge workspace.
**Companion docs:** [`CLAUDE.md`](../CLAUDE.md), [`AGENTS.md`](../AGENTS.md), [`FLINT-FORGE-SPEC.md`](./FLINT-FORGE-SPEC.md).

This document governs *how* we sequence implementation and *how* we compile during
development. It has two parts:

1. **Integration-First Delivery** — the workflow philosophy: build the whole plan,
   wire every connection, fix the bugs, *then* test the shape that is proven to work.
2. **Compile Economy** — the concrete Rust/Cargo settings we use to keep the
   edit-check loop cheap, and the rule that we compile only when it earns its cost.

Both parts sit *on top of* the Prometheus Base Rules and Quality Gates in `CLAUDE.md`.
Nothing here weakens those rules — Integration-First works *because* the base rules
already force us to think, type, and validate before we write.

---

## Part 1 — Integration-First Delivery

### The principle

> Nothing in our plans exists in isolation. A change only has meaning in relation to
> everything else getting done. If all the little unit tests pass but the system does
> not fit together, we have nothing.

We therefore prioritize **implementing the entire plan** over testing each piece along
the way. We err on the side of getting **more code implemented properly** — because the
`CLAUDE.md` rules (think first, strong typing, hexagonal layering, security at every
boundary, no hidden state) already bias us toward correctness *as we write*. The risk
worth managing is not "an untested function"; it is **an unfinished system with gaps
between the parts**.

We value **full integration tests of entire sections of the system** over unit tests
that validate nothing structurally important. We do not know the true shape of the code
until the logical connections are all made and it compiles and runs. So we get to that
point quickly, and only then build tests around the proven shape.

### The rule

For any epoch or goal:

1. **Execute the full plan first.** Implement every change in the plan. Make all the
   logical connections. Leave no gaps and no unimplemented important pieces — no
   `todo!()` on a load-bearing path, no port with no adapter, no handler that never gets
   mounted. The system must be *present* and *fit together*.
2. **Fix all the bugs** surfaced while getting the whole thing to compile and run.
3. **Then, and only then, write the integration tests** — shaped around the code that is
   *proven* to compile and work, not around a guess made before the parts existed.

### The 3-wait budget

> During a single epoch or goal, wait for tests a **maximum of 3 times**.

A "wait for tests" is any point where we stop forward implementation to run a test suite
and block on its result. Reserve those three waits for genuine integration checkpoints —
not for validating an individual function the moment it is written. Spend them on:

- confirming a whole subsystem wires together end-to-end (e.g. "a real JWT produces a
  real RLS row filter"), or
- a final green run before declaring the goal complete.

Track wait-count in the phase state (`.kbd-orchestrator/.../progress.json`) so it is
auditable per Base Rule #18.

### What this does *not* mean

- **It is not "skip testing."** Base Rule #14 stands: implementation is not complete
  until verified. Integration-First changes *when* and *at what granularity* we test —
  the whole section, at the end — not *whether* we test.
- **It is not "skip thinking or typing."** Base Rules #1, #10, #13 stand. The reason we
  can defer tests safely is that these gates catch whole classes of error at author time.
- **It does not license dead ends.** "More code implemented properly" means load-bearing,
  connected code — not speculative abstractions (Base Rule #2 / YAGNI).

### Definition of Done for a goal

- [ ] Every planned change is implemented; no `todo!()` on a load-bearing path.
- [ ] All logical connections are made (every port has its adapter; every route is
      mounted; every use-case is reachable from an interface crate).
- [ ] The workspace compiles clean (`cargo check --workspace`, clippy pedantic `-D warnings`).
- [ ] Bugs found during wiring are fixed.
- [ ] Integration tests exist for the completed sections, written against the proven shape.
- [ ] ≤ 3 test-waits were spent (recorded).

---

## Part 2 — Compile Economy

Compiling Rust is expensive in time, memory, and disk. We compile **only when it earns
its cost**, and when we do, we use the cheapest form that answers the question.

### The rule

- **Prefer `cargo check` over `cargo build`.** `check` runs the full front-end (borrow
  check, type check, trait resolution) but skips codegen and linking — often ~10× faster.
  For "does my code hold together?" — which is the question almost all the time during
  Integration-First implementation — `check` is the correct tool.
- **Do not compile after every component.** Batch. Implement a coherent slice, then run
  one `cargo check` (workspace or `-p <crate>`) to validate it. A full `cargo build` /
  `cargo test` is a checkpoint action, spent against the 3-wait budget — not a reflex.
- **`--release` / production builds happen at the end**, for production use only. Never
  run a release build to "just see if it works."
- **Scope your checks.** Use `cargo check -p <crate>` while iterating on one crate; only
  run `--workspace` when validating cross-crate wiring.

### Repository build configuration

The following settings are (or should be) applied at the workspace root and in
`.cargo/config.toml`. They are chosen to minimize the dev edit-check loop while keeping
`--release` untouched for production.

> Platform note: primary dev is macOS (Darwin, Apple Silicon). `split-debuginfo =
> "unpacked"` is already the default on macOS, so the largest lever here is the linker and
> the debug/codegen settings. On Linux (CI), Rust 1.90+ ships `rust-lld` as the default
> linker on `x86_64-unknown-linux-gnu`, so no linker flag is needed there.

#### `Cargo.toml` — dev profile

```toml
[profile.dev]
# 0 keeps our own crates fast to compile and fully debuggable.
opt-level = 0
# Line-tables only: enough for backtraces, far less info for the compiler/linker
# to carry than full `debug = true` (which is 2).
debug = "line-tables-only"
# Incremental is on by default for dev; state it so intent is explicit.
incremental = true

[profile.dev.package."*"]
# Optimize *dependencies* once (they rarely change) so runtime of the dev binary is
# tolerable, without slowing recompiles of our own code. opt-level 1 avoids the
# monomorphization-sharing penalty that kicks in at 2/3.
opt-level = 1

# Release stays fully optimized — untouched for production builds.
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 16
```

#### `.cargo/config.toml` — faster linking

Linking dominates incremental rebuild time. Use a fast linker per platform. These are
opt-in per developer machine (the tool must be installed); do not hard-fail the build if
it is absent.

```toml
# Apple Silicon (primary dev). Uses LLVM lld's Mach-O driver from Homebrew.
# Install: brew install llvm  (Homebrew symlinks ld64.lld into /opt/homebrew/bin)
[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/opt/homebrew/bin/ld64.lld"]

# Intel macOS
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/ld64.lld"]

# Linux / CI. mold is fastest; rust-lld is the 1.90+ default and needs no flag.
# Install mold: apt-get install mold  (or build from source)
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

> This repo ships the linker config as **`.cargo/config.toml.example`** (committed); the
> live `.cargo/config.toml` is git-ignored. Copy the example to `.cargo/config.toml` only
> if the linker is installed on your machine, and comment out any block whose tool you
> don't have — otherwise every build fails. This keeps a fresh clone building for everyone.

#### Environment / tooling

- **`rust-analyzer` uses a separate target dir.** Point the editor's check at
  `target/rust-analyzer` so its background `cargo check` never contends with your
  foreground `cargo check`/`build` in `target/`. In VS Code:
  `"rust-analyzer.cargo.targetDir": true` (or a named subdir). This alone can turn a
  30 s stall into ~4 s.
- **`sccache`** helps only when many independent projects share the same dependency
  versions (e.g. CI runners, or a dev working across several workspaces). It adds
  overhead for a single incremental workspace, so it is **opt-in, not default** here.
- **`CARGO_INCREMENTAL=1`** is the default for dev; do not disable it locally. (CI may
  disable it because clean builds do not benefit from incremental state.)

### Optional, higher-leverage levers (use with judgment)

- **Cranelift codegen backend** (`-Zcodegen-backend=cranelift`, nightly): generates
  machine code much faster than LLVM at the cost of slower *runtime*, ideal for the dev
  loop. It does not support all features (notably inline asm), and we pin stable
  (`1.90`), so this is a *per-developer, nightly-only* experiment — not a workspace
  default. Never use it for `--release`.
- **Trim dependency features.** Fewer compiled features = less to compile. Audit heavy
  crates' feature flags before adding them (`default-features = false` + explicit
  features — we already do this for `reqwest`).
- **Fewer, smaller crates on the hot path** compile in parallel better; the 500-line
  file / directory-module rule already pushes us this way.

### Compile-economy checklist

- [ ] Reaching for a compile? Use `cargo check`, not `cargo build`, unless you need a
      runnable/testable artifact.
- [ ] Iterating on one crate? Scope with `-p <crate>`.
- [ ] Batched a coherent slice before checking, rather than checking per component?
- [ ] Kept `--release` for production only?
- [ ] Fast linker configured (or knowingly skipped) on this machine?
- [ ] `rust-analyzer` on a separate target dir?

---

## How the two parts reinforce each other

Integration-First means long stretches of pure implementation between checkpoints.
Compile Economy makes those stretches cheap: `cargo check -p <crate>` after each batch
gives fast structural feedback without codegen, and we save the expensive
`build`/`test`/`--release` runs for the ≤ 3 real integration checkpoints. Get the whole
system present and connected quickly and cheaply; prove it compiles and runs; then test
the shape that is real.

---

## Sources

Build-configuration and compile-time guidance in Part 2 is drawn from:

- [The Rust Performance Book — Build Configuration](https://nnethercote.github.io/perf-book/build-configuration.html)
- [The Cargo Book — Profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [The Cargo Book — Optimizing Build Performance](https://doc.rust-lang.org/cargo/guide/build-performance.html)
- [Rust Blog — Faster linking times with 1.90.0 stable on Linux using LLD](https://blog.rust-lang.org/2025/09/01/rust-lld-on-1.90.0-stable/)
- [corrode.dev — Tips For Faster Rust Compile Times](https://corrode.dev/blog/tips-for-faster-rust-compile-times/)
- [David Lattimore — Speeding up the Rust edit-build-run cycle](https://davidlattimore.github.io/posts/2024/02/04/speeding-up-the-rust-edit-build-run-cycle.html)
- [rust-analyzer separate target dir discussion](https://github.com/rust-lang/rust-analyzer/issues/4616)
- [Earthly — Optimizing Rust Build Speed with sccache](https://earthly.dev/blog/rust-sccache/)
