# Flint Forge — API Versioning Policy

This document defines when and how API version numbers are incremented for
the two primary Flint Forge API surfaces:

- **A2UI HTTP API** — version tracked by `FLINT_A2UI_API_VERSION` (`.env.example`)
  and embedded in `docs/api/a2ui.md` as `**Current version:** \`N\``
- **Kiln WIT ABI** — version tracked by `FLINT_KILN_ABI_VERSION` (`.env.example`)
  and embedded in `docs/api/kiln-abi.md` as `**Current ABI version:** \`N\``

Both versions are integers starting at `1`. They are independent of the
semver workspace version (`Cargo.toml`).

---

## 1. What constitutes a breaking change

### A2UI HTTP API (increment `FLINT_A2UI_API_VERSION`)

A **breaking change** is any modification that would cause a correctly-written
client targeting the current version to fail or behave incorrectly:

| Change type | Breaking? |
|---|---|
| Remove a route (`GET /a2ui/v1/components` → gone) | ✅ **YES** |
| Rename or remove a required response field | ✅ **YES** |
| Change the type of an existing field | ✅ **YES** |
| Change auth requirements (e.g., make an authenticated route public) | ✅ **YES** |
| Change the HTTP method of an existing route | ✅ **YES** |
| Remove an enum variant that clients may receive | ✅ **YES** |
| Add a new required request field | ✅ **YES** |
| Add a new optional response field | ❌ no |
| Add a new route | ❌ no |
| Add a new optional query parameter | ❌ no |
| Fix a bug that changes incorrect behaviour to correct behaviour | ❌ no (document in CHANGELOG) |

### Kiln WIT ABI (increment `FLINT_KILN_ABI_VERSION`)

| Change type | Breaking? |
|---|---|
| Remove a WIT interface from `world edge-function` | ✅ **YES** |
| Remove or rename a function from an interface | ✅ **YES** |
| Change a function signature (param type, return type, or count) | ✅ **YES** |
| Add a new required import to the world | ✅ **YES** |
| Remove a resource type | ✅ **YES** |
| Change error codes emitted by an interface | ✅ **YES** |
| Add a new function to an interface | ❌ no (additive) |
| Add a new interface (only imports; no export change) | ❌ no (additive) |
| Change fuel/epoch defaults (non-ABI, configurable) | ❌ no (document in runbook) |

---

## 2. How to make a breaking change

1. **Increment the version integer** in both of:
   - The `**Current version:** \`N\`` line in the relevant `docs/api/*.md`
   - The `FLINT_A2UI_API_VERSION=N` or `FLINT_KILN_ABI_VERSION=N` line in `.env.example`

2. **Update `MIGRATION.md`** at the workspace root — add a section for the new version
   describing the breaking delta and the required client changes.

3. **Update `docs/api/<surface>.md`** — reflect the changed contract.

4. **Add `@since` annotation** to any new WIT interface or function (Kiln ABI only):
   `@since(version = 0.2.0)` where the minor version matches the new ABI version.

5. **Bump the WIT package version** in `wit/flint/host/world.wit` if the Kiln ABI
   changes: `package flint:host@0.2.0`.

### Version consistency enforcement

A CI step (`scripts/check_api_versions.sh`) verifies that the version embedded
in each `docs/api/*.md` matches the corresponding variable in `.env.example`.
The build fails if they are out of sync. This prevents a documentation update
from shipping without the version bump, or vice versa.

---

## 3. Non-breaking additions — no version bump needed

When adding new functionality without removing or altering existing contracts:
- Add the new route/field/interface to `docs/api/*.md`
- Add a `@since` annotation to new WIT functions
- Update `CHANGELOG.md` but do **not** increment the API version
- Document the addition in the relevant section of `docs/api/*.md`

---

## 4. Deprecation process

Before removing an existing API surface:

1. Mark it deprecated in `docs/api/*.md` with a `> **Deprecated** (v_N_)` callout
2. Keep it functional for at least one minor release cycle
3. Remove it in the next breaking-change release (which increments the version)
4. Document the removal in `MIGRATION.md`

---

## 5. Current versions

| Surface | Variable | Value | Doc |
|---|---|---|---|
| A2UI HTTP API | `FLINT_A2UI_API_VERSION` | `1` | `docs/api/a2ui.md` |
| Kiln WIT ABI | `FLINT_KILN_ABI_VERSION` | `1` | `docs/api/kiln-abi.md` |

---

## 6. CI enforcement

`scripts/check_api_versions.sh` is run on every push in `ci.yml`. It:
1. Extracts `Current version: N` from `docs/api/a2ui.md`
2. Extracts `Current ABI version: N` from `docs/api/kiln-abi.md`
3. Reads `FLINT_A2UI_API_VERSION=N` and `FLINT_KILN_ABI_VERSION=N` from `.env.example`
4. Fails with a clear error if any pair is out of sync

To update: change the integer in **both** the doc and `.env.example` in the same
commit. The CI check will pass only when they agree.
