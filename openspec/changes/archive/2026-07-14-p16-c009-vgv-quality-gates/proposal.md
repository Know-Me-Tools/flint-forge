# p16-c009 — VGV Enterprise Quality Gates

**Phase:** 16 — Production Remediation
**Priority:** P3
**Depends on:** p16-c001 through p16-c008 (audits the final code state; running earlier would need re-running after each prior change)

## What this change delivers

- `cargo llvm-cov` coverage gate in CI (≥90% on changed crates).
- `deny.toml` + `cargo-deny` (license + advisory policy) alongside the
  existing `cargo audit`.
- `#![deny(missing_docs)]` on library crates, with `# Errors` sections on
  fallible public functions.
- Deduplicated dependency tree (160 duplicate versions today).
- `// SAFETY:` justification on every `unsafe` block (19 non-pgrx sites as of
  the 2026-07-12 audit; re-count after p16-c001–c008, since c002/c003 may add
  more `unsafe` in signature/capability plumbing).
- A reviewed disposition for each of the 47 error-swallow (`.ok()` / `let _ =`)
  sites on non-test paths.

## Problem

Per the VGV Rust Code Assessment (`RUST_ASSESSMENT.md`), CI runs `cargo fmt
--check`, `cargo clippy -D warnings`, `cargo test`, and `cargo audit` — a solid
baseline, but missing the coverage and license/advisory-policy gates VGV
mandates. `missing_docs` is enforced in only one crate. `cargo tree
--duplicates` shows 160 duplicate dependency versions. 19 `unsafe` blocks in
non-pgrx crates lack documented invariants. 47 `.ok()`/`let _ = ` sites on
non-test paths were flagged as needing individual review (most are likely
legitimate fire-and-forget, but the category is where visibility bugs hide —
see the fake-hash and no-op-capability-check findings, both of which were
"successfully" silent).

## Design

Each sub-task below is independent and can be split across parallel agent
sessions:

### 1. Coverage gate
Add `cargo-llvm-cov` to CI; set threshold ≥90% on changed crates (not
workspace-wide, to avoid punishing legacy low-coverage areas in one PR — but
track workspace-wide coverage as a visible metric).

### 2. `deny.toml`
Add `cargo-deny check licenses` and `cargo-deny check advisories` as CI steps,
configured per the project's actual license posture (check `LICENSE` — MIT
per `README.md`) and any denied/banned crates.

### 3. `missing_docs`
Add `#![deny(missing_docs)]` incrementally, crate by crate (start with
smaller, more stable crates like `forge-domain`), adding `///` docs and
`# Errors` sections as needed. Do not do this in one giant PR — it will
generate enormous, low-review-value diffs; batch by crate.

### 4. Dependency dedup
Run `cargo tree --workspace --duplicates`; for each duplicate, align to a
single version via `[workspace.dependencies]` where compatible; document any
duplicate that cannot be resolved (e.g. a transitive conflict) rather than
leaving it silently unaddressed.

### 5. `unsafe` audit
Re-run the `unsafe` grep after p16-c001–c008 land. For each block, add a
`// SAFETY: <invariant>` comment explaining why it's sound. Where a safe
alternative exists, replace the `unsafe` block entirely instead of just
documenting it.

### 6. Error-swallow review
For each of the 47 `.ok()`/`let _ = ` sites, classify as: (a) genuinely
fire-and-forget (add a comment saying so), (b) should propagate the error
(fix), or (c) should log at minimum (add `tracing::warn!`/`debug!`).

## Verification (gate)

- CI enforces coverage threshold and fails below it.
- CI enforces `cargo-deny` license/advisory checks.
- `missing_docs` clean (denied, not just warned) on all library crates.
- `cargo tree --duplicates` count materially reduced from 160; remaining
  duplicates documented with a reason.
- Every `unsafe` block has a `// SAFETY:` comment.
- Every flagged error-swallow site has an explicit classification (comment or
  fix) — none left silently unreviewed.
