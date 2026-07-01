# KBD Constraints — Flint Forge

Derived from CLAUDE.md quality gates and RFC-FORGE-001 §2.5.

---

## BLOCKING (hard stop — do not proceed)

- **BLOCK** importing an adapter crate (`fdb-postgres`, `fdb-realtime`, `fke-store-*`, `fke-sign-*`) from a domain or app crate (`fdb-domain`, `fdb-app`, `fke-domain`, `forge-domain`, `forge-policy`). Composition is only allowed in interface crates (`fdb-gateway`, `fke-server`).
- **BLOCK** calling `unwrap()` or `expect()` in any library crate. Use `thiserror` in libs; `anyhow` is permitted only at binary entry points (`fdb-gateway`, `fke-server`, `forge-cli`).
- **BLOCK** logging JWT payloads, JWT claims, relation tuples, or tenant identifiers at any log level.
- **BLOCK** any file exceeding 500 lines — split into directory modules.
- **BLOCK** adding a new dependency without checking for an existing workspace dependency in `[workspace.dependencies]` first.
- **BLOCK** modifying pgrx extension crates (`ext-flint-*`) via `cargo build`/`cargo check` — they require `cargo pgrx` and are workspace-excluded by design.
- **BLOCK** introducing a `clippy::allow` attribute that suppresses a `pedantic` lint without an explicit justification comment explaining why it is a scaffold-stage concession.

---

## WARNING (flag and confirm before proceeding)

- **WARN** introducing a new third-party crate not yet in `[workspace.dependencies]` — check compatibility and justify.
- **WARN** adding a public enum variant without `#[non_exhaustive]` on the enum.
- **WARN** creating a domain ID type without `#[repr(transparent)]` newtype wrapping.
- **WARN** adding `tracing` instrumentation that crosses a port boundary without a `#[tracing::instrument]` span.
- **WARN** changing `forge-domain` types — this crate is a semver-disciplined contract consumed by all subsystems.
- **WARN** mixing `anyhow` into a library crate (non-binary) — confirm this is intentional.
- **WARN** any change to the JWT GUC injection sequence (the three `SET LOCAL` statements in `fdb-auth`) — this is the load-bearing RLS mechanism.
- **WARN** disabling the per-event RLS re-query in `fdb-realtime` subscriptions — predicate-pushdown optimization exists but is off by default for data-leak safety reasons.

---

## STACK NOTES

- Toolchain: `1.90` (pinned via `rust-toolchain.toml`), MSRV `1.85`
- Clippy gate: `pedantic` at warn, `-D warnings` in CI
- pgrx versions are intentionally split: `0.12` for `ext-flint-auth` (pg17), `0.18.1` for `flint_vault` (pg18)
- OpenSpec change sets in `openspec/changes/` are the authoritative build sequence; start with `p0-c001-workspace`
