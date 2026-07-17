//! Ahead-of-time Cranelift compilation for Flint Kiln components (control plane).

use anyhow::{Context, Result};
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::helpers::wt;

/// Control-plane, ahead-of-time Cranelift compiler for Kiln components.
///
/// Only present behind the `compiler` feature — it is not part of the Kiln
/// data-plane (`EdgeRuntime`), which loads components JIT-style via
/// `Component::from_binary`. `AotCompiler` is used out-of-band (e.g. by a
/// build pipeline or admin endpoint) to pre-serialize a `.cwasm` artifact so
/// the data plane can later load it without repeating compilation.
pub struct AotCompiler {
    /// The Wasmtime engine used to compile and serialize components. Built
    /// once with the Component Model enabled; deliberately not the same
    /// `Engine` instance as any `EdgeRuntime`, since AOT compilation and
    /// data-plane execution are separate concerns run in separate processes.
    engine: Engine,
}

impl AotCompiler {
    /// Build a new compiler with the WASM Component Model enabled.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying `wasmtime::Engine` fails to
    /// initialize (e.g. an unsupported target or invalid `Config`).
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        Ok(Self {
            engine: wt(Engine::new(&cfg)).context("AotCompiler Engine::new")?,
        })
    }

    /// Ahead-of-time compile a WASM component binary and return its
    /// serialized (`.cwasm`) form.
    ///
    /// `_target` is accepted for the eventual cross-compilation control-plane
    /// API (see module docs) but is not yet consulted — compilation always
    /// targets the host the `AotCompiler` was constructed on.
    ///
    /// # Errors
    ///
    /// Returns an error if `artifact` is not a valid WASM component binary
    /// (`Component::from_binary` fails), or if Wasmtime fails to serialize
    /// the compiled component (`Component::serialize` fails).
    pub fn precompile(&self, artifact: &[u8], _target: &fke_domain::TargetArch) -> Result<Vec<u8>> {
        let component = wt(Component::from_binary(&self.engine, artifact))
            .context("AotCompiler: Component::from_binary")?;
        wt(component.serialize()).context("Component::serialize")
    }
}

impl Default for AotCompiler {
    /// Build a compiler with default settings.
    ///
    /// # Panics
    ///
    /// Panics if [`AotCompiler::new`] fails (i.e. if `wasmtime::Engine`
    /// initialization fails). Prefer [`AotCompiler::new`] directly in
    /// contexts where engine construction failure must be handled instead of
    /// panicking.
    fn default() -> Self {
        Self::new().expect("AotCompiler::default")
    }
}
