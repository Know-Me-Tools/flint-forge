# p9-c005 Tasks — Security Audit

## Tasks

- [ ] Check if `tower-http` is already a direct dep in `fdb-gateway/Cargo.toml`; add if not
- [ ] Add `SetResponseHeaderLayer` for `X-Content-Type-Options: nosniff` to `fdb-gateway/src/main.rs`
- [ ] Add `SetResponseHeaderLayer` for `X-Frame-Options: DENY`
- [ ] Add `SetResponseHeaderLayer` for `Referrer-Policy: strict-origin-when-cross-origin`
- [ ] Rename `AllowAllPolicySource` → `TestAllowAllPolicySource` in `fke-server/src/kiln_policy.rs` and gate with `#[cfg(test)]`
- [ ] Update any test references to use the new name
- [ ] Audit all `tracing::` macros in `fdb-gateway` and `fke-server` — confirm no bearer/JWT/claims data
- [ ] Audit all `expect()` / `panic!()` in handler code (not startup) — eliminate or add context
- [ ] Verify `tools/call` in `routes/mcp.rs` validates tool name against `tool_definitions()` before dispatch
- [ ] Create `docs/security-audit.md` — OWASP Top 10 findings and mitigations
- [ ] Unit test: security headers present on `GET /healthz` response
- [ ] Unit test: security headers present on `GET /a2ui/v1/components` response
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
