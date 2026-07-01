# Flint Forge — Dagger CI

CI runs on Dagger (not GitHub Actions). The `check` pipeline wraps `scripts/ci-check.sh`
in a pinned `rust:1.90` container so local and CI runs are identical.

## Run
```
# one-time: install the dagger CLI, then generate bindings
curl -fsSL https://dl.dagger.io/dagger/install.sh | sh
dagger develop                 # generates .dagger/internal + go.mod (committed after first run)

# the gate
dagger call check --source=.
```

## Locally, without Dagger
```
./scripts/ci-check.sh          # same three steps: fmt --check, clippy -D warnings, cargo check
```

> The `.dagger/internal` bindings and `go.mod` are produced by `dagger develop` and are not
> scaffolded here (the dagger CLI was not present at scaffold time). The wrapped checks are
> verified green locally — see the p0-c001 gate.
