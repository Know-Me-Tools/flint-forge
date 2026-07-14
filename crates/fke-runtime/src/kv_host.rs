//! `flint:host/kv` — ephemeral per-invocation key-value store.
//!
//! Backed by `KilnHostState::kv_store`, a plain in-memory map created fresh
//! per `Store` and dropped when the invocation ends. Not durable across
//! invocations — see the WIT doc comment on `interface kv`.

use crate::host_bindings::flint::host::kv::Host;
use crate::KilnHostState;

impl Host for KilnHostState {
    fn get(&mut self, k: String) -> Option<Vec<u8>> {
        self.kv_store.get(&k).cloned()
    }

    fn set(&mut self, k: String, v: Vec<u8>) {
        self.kv_store.insert(k, v);
    }
}
