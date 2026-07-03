---
type: Reference
id: integration-first-compile-economy
title: Integration-First Delivery and Compile Economy
description: Binding development-management philosophy for Flint Forge — implement the full plan first, wire everything, fix bugs, then integration-test; and compile Rust only when necessary.
tags:
- development-workflow
- rust
- compile-times
- integration-testing
- philosophy
- karpathy-wiki
sources:
- manual:user-tjames
- doc:docs/RUST-DEVELOPMENT-MANAGEMENT.md
timestamp: 2026-07-03T00:00:00.000000+00:00
created_at: 2026-07-03T00:00:00.000000+00:00
updated_at: 2026-07-03T00:00:00.000000+00:00
revision: 0
---

Binding development-management philosophy set by the user (tjames) for Flint Forge and
general Rust work. Canonical detail lives in the repo doc
[RUST-DEVELOPMENT-MANAGEMENT.md](../../../docs/RUST-DEVELOPMENT-MANAGEMENT.md); summaries
are in `CLAUDE.md` and `AGENTS.md`.

## Integration-First Delivery

Nothing in a plan exists in isolation — a change only has meaning in relation to
everything else getting done. If all the little unit tests pass but the system does not
fit together, we have nothing.

- Prioritize implementing the **entire plan** over testing it along the way. Err toward
  getting **more code implemented properly** — the base rules (think-first, strong
  typing, hexagonal layering, security at every boundary) already enforce correctness at
  author time, so the risk worth managing is an unfinished, disconnected system, not an
  untested function.
- Execute the full plan first: every logical connection made, no gaps, no unimplemented
  load-bearing pieces (no `todo!()` on a live path, no port without an adapter, no
  unmounted handler). Then fix all the bugs. Then — and only then — write the integration
  tests, shaped around code proven to compile and work. We do not know that shape until
  the end.
- Favor full integration tests of whole sections over unit tests that validate nothing
  structurally important.
- 3-wait budget: wait for tests a maximum of three times per epoch or goal. Spend those
  waits on genuine integration checkpoints, not per-function validation. Record the count
  in phase `progress.json`.

This changes when and at what granularity we test — never whether.

## Compile Economy

Compiling Rust costs time, memory, and disk. Compile only when it earns its cost.

- Prefer `cargo check` over `cargo build` (roughly 10x faster; full front-end, skips
  codegen and linking) — it answers "does this hold together?", which is the question
  almost all the time.
- Do not compile after every component. Batch a coherent slice, then check once; scope
  with `-p <crate>`. A full `build` or `test` is a checkpoint action counted against the
  3-wait budget.
- `--release` and production builds only at the very end, for production use.
- Dev-loop settings: `debug = "line-tables-only"`, `opt-level 0` for our crates plus
  `opt-level 1` for dependencies, a fast platform linker (`lld` on macOS, `mold` or
  bundled `rust-lld` on Linux), `rust-analyzer` on a separate target directory, and
  `sccache` opt-in only. `--release` stays fully optimized.

## Why It Holds Together

Integration-First means long stretches of pure implementation between checkpoints;
Compile Economy makes those stretches cheap via `cargo check` for structural feedback.
Get the whole system present and connected quickly and cheaply, prove it compiles and
runs, then test the shape that is real.

# Citations

1. manual:user-tjames
2. doc:docs/RUST-DEVELOPMENT-MANAGEMENT.md
