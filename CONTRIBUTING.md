# Contributing to Flint Forge

Thanks for your interest in Flint Forge. This document covers the mechanics
of contributing code; for the architecture, hexagonal layering rules, and
design contracts, start with [CLAUDE.md](CLAUDE.md) and
[docs/FLINT-FORGE-SPEC.md](docs/FLINT-FORGE-SPEC.md).

## Before You Start

- **Security vulnerabilities** go through [SECURITY.md](SECURITY.md), not a
  pull request or public issue.
- **Non-security questions** go through [SUPPORT.md](SUPPORT.md).
- For anything beyond a small fix, open an issue first to discuss the
  approach — this project has strict architectural rules (see below) and it's
  easier to align before you've written the code than after.

## Development Setup

```bash
# Check all workspace crates (non-pgrx)
cargo check --workspace

# Run tests
cargo test --workspace

# Lint — this is the CI gate, treat warnings as errors
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

`ext-flint-*` (Flint Anvil) crates are excluded from the default workspace
because they require a Postgres/pgrx toolchain — see CLAUDE.md for the
`cargo pgrx` setup.

Run the full CI check locally before opening a PR:

```bash
./scripts/ci-check.sh
```

## Code Standards (non-negotiable, CI-enforced)

- No `unwrap()`/`expect()` in library crates — `thiserror` in libs, `anyhow`
  only at binary entry points (`fdb-gateway`, `fke-server`, `forge-cli`).
- `clippy::pedantic` + `-D warnings` across the workspace.
- The **hexagonal dependency rule**: `forge-domain` → `forge-ports`/`*-app` →
  adapters → interface crates. Domain and app crates never import adapter
  crates. See CLAUDE.md for the full crate map.
- No file over 500 lines — split into directory modules.
- Never log JWT payloads, claims, relation tuples, or tenant identifiers.
- `#[non_exhaustive]` on public enums; newtype IDs as `#[repr(transparent)]`.

## Submitting a Pull Request

1. Fork the repository and create a branch from `main`.
2. Make your change, matching existing conventions in the touched crate.
3. Add or update tests for behavioral changes.
4. Run `cargo fmt`, `cargo clippy --workspace -- -D warnings`, and
   `cargo test --workspace` locally.
5. Write a clear commit message (`type(scope): summary` — see recent commit
   history for the convention this project uses).
6. Open a PR describing what changed and why. Link any related issue.

## Commit Messages

Follow the existing convention visible in `git log`: a `type(scope):
summary` subject line (`fix`, `feat`, `docs`, `chore`, `perf`, `ci`, ...),
optionally followed by a body explaining rationale for non-obvious changes.

## Code of Conduct

There is no separate `CODE_OF_CONDUCT.md` yet. Be respectful and
constructive in issues, discussions, and PR reviews; maintainers may close or
lock threads that don't meet that bar.
