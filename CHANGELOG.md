# Changelog

All notable changes to Flint Forge are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] — 2026-07-07

### Bug Fixes
- **security**: Update crossbeam-epoch 0.9.18→0.9.20 (RUSTSEC-2026-0204) ([`6718592`](https://github.com/Know-Me-Tools/flint-forge/commit/67185926e2001c5238d1ee945a78204681bd758c))


### Features
- **perf,ops**: K6 baseline annotation + staging token rotation (p11-c004 + p11-c006) ([`ab5d7e7`](https://github.com/Know-Me-Tools/flint-forge/commit/ab5d7e77242d7e810317cded58edabc937e6e7e4))
- **ops**: Dockerfile entrypoint secrets wiring (p11-c005) ([`ce59e26`](https://github.com/Know-Me-Tools/flint-forge/commit/ce59e2659dbc8930f030a30b23d516685936c53d))
- **api**: A2UI + Kiln ABI freeze (p11-c001 + p11-c002) ([`6e393bd`](https://github.com/Know-Me-Tools/flint-forge/commit/6e393bdad090106dc16bcc3ad526ce8656bdf67f))


## [0.10.0] — 2026-07-07

### Bug Fixes
- **p35-c004**: Repair PgBackend::acquire RLS setup + add DB-integration tests (G2) (**BREAKING**) ([`35fdf01`](https://github.com/Know-Me-Tools/flint-forge/commit/35fdf01a27dd2593b266f5b334e0243d41a6d40a))
- **p35-c002**: De-flake keto_sync interval test via pure resolve_interval (G3) ([`007ce1f`](https://github.com/Know-Me-Tools/flint-forge/commit/007ce1f2dc19beda30d5f1645e4a45040919a165))
- **p35-c001**: Clear workspace clippy-pedantic blockers (G4) ([`f2946f3`](https://github.com/Know-Me-Tools/flint-forge/commit/f2946f38758181cd112670a3483b9b5d7cd536ae))


### Documentation
- **p3-c019**: Mark Phase-1 tasks (T1-T9) complete in change tasks ([`0f06d88`](https://github.com/Know-Me-Tools/flint-forge/commit/0f06d88d2a91218722e57f04bcd4a14b533fc312))
- Add Integration-First + Compile Economy development-management policy ([`0669504`](https://github.com/Know-Me-Tools/flint-forge/commit/066950455971399187bc1f28255a8b2c5b529c42))
- Add MIT license and rewrite README ([`124ad1d`](https://github.com/Know-Me-Tools/flint-forge/commit/124ad1de2e4eb02557e7c869d09c84749ac6afec))


### Features
- **p35-c003**: CI Postgres image + DB test runner + Dagger service binding (G1) ([`4ef8150`](https://github.com/Know-Me-Tools/flint-forge/commit/4ef81509181872c75dd5860b2af455816e089d88))
- **p3-c020**: In-process Postgres LISTEN/NOTIFY ChangeStreamSource (OQ-FRF-1 workaround) (#6) ([`094f74e`](https://github.com/Know-Me-Tools/flint-forge/commit/094f74e77a7a59ab6f87955de10ae2800b698b20))
- **p3-c019**: Wire resource embedding into the REST list handler (#5) ([`b786335`](https://github.com/Know-Me-Tools/flint-forge/commit/b78633594f82ee8e3fb64a6dffcf153b07f63c52))
- **p3-c019**: PostgREST parity pass — resource embedding, FTS, edge cases (T10-T13) ([`c4b89f0`](https://github.com/Know-Me-Tools/flint-forge/commit/c4b89f0d1fd03966b6106b48f7e49094d18b4d33))
- **p3-c019**: Route fdb-reflection REST handlers through fdb-query (T7) ([`b73b5bb`](https://github.com/Know-Me-Tools/flint-forge/commit/b73b5bbab3064df6ca70e7a500f04d2d93a69d71))
- **p3-c019**: Complete fdb-query read+write translator; wire PgRest::execute (T4-T6,T8) ([`96146cc`](https://github.com/Know-Me-Tools/flint-forge/commit/96146cc9f24fe2004dd7e6a27f00d9a5d6c8227a))
- **p3-c019**: Fdb-query crate foundation — PostgREST operator + safety layer ([`8251704`](https://github.com/Know-Me-Tools/flint-forge/commit/8251704204af597501351ed3040fe54777c04935))
- **p3-g4**: Wire GraphQL subscription seam to RLS-filtered change stream ([`c03aae2`](https://github.com/Know-Me-Tools/flint-forge/commit/c03aae2c02e008e9fc3b23e7f521b5f15de9596f))
- **p3-c014**: REST mutation handlers (insert/update/delete) with Keto+Cedar gates ([`f43dccf`](https://github.com/Know-Me-Tools/flint-forge/commit/f43dccfdf85d0476b5fcc4be5b32adcab939f660))
- **p3-c013**: REST handle_list with 12 filter operators + is_safe_identifier ([`9bfd8eb`](https://github.com/Know-Me-Tools/flint-forge/commit/9bfd8eb640f28c31bde7d0355ff24b3abada1bd4))
- **p3**: Mount reflection router, KetoCheck port, Cedar policy engine ([`11486ce`](https://github.com/Know-Me-Tools/flint-forge/commit/11486ce7a3ad2f11aecf57bd0d7203620ecf61e8))


### Maintenance
- Initial commit — Flint Forge scaffold + KBD orchestrator state ([`2927d55`](https://github.com/Know-Me-Tools/flint-forge/commit/2927d5550ce4cc5e16ca1361332bfd0b11459f35))


### Style
- **p35-c001**: Cargo fmt --all — normalize pre-existing workspace drift (G4) ([`e5f438d`](https://github.com/Know-Me-Tools/flint-forge/commit/e5f438dca568578805299e2f06ed1605bf37cfe4))


### Testing
- **p3-c015**: REST filter-safety + vault DEK serde security gates ([`a0b180d`](https://github.com/Know-Me-Tools/flint-forge/commit/a0b180de1a87976135a57dee0dbb78e2b2da815c))



