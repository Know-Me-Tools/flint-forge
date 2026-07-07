# p6b-c008 — S3/R2 Artifact Store

**Phase:** 6b — Kiln Hardening
**Priority:** P2
**Depends on:** none

## What this change delivers

Replaces `StoreS3`'s three `todo!()` methods with an S3/R2 adapter using the
`object_store` crate (supports AWS S3, Cloudflare R2, and GCS from one API).

## Design

- `put(bytes)` → compute sha256 → `object_store.put(&path, bytes)` → `ContentId("sha256:<hex>")`
- `get(id)` → `object_store.get(&path).await?.bytes().await`
- `exists(id)` → `object_store.head(&path).await.is_ok()`

`KILN_S3_BUCKET`, `KILN_S3_ENDPOINT` (for R2/MinIO), `KILN_S3_ACCESS_KEY`, `KILN_S3_SECRET_KEY`.

### New deps

```toml
object_store = { version = "0.11", features = ["aws"] }
sha2 = "0.10"
tokio = { workspace = true }
anyhow = { workspace = true }
```
