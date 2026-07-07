# p14-c002 — Kiln Guest Rust SDK (`flint-skill`)

**Phase:** 14 — v1.1.0  **Priority:** P1  **Depends on:** none

## Problem

Skill authors targeting `flint:host@0.1.0` must use raw WIT bindings — no
ergonomic Rust SDK exists. The barrier to writing a Flint edge function is
too high for casual skill authors.

## Solution

Create `crates/flint-skill/` — a thin ergonomic wrapper crate.

### Structure

```
crates/flint-skill/
  Cargo.toml         — crate manifest; wasm32-wasip2 target
  src/
    lib.rs           — public API re-exports
    db.rs            — typed db::query wrapper (JSON params/rows → serde_json::Value)
    llm.rs           — typed llm::complete/embed wrapper
    kv.rs            — typed kv::get/set wrapper
    identity.rs      — identity::claims wrapper
    secrets.rs       — secrets::get/reveal wrapper
    error.rs         — SkillError (thiserror); wraps WIT host-error
  tests/
    integration.rs   — mock host calls; verify typed wrappers
  README.md          — quick-start guide
```

### Public API design

```rust
use flint_skill::{Database, Llm, Kv, Identity};

// Database — typed query
let rows: Vec<serde_json::Value> = Database::query(
    "SELECT * FROM users WHERE id = $1",
    &[json!(user_id)],
).await?;

// LLM — completion
let response = Llm::complete("Summarize this text", None).await?;

// Kv — ephemeral key-value
Kv::set("cache_key", b"cached value");
let val: Option<Vec<u8>> = Kv::get("cache_key");

// Identity — claims
let claims: serde_json::Value = Identity::claims();
```

### Cargo.toml

```toml
[package]
name = "flint-skill"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[lib]
crate-type = ["cdylib", "rlib"]
```

### Note on WIT bindings

The SDK does NOT embed the WIT bindings directly. Instead, it provides typed
wrappers that a skill component calls. The skill component's own `Cargo.toml`
includes both `flint-skill` (for the ergonomic API) and the WIT-generated
bindings (for `bindings::export!`). The SDK calls through the bindings.

Alternatively, the SDK can be structured as a proc-macro or build-script that
generates the binding glue — but that's over-engineering for v0.1.0. The thin
wrapper approach is the right first step.

### Gate

- `cargo check -p flint-skill` compiles (host target)
- `cargo test -p flint-skill` passes
- `cargo clippy -p flint-skill -- -D warnings` clean
- `README.md` has a working quick-start example
