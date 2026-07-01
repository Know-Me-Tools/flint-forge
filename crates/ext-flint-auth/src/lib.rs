//! flint_auth — JWT/RLS context helpers. Defines the `auth` vocabulary RLS policies are written in.
//! NOTE: target is Postgres 18; pgrx `pg18` feature lands as the toolchain catches up (spec §8).
use pgrx::prelude::*;

pgrx::pg_module_magic!();

extension_sql_file!("sql/flint_auth.sql", name = "flint_auth_schema");

#[pg_extern]
fn flint_auth_version() -> &'static str { "0.1.0" }

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn version_present() { assert_eq!("0.1.0", crate::flint_auth_version()); }

    #[pg_test]
    fn test_uid_from_claims() {
        Spi::run("SELECT set_config('request.jwt.claims', '{\"sub\":\"user-abc-123\",\"role\":\"authenticated\"}', true)").unwrap();
        let uid = Spi::get_one::<String>("SELECT auth.uid()").unwrap().unwrap();
        assert_eq!(uid, "user-abc-123");
    }

    #[pg_test]
    fn test_role_with_claim_present() {
        Spi::run("SELECT set_config('request.jwt.claims', '{\"sub\":\"u1\",\"role\":\"authenticated\"}', true)").unwrap();
        let role = Spi::get_one::<String>("SELECT auth.role()").unwrap().unwrap();
        assert_eq!(role, "authenticated");
    }

    #[pg_test]
    fn test_role_fallback_to_anon() {
        Spi::run("SELECT set_config('request.jwt.claims', '{\"sub\":\"u1\"}', true)").unwrap();
        let role = Spi::get_one::<String>("SELECT auth.role()").unwrap().unwrap();
        assert_eq!(role, "anon");
    }

    #[pg_test]
    fn test_bearer_from_headers() {
        Spi::run("SELECT set_config('request.headers', '{\"authorization\":\"Bearer test-token-xyz\"}', true)").unwrap();
        let bearer = Spi::get_one::<String>("SELECT auth.bearer()").unwrap().unwrap();
        assert_eq!(bearer, "Bearer test-token-xyz");
    }

    #[pg_test]
    fn test_tenant_id_from_claims() {
        Spi::run("SELECT set_config('request.jwt.claims', '{\"sub\":\"u1\",\"tenant_id\":\"org-456\"}', true)").unwrap();
        let tid = Spi::get_one::<String>("SELECT auth.tenant_id()").unwrap().unwrap();
        assert_eq!(tid, "org-456");
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {}
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
