-- The Singapore tongyi-embedding-vision-plus model returns exactly 1152 values.
-- Take the lock BEFORE checking emptiness so a concurrent backfill cannot race.
LOCK TABLE flint_a2ui.embeddings IN ACCESS EXCLUSIVE MODE;
DO $$ BEGIN
  IF EXISTS (SELECT 1 FROM flint_a2ui.embeddings) THEN
    RAISE EXCEPTION 'A2UI embedding conversion requires an empty table; preserve existing vectors and plan a model migration'
      USING ERRCODE='55000';
  END IF;
END $$;
ALTER TABLE flint_a2ui.embeddings ADD CONSTRAINT embeddings_component_entity_aspect_key
  UNIQUE (component_id,entity_type,aspect);
DROP INDEX flint_a2ui.idx_embeddings_hnsw;
ALTER TABLE flint_a2ui.embeddings ALTER COLUMN embedding TYPE vector(1152);
ALTER TABLE flint_a2ui.embeddings ALTER COLUMN model SET DEFAULT 'tongyi-embedding-vision-plus';
CREATE INDEX idx_embeddings_hnsw ON flint_a2ui.embeddings
  USING hnsw (embedding vector_cosine_ops) WITH (m=16,ef_construction=64);

CREATE OR REPLACE FUNCTION flint_a2ui.semantic_search(query_embedding vector, result_limit int DEFAULT 10)
RETURNS TABLE(component_id uuid,slug text,similarity float)
LANGUAGE plpgsql STABLE SECURITY INVOKER SET search_path=pg_catalog,public AS $$
BEGIN
 IF vector_dims(query_embedding) <> 1152 THEN
   RAISE EXCEPTION 'tongyi-embedding-vision-plus query requires 1152 dimensions' USING ERRCODE='22023';
 END IF;
 RETURN QUERY SELECT c.id,c.slug,1.0-(e.embedding <=> query_embedding)
 FROM flint_a2ui.embeddings e JOIN flint_a2ui.components c ON c.id=e.component_id
 WHERE e.aspect='description' AND e.model='tongyi-embedding-vision-plus'
 ORDER BY e.embedding <=> query_embedding LIMIT least(greatest(result_limit,1),100);
END $$;

CREATE OR REPLACE FUNCTION flint_a2ui.hybrid_search(query_text text,query_embedding vector,
 result_limit int DEFAULT 10,vector_weight float DEFAULT 0.7,text_weight float DEFAULT 0.3)
RETURNS TABLE(component_id uuid,slug text,score float)
LANGUAGE plpgsql STABLE SECURITY INVOKER SET search_path=pg_catalog,public AS $$
BEGIN
 IF vector_dims(query_embedding) <> 1152 THEN
   RAISE EXCEPTION 'tongyi-embedding-vision-plus query requires 1152 dimensions' USING ERRCODE='22023';
 END IF;
 RETURN QUERY SELECT c.id,c.slug,
   vector_weight*(1-(e.embedding <=> query_embedding))+text_weight*ts_rank(
     to_tsvector('english',COALESCE(c.description,'')||' '||c.slug),plainto_tsquery('english',query_text))
 FROM flint_a2ui.embeddings e JOIN flint_a2ui.components c ON c.id=e.component_id
 WHERE e.aspect='description' AND e.model='tongyi-embedding-vision-plus'
 ORDER BY 3 DESC LIMIT least(greatest(result_limit,1),100);
END $$;
REVOKE ALL ON FUNCTION flint_a2ui.semantic_search(vector,int),
 flint_a2ui.hybrid_search(text,vector,int,float,float) FROM PUBLIC,anon;
GRANT EXECUTE ON FUNCTION flint_a2ui.semantic_search(vector,int),
 flint_a2ui.hybrid_search(text,vector,int,float,float) TO authenticated,service_role;
