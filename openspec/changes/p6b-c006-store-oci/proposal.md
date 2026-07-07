# p6b-c006 — OCI Registry Artifact Store

**Phase:** 6b — Kiln Hardening
**Priority:** P1
**Depends on:** none

## What this change delivers

Replaces all three `todo!()` methods in `fke-store-oci/src/lib.rs` with a real
OCI registry adapter. WASM artifacts are stored as OCI layers keyed by their
content digest.

## Design

### OCI layer layout

```
oci://<registry>/<repo>:<tag>
  └── layer: application/vnd.wasm.content.layer.v1+wasm
       └── digest: sha256:<hex>
```

`ContentId("sha256:<hex>")` maps directly to the OCI layer digest.

### `StoreOci` state

```rust
pub struct StoreOci {
    client:     oci_client::Client,
    registry:   String,   // from KILN_OCI_REGISTRY env
    repository: String,   // from KILN_OCI_REPO env
}
```

### API

- `put(bytes)` → push layer, push manifest → return `ContentId("sha256:<digest>")`
- `get(id)` → pull layer by digest
- `exists(id)` → HEAD manifest at digest; 404 → false

### New deps

```toml
# fke-store-oci/Cargo.toml
oci-client = "0.14"
sha2 = "0.10"
tokio = { workspace = true }
anyhow = { workspace = true }
```
