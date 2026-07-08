//! flint_hooks — webhook dispatch. Registry-driven generic trigger; Option-3 forwarding; two tiers.
use pgrx::prelude::*;

pgrx::pg_module_magic!();

extension_sql_file!("../sql/flint_hooks.sql", name = "flint_hooks_schema");

/// Returns the version string for this extension.
#[pg_extern]
fn flint_hooks_version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_o: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    /// Verify that pgcrypto hmac() produces a correctly formatted HMAC-SHA256 signature header.
    /// The prefix "sha256=" and 64-character hex digest are both required by consumers.
    #[pg_test]
    fn test_hmac_signature_format() {
        let sig = Spi::get_one::<String>(
            "SELECT 'sha256=' || encode(hmac('test-payload', 'test-secret', 'sha256'), 'hex')",
        )
        .unwrap()
        .unwrap();

        assert!(sig.starts_with("sha256="), "signature must start with sha256=");
        // sha256 produces a 256-bit (32-byte) digest; 64 hex characters after the 7-char prefix.
        assert_eq!(sig.len(), 7 + 64, "sha256 hex digest must be 64 chars");
    }

    /// With no matching webhooks registered, dispatch_webhook() must be a no-op and not error.
    #[pg_test]
    fn test_webhook_dispatch_no_matching_webhooks() {
        // Create a test table and bind the dispatch trigger.
        Spi::run("CREATE TABLE IF NOT EXISTS public.hooks_test_002 (id int)").unwrap();
        Spi::run(
            "CREATE OR REPLACE TRIGGER flint_dispatch_test \
             AFTER INSERT ON public.hooks_test_002 \
             FOR EACH ROW EXECUTE FUNCTION flint.dispatch_webhook()",
        )
        .unwrap();

        // Insert with no registered webhooks — must not raise an error.
        Spi::run("INSERT INTO public.hooks_test_002 VALUES (42)").unwrap();

        // Cleanup.
        Spi::run("DROP TABLE IF EXISTS public.hooks_test_002 CASCADE").unwrap();
    }

    /// With an empty outbox, process_webhook_outbox() must return 0 without error.
    #[pg_test]
    fn test_process_webhook_outbox_returns_zero_when_empty() {
        // With no pending outbox entries, the dispatcher should return 0 processed.
        let count = Spi::get_one::<i32>(
            "SELECT flint.process_webhook_outbox()"
        ).unwrap().unwrap_or(0);
        assert_eq!(count, 0, "dispatcher returns 0 when outbox is empty");
    }

}