# p8-c006 Tasks — OpenDesign ZIP Import

## Tasks

- [ ] Add `zip = "2"` to `[workspace.dependencies]` in `Cargo.toml` (check `cargo search zip` version first)
- [ ] Add `zip = { workspace = true }` to `fdb-gateway/Cargo.toml`
- [ ] Implement `import_claude_design_zip(state, body)` in `design_import.rs`:
  - base64-decode `body.content` → ZIP bytes
  - `zip::ZipArchive::new(Cursor::new(bytes))` → iterate entries → find `DESIGN.md`
  - Call `import_design_md(state, new_body)` with extracted content
- [ ] Add `"claude_design_zip" => import_claude_design_zip(state, body).await` match arm
- [ ] Update `ImportBody.format` doc comment to include `"claude_design_zip"`
- [ ] Unit test: minimal ZIP containing `DESIGN.md` (base64-encoded) → returns `design_system_id`
- [ ] Unit test: ZIP with no `DESIGN.md` → 400 error
- [ ] Unit test: invalid base64 body → 400 error
- [ ] `cargo clippy -p fdb-gateway -- -D warnings` clean
- [ ] `cargo test -p fdb-gateway` passes
