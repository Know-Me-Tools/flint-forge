# Migration Guide — Flint Forge v0.10.0 → v1.0.0

This document covers breaking and notable changes between v0.10.0 and v1.0.0
for consumers of the Flint Forge platform: skill authors, SDK users, and
operators deploying the stack.

---

## Summary

v1.0.0 is an **API stability release**. All changes are additive or
documentation-only. There are **no runtime behaviour changes** between v0.10.0
and v1.0.0.

---

## Rust crate consumers

### `#[non_exhaustive]` added to 9 public enums

The following enums were marked `#[non_exhaustive]` in v1.0.0 to signal that
new variants may be added in future minor releases. If your code uses an
exhaustive `match` on any of these types, the Rust compiler will emit an error
until you add a wildcard arm.

| Enum | Crate | Required change |
|---|---|---|
| `AgUiEvent` | `fdb-domain` | Add `_ => { /* forward-compat */ }` arm |
| `EndpointKind` | `fdb-reflection` | Add `_ => { /* forward-compat */ }` arm |
| `Capability` | `fke-domain` | Add `_ => { /* forward-compat */ }` arm |
| `TargetArch` | `fke-domain` | Add `_ => { /* forward-compat */ }` arm |
| `Decision` | `forge-policy` | Add `_ => { /* forward-compat */ }` arm |
| `PolicyLoadError` | `forge-policy` | Add `_ => { /* forward-compat */ }` arm |

**Example migration:**

```rust
// Before (v0.10.0) — exhaustive match, breaks in v1.0.0
match capability {
    Capability::Db         => { /* ... */ },
    Capability::Llm        => { /* ... */ },
    Capability::Kv         => { /* ... */ },
    Capability::Identity   => { /* ... */ },
    Capability::Secrets    => { /* ... */ },
    Capability::HttpOutgoing => { /* ... */ },
}

// After (v1.0.0) — forward-compatible
match capability {
    Capability::Db         => { /* ... */ },
    Capability::Llm        => { /* ... */ },
    Capability::Kv         => { /* ... */ },
    Capability::Identity   => { /* ... */ },
    Capability::Secrets    => { /* ... */ },
    Capability::HttpOutgoing => { /* ... */ },
    // Forward-compatibility: new capabilities added in future versions
    // will not break existing match arms.
    _ => { /* handle unknown capability */ },
}
```

---

## Kiln skill authors (WASM component model)

### WIT `@since` annotations

All interfaces in `flint:host@0.1.0` are now annotated with
`@since(version = 0.1.0)`. This is **informational only** — it does not change
the compiled component output or affect existing skills. No action required.

```wit
// v1.0.0 — annotation added, no functional change
@since(version = 0.1.0)
interface db {
    query: func(sql: string, params: list<string>) -> result<list<string>, host-error>;
}
```

For the full ABI reference, see [`docs/api/kiln-abi.md`](docs/api/kiln-abi.md).

---

## SDK consumers

### `@flint/react`

Bumped from `0.1.0` to `1.0.0`. No API breaking changes. The 1.0.0 release
adds stable exports for:
- `useFlintRegistry()` hook
- `FlintProvider` context
- `exportDesignSyncTokens()` utility
- `useAgUiStream()` AG-UI hook

If you were importing from `@flint/react@0.1.0`, update your `package.json`:

```json
{
  "dependencies": {
    "@flint/react": "^1.0.0"
  }
}
```

See [`packages/flint-react/CHANGELOG.md`](packages/flint-react/CHANGELOG.md)
for the full feature list.

### `flint_genui` (Flutter/Dart)

Bumped from `0.1.0` to `1.0.0`. No API breaking changes. The 1.0.0 release
adds stable exports for:
- `FlintSseClient` with auto-reconnect
- `FlintCatalog` component loader
- `FlintSurface` widget
- `FlintTokens` design token accessor

Update your `pubspec.yaml`:

```yaml
dependencies:
  flint_genui: ^1.0.0
```

See [`packages/flint_genui/CHANGELOG.md`](packages/flint_genui/CHANGELOG.md)
for the full feature list.

---

## Operators / infrastructure

### Dockerfile entrypoints (v1.0.0, p11-c005)

Both `docker/fdb-gateway/Dockerfile` and `docker/fke-server/Dockerfile` now
use shell entrypoint scripts that read Docker secret files before exec-ing the
binary. This eliminates the requirement for a `.env` file on production hosts.

**What changes:**
- `DATABASE_URL` is now constructed from `/run/secrets/postgres_password` inside
  the container. The `.env` file on the host is no longer required in production.
- `FLINT_JWT_SECRET` is set from `/run/secrets/jwt_secret` in the gateway container.
  Note: as of the JWKS-based rewrite of bearer verification, `fdb-gateway`
  itself does not read `FLINT_JWT_SECRET` — inbound auth requires
  `FLINT_GATE_JWKS_URL`/`FLINT_GATE_ISSUER` instead (see
  [`docs/runbook.md §2.2`](docs/runbook.md)). This entrypoint behavior is
  unchanged but no longer feeds the active auth path.

**What you need to do:**
1. Run `./scripts/rotate_secrets.sh` to generate `secrets/*.txt` files
2. Use `docker compose -f docker-compose.yml -f docker-compose.prod.yml up -d`
3. The `.env` file is no longer required for production hosts (still useful for
   local dev with `docker compose up`)

See [`docs/runbook.md §10.7`](docs/runbook.md) for the full rotation procedure.

### `cargo audit` gate now in CI

The CI pipeline (`ci.yml`) now runs `cargo audit` after tests. Any unfixed
advisory with CVSS ≥ 7.0 that is not in `.cargo/audit.toml` will fail the build.

---

## API documentation

New in v1.0.0:

| Document | Path | Covers |
|---|---|---|
| A2UI HTTP API | [`docs/api/a2ui.md`](docs/api/a2ui.md) | All `/a2ui/v1/*` endpoints, schemas, versioning policy |
| Kiln ABI | [`docs/api/kiln-abi.md`](docs/api/kiln-abi.md) | WIT interfaces, fuel/epoch limits, Cedar flow, skill authoring |

---

## Version environment variables

Two new informational environment variables document the API versions in use:

| Variable | Value | Purpose |
|---|---|---|
| `FLINT_A2UI_API_VERSION` | `1` | A2UI HTTP API contract version |
| `FLINT_KILN_ABI_VERSION` | `1` | Kiln WIT ABI version |

These are not enforced at runtime — they are advisory for SDK clients and
monitoring tooling. Add them to your `.env` if you want them included in
container environment inspection.
