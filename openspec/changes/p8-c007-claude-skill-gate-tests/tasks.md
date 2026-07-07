# p8-c007 Tasks — Claude Skill Gate Tests

## Tasks

- [ ] Create `crates/fdb-gateway/tests/skill_catalog_test.rs` — slug accuracy test (skips when `DATABASE_URL` not set)
- [ ] Verify test passes locally with a running DB: `DATABASE_URL=... cargo test -p fdb-gateway skill_catalog_slugs_match_db`
- [ ] Add `## Installation` section to `skills/flint-ui/SKILL.md` documenting `claude plugin install`
- [ ] Verify `skills/flint-ui/catalogs/components.md` has all 55 slugs in `### \`<slug>\`` format (required by the test parser)
- [ ] `cargo clippy -p fdb-gateway -- -D warnings` clean (test file must not add warnings)
