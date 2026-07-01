//! Wasmtime host: ProxyPre cache, Cedar-gated linker, fuel/epoch limits, .cwasm (de)serialize.
//! Shares its component-host primitives with UAR Tier-2 skills.
#![forbid(unsafe_code)]

use fke_domain::Capability;

/// The data-plane runner. Built WITHOUT compiler features — it can only deserialize + run.
pub struct EdgeRuntime {/* wasmtime::Engine (compiler off), ProxyPre cache */}

impl EdgeRuntime {
    pub fn new() -> Self {
        Self {}
    }

    /// Load a pre-compiled native artifact. RCE-sensitive: only ever fed control-plane output.
    pub fn load_cwasm(&self, _native: &[u8]) -> placeholder::Result<()> {
        todo!("deserialize component")
    }

    /// Instantiate per request (fresh Store) and dispatch via wasi:http/incoming-handler.
    pub fn handle(&self, _granted: &[Capability]) {
        todo!("fresh Store + ProxyPre::instantiate_async + fuel/epoch")
    }
}

impl Default for EdgeRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// Placeholder error/result until wasmtime + anyhow land (p5-c001).
mod placeholder {
    #[derive(Debug)]
    pub struct RuntimeError;
    pub type Result<T> = core::result::Result<T, RuntimeError>;
}

/// Control-plane compiler (Cranelift). Present only in the admin build via a feature flag.
#[cfg(feature = "compiler")]
pub struct AotCompiler;
#[cfg(feature = "compiler")]
impl AotCompiler {
    pub fn precompile(&self, _artifact: &[u8], _target: &fke_domain::TargetArch) -> Vec<u8> {
        todo!("Engine::precompile_component → .cwasm; cache key (digest,target,wasmtime_version)")
    }
}
