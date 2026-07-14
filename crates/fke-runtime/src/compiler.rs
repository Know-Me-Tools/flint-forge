//! Ahead-of-time Cranelift compilation for Flint Kiln components (control plane).

use anyhow::{Context, Result};
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::helpers::wt;

pub struct AotCompiler {
    engine: Engine,
}

impl AotCompiler {
    pub fn new() -> Result<Self> {
        let mut cfg = Config::new();
        cfg.wasm_component_model(true);
        Ok(Self {
            engine: wt(Engine::new(&cfg)).context("AotCompiler Engine::new")?,
        })
    }

    pub fn precompile(&self, artifact: &[u8], _target: &fke_domain::TargetArch) -> Result<Vec<u8>> {
        let component = wt(Component::from_binary(&self.engine, artifact))
            .context("AotCompiler: Component::from_binary")?;
        wt(component.serialize()).context("Component::serialize")
    }
}

impl Default for AotCompiler {
    fn default() -> Self {
        Self::new().expect("AotCompiler::default")
    }
}
