# flint-skill

Ergonomic Rust SDK for authoring [Flint Kiln](../../docs/api/kiln-abi.md)
edge-function skills that target the `flint:host@0.1.0` WIT world.

`flint-skill` is the **consumer-side** helper crate. Skill authors include it
alongside the `wit-bindgen`-generated bindings in their own component crate.
The SDK provides:

- A single typed [`SkillError`](src/error.rs) covering every host interface.
- Helper types for the JSON-encoded payloads the WIT surface exchanges
  ([`LlmOptions`](src/types.rs), [`DbRow`](src/types.rs), …).
- Trait abstractions ([`Database`](src/db.rs), [`Llm`](src/llm.rs),
  [`Kv`](src/kv.rs), [`Identity`](src/identity.rs),
  [`Secrets`](src/secrets.rs)) that skill authors implement as one-line
  adapters over their generated `bindings::flint::host::*` module.

The crate **compiles on any target** (including `wasm32-wasip2`) because it
contains no WIT calls of its own — all actual host calls live in the skill
author's component crate.

## Quick start

### 1. Create a skill component crate

```bash
cargo component new --lib my-skill
cd my-skill
```

### 2. Point the component at the Flint world

`my-skill/Cargo.toml`:

```toml
[package]
name = "my-skill"
version = "0.1.0"
edition = "2021"

[dependencies]
wit-bindgen-rt = { version = "0.44", features = ["bitflags"] }
flint-skill = { path = "../../crates/flint-skill" }   # or registry version
serde_json = "1"

[lib]
crate-type = ["cdylib"]

[package.metadata.component]
package = "component:my-skill"

[package.metadata.component.target]
path = "../../wit/flint/host"
world = "edge-function"

[package.metadata.component.target.dependencies]
"wasi:http" = { version = "0.2.12" }
```

### 3. Generate the WIT bindings and write adapters

`my-skill/src/lib.rs`:

```rust
// Generate the WIT host import bindings inline. This produces the
// `bindings::flint::host::{db, llm, kv, identity, secrets}` modules that
// the flint-skill adapters below delegate to.
wit_bindgen_rt::generate!({
    path: "../../wit/flint/host",
    world: "edge-function",
});

use flint_skill::{
    CompletionResult, Database, DbRow, EmbeddingResult, HostInterface, Identity, Kv, Llm,
    LlmOptions, SecretHandle, Secrets, SkillError, SkillResult,
};
use serde_json::Value;

struct Host;

impl Database for Host {
    async fn query(&self, sql: &str, params: &[String]) -> SkillResult<Vec<DbRow>> {
        let rows = bindings::flint::host::db::query(sql, params.to_vec())
            .await
            .map_err(|e| SkillError::from_host_error(
                HostInterface::Db, e.code, e.message,
            ))?;
        rows.iter().map(|s| DbRow::from_json_str(s)).collect()
    }
}

impl Llm for Host {
    async fn complete(&self, prompt: &str, opts: &LlmOptions) -> SkillResult<CompletionResult> {
        let text = bindings::flint::host::llm::complete(prompt, opts.to_json()?)
            .await
            .map_err(|e| SkillError::from_host_error(
                HostInterface::Llm, e.code, e.message,
            ))?;
        Ok(CompletionResult { text })
    }

    async fn embed(&self, input: &str, model: Option<&str>) -> SkillResult<EmbeddingResult> {
        let vector = bindings::flint::host::llm::embed(input, model.map(String::from))
            .await
            .map_err(|e| SkillError::from_host_error(
                HostInterface::Llm, e.code, e.message,
            ))?;
        Ok(EmbeddingResult { vector })
    }
}

impl Kv for Host {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        bindings::flint::host::kv::get(key)
    }
    fn set(&self, key: &str, value: &[u8]) {
        bindings::flint::host::kv::set(key, value.to_vec())
    }
}

impl Identity for Host {
    fn origin_jwt(&self) -> Option<String> {
        bindings::flint::host::identity::origin_jwt()
    }
    fn claims_json(&self) -> String {
        bindings::flint::host::identity::claims()
    }
}

// `Secrets`/`SecretHandle` are slightly more involved because the WIT
// `secret` is a resource. See examples/secrets-component for a complete
// adapter; the surface skill code uses is identical to the above.

// Export the wasi:http/incoming-handler entry point that the Kiln runtime
// dispatches into. Inside the handler, instantiate `Host` and call through
// the trait methods — every host call is now typed and Cedar-deny-aware.
bindings::export!(Component with_types_in = bindings);

struct Component;

impl bindings::exports::wasi::http::incoming_handler::Guest for Component {
    fn handle(_request: bindings::exports::wasi::http::types::IncomingRequest,
              _response: bindings::exports::wasi::http::types::ResponseOutparam) {
        // let host = Host;
        // let rows = futures::executor::block_on(host.query("SELECT 1", &[])).unwrap();
        // …build response…
    }
}
```

### 4. Map WIT `host-error` to `SkillError`

Every host interface that can fail returns a `host-error { code, message }`
record. The single conversion entry point is
[`SkillError::from_host_error`](src/error.rs):

```rust
let rows = bindings::flint::host::db::query(sql, params)
    .await
    .map_err(|e| SkillError::from_host_error(HostInterface::Db, e.code, e.message))?;
```

Switch on the code to detect Cedar denials, rate limits, etc.:

```rust
match err {
    SkillError::Secrets { code, .. } if code == "CEDAR_DENY" => {
        // publisher has no reveal grant; fall back to host-brokered call
    }
    SkillError::Llm { code, .. } if code == "PROVIDER_429" => {
        // back off and retry
    }
    _ => return Err(err),
}
```

## Stability

All types in this crate track `flint:host@0.1.0`. Breaking changes in the WIT
world will bump this crate's minor version and be announced in
[`docs/api/kiln-abi.md`](../../docs/api/kiln-abi.md).

## License

MIT, same as the rest of the Flint Forge workspace.
