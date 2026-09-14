# Know-me A2UI repair

Read-only diagnosis confirmed missing flint_llm binary, zero of 55 component embeddings, and eight API-granted tables without RLS. Inspected all three repositories and live database metadata. Existing unrelated working-tree changes will be preserved. Provider secret values are excluded from audit records.

- Installed provider keys in Kubernetes Secret liter-llm-providers via server-side apply; no credential values stored in Git.
- Live authenticated DashScope compatible-mode probe rejected tongyi-embedding-vision-plus (model_not_supported); awaiting endpoint/model choice.
- Added versioned Ember adoption script and production image overlay preserving the prior digest and volume.

- Applied migrations 0016/0017 only to isolated flint_a2ui_repair_test restored from the live schema and A2UI catalog. Live flint database unchanged.
- SQL authorization tests passed, including caller-claims spoofing denial. Dedicated API tests: 7 passed. Scoped Clippy passed. Ember cargo check passed using current SDK for CFLAGS/BINDGEN_EXTRA_CLANG_ARGS (no machine configuration modified).
- Generated internal Secret liter-llm-service and registered only its service-token SHA-256 digest in flintgate.public.api_keys. Initial schema-qualified insert failed without mutation; corrected public schema succeeded.
- Infra workflow now targets Ember production overlay and adds GQAdonis/liter-llm build. Liter manifests currently configure Token Plan chat only; embedding configuration/digest/deployment remain pending. No images deployed or Git commits/pushes made.
- Live Quarry ready replicas: 2. Model choice remains required because provider explicitly rejected the requested model/endpoint pair.
