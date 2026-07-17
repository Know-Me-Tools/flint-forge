# p11-c003 Tasks — SDK v1.0 Alignment

## Tasks

- [x] Bump `packages/flint-react/package.json` `"version"` from `"0.1.0"` to `"1.0.0"`
- [x] Bump `packages/flint_genui/pubspec.yaml` `version:` from `0.1.0` to `1.0.0`
- [x] Write `packages/flint-react/CHANGELOG.md` (1.0.0 section documenting p5–p10 additions)
- [x] Write `packages/flint_genui/CHANGELOG.md` (1.0.0 section documenting p8 additions)
- [x] Write `MIGRATION.md` at workspace root — v0.10.0→v1.0.0 delta: new `#[non_exhaustive]` match arm requirements, WIT `@since` annotations, Dockerfile entrypoint behaviour, SDK version bumps — 179 lines
- [x] Validate `packages/flint-react/package.json` is valid JSON: `python3 -m json.tool packages/flint-react/package.json`
- [x] Validate `packages/flint_genui/pubspec.yaml` is valid YAML: `python3 -c "import yaml; yaml.safe_load(open('packages/flint_genui/pubspec.yaml'))"`
- [x] `cargo test --workspace` passes (no Rust changes)
