#!/usr/bin/env bash
# Flint Forge — canonical CI check (p0-c001 gate).
# Runs locally and unchanged inside the Dagger container.
set -euo pipefail

echo "==> rustfmt --check"
cargo fmt --all --check

echo "==> clippy (pedantic, -D warnings)"
cargo clippy --workspace --all-targets -- -D warnings

echo "==> cargo check"
cargo check --workspace

echo "OK: fmt + clippy::pedantic + cargo check all green"
