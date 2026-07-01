//! flint_llm (Flint Ember) — in-DB LLM via liter-llm, routed inward to flint-gate / UAR.
//! Surface 1: synchronous, interrupt/timeout-safe, gated.  Surface 2: async BGW + queue (default).
use pgrx::prelude::*;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/flint_llm.sql", name = "flint_llm_schema");

/// Surface 1 (sync) — read/explicit path only. Runs liter-llm on a dedicated runtime thread;
/// backend blocks under statement_timeout + CHECK_FOR_INTERRUPTS. Never default in a write trigger.
#[pg_extern]
fn llm_version() -> &'static str { "0.1.0" }

// TODO(p4-c002): #[pg_guard] background worker registered via shared_preload_libraries;
// dequeue llm.jobs FOR UPDATE SKIP LOCKED; batch; call flint-gate; write back via SPI.

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_o: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> { vec![] }
}
