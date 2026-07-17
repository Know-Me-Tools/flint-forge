# p16-c007 — 500-Line File-Size Compliance

**Phase:** 16 — Production Remediation
**Priority:** P1 (BLOCKING per `constraints.md`, not security-critical)
**Depends on:** p16-c001, p16-c002, p16-c003, p16-c004 (touches the same files those changes modify; sequenced after to avoid rebasing conflicts)

## What this change delivers

- All 17 files currently over the project's own 500-line BLOCKING limit are
  split into directory modules, with no behavior change.

## Problem

`constraints.md` and `CLAUDE.md` both state: "No file over 500 lines — split
into directory modules." As of the 2026-07-12 audit, 17 files violate this:

| File | Lines |
|---|---|
| `crates/fdb-gateway/src/routes/htmx/renderers.rs` | 1267 |
| `crates/fdb-gateway/src/main.rs` | 990 |
| `crates/fdb-gateway/src/routes/a2ui.rs` | 802 |
| `crates/fdb-realtime/src/listen.rs` | 769 |
| `crates/fdb-gateway/src/routes/mcp.rs` | 732 |
| `crates/fke-runtime/src/lib.rs` | 674 |
| `crates/fdb-reflection/src/compilers/mcp.rs` | 668 |
| `crates/fdb-gateway/src/routes/htmx/mod.rs` | 636 |
| `crates/fdb-query/src/operator.rs` | 632 |
| `crates/fdb-reflection/src/compilers/rest/mod.rs` | 605 |
| `crates/fdb-gateway/src/routes/a2a.rs` | 552 |
| `crates/fdb-app/src/a2ui/design_md_parser.rs` | 536 |
| `crates/fdb-query/src/plan.rs` | 532 |
| `crates/forge-cli/src/main.rs` | 528 |
| `crates/ext-flint-vault/src/lib.rs` | 513 |
| `crates/fdb-reflection/src/compilers/a2ui.rs` | 510 |
| `crates/fdb-gateway/src/routes/agui.rs` | 508 |

(Line counts will shift after p16-c001–c004 land; re-measure before starting
this change.)

## Design

Pure mechanical refactor — no behavior change, no public-API change unless a
module boundary genuinely improves encapsulation (prefer re-exporting to keep
call sites stable). For each file:

1. Identify natural seams (per-route-group in `main.rs`/`routes/*`, per-
   operator-category in `fdb-query`, per-renderer-kind in `htmx/renderers.rs`).
2. Extract into a `mod.rs` + submodule directory, re-exporting the prior public
   surface from the parent `mod.rs` so downstream imports don't break.
3. Re-measure line counts after each split; confirm no split result itself
   exceeds 500 lines.
4. Run `cargo check -p <crate>` after each file to catch import breakage
   early (compile-economy: check, not full build).

Order: start with `ext-flint-vault` (workspace-excluded, pgrx — do not run
`cargo check`/`cargo build` on it per `constraints.md`; verify via `cargo
pgrx check` or equivalent instead) and the largest non-pgrx files first
(`renderers.rs`, `main.rs`) since they carry the most risk of hidden coupling.

## Verification (gate)

- `find crates -name '*.rs' -not -path '*/target/*' | xargs wc -l | awk '$1>500'`
  returns nothing.
- `cargo check --workspace` and `cargo clippy --workspace --all-targets -- -D
  warnings` stay green throughout (check after each file split, not just at
  the end — compile economy: `cargo check`, not `cargo build`).
- `cargo test --workspace` passes (no behavior change).
