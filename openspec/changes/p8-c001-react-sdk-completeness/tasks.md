# p8-c001 Tasks — `@flint/react` SDK Completeness

## Tasks

- [ ] Create `packages/flint-react/src/registry/slugMap.ts` mapping all 55 catalog slugs to exported components
- [ ] Export `fromSlug()` and `SLUG_MAP` from `src/index.ts`
- [ ] Add `useFlintRegistry()` hook to `src/provider/` — wraps `useFlint()` + `fromSlug`
- [ ] Export `useFlintRegistry` from `src/index.ts`
- [ ] Run `npm run size` in `packages/flint-react/` — confirm < 80 KB gzipped
- [ ] Fix any bundle over-limit by ensuring components are tree-shakeable
- [ ] Verify all 55 slugs are covered in `slugMap.ts` against `skills/flint-ui/catalogs/components.md`
- [ ] Run `npm run typecheck` in `packages/flint-react/` — zero type errors
- [ ] Run `npm test` in `packages/flint-react/` — all tests pass
