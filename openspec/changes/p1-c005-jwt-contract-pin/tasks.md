# p1-c005 — Tasks

- [x] Read flint-gate `jwt_verify.rs` — extract inbound claim shape (sub, OIDC traits, metadata_public)
- [x] Read flint-gate `jwt_mint.rs` — extract minted claim shape (iss, sub, iat, exp, jti + merged claims)
- [x] Read flint-gate `identity.rs` — confirm Identity struct field mapping
- [x] Read flint-gate `config/types.rs` — confirm JwtConfig defaults (HS256, issuer=flint-gate, ttl=300)
- [x] Read flint-gate `middleware/pipeline.rs` — confirm header injection sequence and additional_claims merge
- [x] Write `docs/contracts/jwt-contract.md` — all sections complete
- [x] Document critical `role` claim requirement: must be in additional_claims, not auto-included
- [x] Document service-identity format (OQ-5): sub=service-name, role=service_role, scope=service-scope
- [ ] Update `openspec/changes/p1-c001-flint-auth/tasks.md` cross-reference (already done above)
- [x] GATE: `docs/contracts/jwt-contract.md` exists, reviewed, OQ-4 and OQ-5 closed

## Notes

This change is complete. The [x] items were completed during the assessment phase of this planning session.
