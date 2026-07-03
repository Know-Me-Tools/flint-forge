# p35-c001 — Unblock workspace clippy gate (hello-component)

## Change ID
`p35-c001-clippy-unblock-hello-component`

## Phase
`p3.5-ci-postgres-hardening`

## Goal Mapping
**G4** — workspace clippy clean end-to-end.

## Problem
`scripts/ci-check.sh` runs `cargo clippy --workspace --all-targets -- -D warnings`,
which **currently fails** on `examples/hello-component`: `used_underscore_items` fires
inside the macro-generated WASI bindings (`bindings::export` → `__export_...`). The CI
gate is red regardless of any other work, so nothing can be CI-validated until this is
fixed.

## Scope
- Add a narrow `#[allow(clippy::used_underscore_items)]` on the generated-binding module
  in `examples/hello-component` (the lint targets code we do not author), OR exclude the
  example crate from the workspace lint gate if an allow cannot be scoped to the macro
  expansion.
- No behavior change; example crate only.

## Out of Scope
- Any `crates/*` lint changes (they are already clean).

## Acceptance Criteria
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` exits 0.
- [ ] `scripts/ci-check.sh` passes locally.
- [ ] The allow is scoped as narrowly as possible (module/expansion, not crate-wide if avoidable).
