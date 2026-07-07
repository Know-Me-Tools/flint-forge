# p6b-c007 — IPFS Artifact Store (Kubo HTTP API)

**Phase:** 6b — Kiln Hardening
**Priority:** P2
**Depends on:** none

## What this change delivers

Replaces `StoreIpfs`'s three `todo!()` methods with a Kubo HTTP API adapter.
`ContentId` holds the CID string returned by Kubo's `/api/v0/add`.

## Design

- `put(bytes)` → POST `/api/v0/add`, parse JSON `{ "Hash": "Qm..." }` → `ContentId("Qm...")`
- `get(id)` → POST `/api/v0/cat?arg=<cid>` → raw bytes
- `exists(id)` → POST `/api/v0/stat?arg=<cid>` → 200 = true, 500 = false

`FLINT_IPFS_URL` env var (default: `http://localhost:5001`).

### New deps

```toml
reqwest = { workspace = true }
tokio   = { workspace = true }
anyhow  = { workspace = true }
```
