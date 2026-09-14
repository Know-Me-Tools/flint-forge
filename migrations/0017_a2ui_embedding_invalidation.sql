-- Persist invalidation so periodic recovery catches updates even if a replica
-- misses NOTIFY or loses the advisory-lock race. The embedder holds FOR SHARE
-- while reading and storing each component, preventing stale post-update writes.
CREATE FUNCTION flint_a2ui.invalidate_embedding() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog AS $$
BEGIN
  DELETE FROM flint_a2ui.embeddings WHERE component_id=NEW.id;
  PERFORM pg_notify('a2ui_embed',NEW.id::text);
  RETURN NEW;
END $$;
ALTER FUNCTION flint_a2ui.invalidate_embedding() OWNER TO service_role;
REVOKE ALL ON FUNCTION flint_a2ui.invalidate_embedding() FROM PUBLIC, anon, authenticated;
CREATE TRIGGER a2ui_component_embedding_changed
  AFTER UPDATE OF slug, primitive_type, category, description, schema, usage_examples
  ON flint_a2ui.components FOR EACH ROW
  EXECUTE FUNCTION flint_a2ui.invalidate_embedding();
