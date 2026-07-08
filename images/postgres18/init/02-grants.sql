-- Grant execute on Rust-backed flint_meta functions to service_role.
-- These functions are created by pgrx after the extension SQL runs, so the
-- grants must happen in a separate init script after CREATE EXTENSION.

GRANT EXECUTE ON FUNCTION flint_meta.check_permission(text, text, text, text) TO service_role;
GRANT EXECUTE ON FUNCTION flint_meta.set_identity(text) TO service_role;
GRANT EXECUTE ON FUNCTION flint_meta.full_refresh() TO service_role;
GRANT EXECUTE ON FUNCTION flint_meta.agui_descriptor() TO service_role;
GRANT EXECUTE ON FUNCTION flint_meta.openapi() TO service_role;

-- Lock down LLM internal helpers after they have been created by pgrx.
REVOKE ALL ON FUNCTION llm._embed_text(text, text) FROM PUBLIC;
REVOKE ALL ON FUNCTION llm._complete(text, text, jsonb) FROM PUBLIC;
REVOKE ALL ON FUNCTION llm._render_template(text, jsonb) FROM PUBLIC;
