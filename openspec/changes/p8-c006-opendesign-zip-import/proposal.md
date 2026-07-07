# p8-c006 — OpenDesign ZIP Import (`claude_design_zip` format)

**Phase:** 8 — SDK Completeness
**Priority:** P1
**Depends on:** none (p5-c013 `parse_design_md()` already exists)

## What this change delivers

Adds `format: "claude_design_zip"` support to `POST /a2ui/v1/design-systems/import`.
Extracts `DESIGN.md` from a base64-encoded ZIP body and passes it through the
existing `parse_design_md()` pipeline.

## Design

### New dep

```toml
# [workspace.dependencies]
zip = "2"
```

### `import_claude_design_zip()` in `design_import.rs`

```rust
async fn import_claude_design_zip(
    state: A2uiState,
    body: ImportBody,
) -> axum::response::Response {
    use std::io::Cursor;

    // Decode base64 body
    let zip_bytes = base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        &body.content,
    )
    .map_err(|_| ...)?;

    // Extract DESIGN.md from ZIP
    let mut archive = zip::ZipArchive::new(Cursor::new(zip_bytes))
        .map_err(|_| ...)?;

    let design_md = (0..archive.len())
        .find_map(|i| {
            let mut file = archive.by_index(i).ok()?;
            if file.name().ends_with("DESIGN.md") {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut file, &mut buf).ok()?;
                Some(buf)
            } else {
                None
            }
        })
        .ok_or_else(|| /* DESIGN.md not found error */)?;

    // Reuse existing import_design_md() logic
    let new_body = ImportBody {
        format: "design_md".to_owned(),
        content: design_md,
        design_system_id: body.design_system_id,
        name: body.name,
    };
    import_design_md(state, new_body).await
}
```

Add `"claude_design_zip" => import_claude_design_zip(state, body).await` arm.
