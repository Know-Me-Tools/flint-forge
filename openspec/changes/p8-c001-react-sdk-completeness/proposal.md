# p8-c001 — `@flint/react` SDK Completeness

**Phase:** 8 — SDK Completeness
**Priority:** P0
**Depends on:** none
**Blocks:** p8-c007 (gate tests verify export accuracy)

## What this change delivers

- All 55 A2UI catalog slugs available as named exports from `@flint/react`
- `useFlintRegistry()` hook (alias/enhancement of `useFlint()`)
- Bundle size confirmed < 80 KB gzipped via `size-limit`
- `fromSlug(slug: string)` utility that returns the component for a given catalog slug

## Design

### Slug → export mapping

The current index exports components under semantic PascalCase names (`Stack`, `Card`, `DataGrid`).
The catalog uses slug names (`container`, `row`, `column`, `data-grid`, `text-input`). Add:

```ts
// src/registry/slugMap.ts
export const SLUG_MAP: Record<string, React.ComponentType<any>> = {
  'container':     Container,   // or Stack depending on semantics
  'row':           Row,
  'column':        Column,
  'data-grid':     DataGrid,
  'text-input':    TextField,
  // … all 55 slugs
};

export function fromSlug(slug: string): React.ComponentType<any> | undefined {
  return SLUG_MAP[slug];
}
```

Export `SLUG_MAP` and `fromSlug` from `index.ts`.

### `useFlintRegistry()` hook

```ts
export function useFlintRegistry() {
  const { catalog, gateway } = useFlint();
  return {
    listComponents: catalog.listComponents,
    getComponent:   catalog.getComponent,
    search:         catalog.search,
    fromSlug,
  };
}
```

### Bundle audit
Run `npm run size` in `packages/flint-react/`. If over 80 KB:
- Ensure all component exports are tree-shakeable (no barrel re-exports that import everything)
- Move heavy renderers to dynamic imports
