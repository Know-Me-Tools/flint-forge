-- Authored migration only. Runtime enablement and policy authorization are separate.
-- file-schema-v1 is a strict JSON Schema subset, never A2UI property schemas.
CREATE SCHEMA flint_files;
REVOKE ALL ON SCHEMA flint_files FROM PUBLIC;

-- Pure, bounded validator: unsupported vocabulary fails registration, never silently ignored.
CREATE FUNCTION flint_files.schema_valid(s jsonb, depth integer DEFAULT 0)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT SET search_path = pg_catalog AS $$
DECLARE k text; v jsonb; item jsonb; typ text;
BEGIN
  IF depth > 16 OR jsonb_typeof(s) <> 'object' OR octet_length(s::text) > 65536 THEN RETURN false; END IF;
  typ := s->>'type';
  IF typ IS NULL OR typ NOT IN ('object','array','string','number','integer','boolean','null') THEN RETURN false; END IF;
  FOR k,v IN SELECT * FROM jsonb_each(s) LOOP
    IF k NOT IN ('type','properties','required','additionalProperties','items','enum','title','description') THEN RETURN false; END IF;
    IF k IN ('title','description') AND jsonb_typeof(v) <> 'string' THEN RETURN false; END IF;
  END LOOP;
  IF s ? 'properties' THEN
    IF jsonb_typeof(s->'properties') <> 'object' THEN RETURN false; END IF;
    FOR v IN SELECT value FROM jsonb_each(s->'properties') LOOP
      IF NOT flint_files.schema_valid(v, depth+1) THEN RETURN false; END IF;
    END LOOP;
  END IF;
  IF s ? 'required' THEN
    IF jsonb_typeof(s->'required') <> 'array' THEN RETURN false; END IF;
    FOR item IN SELECT * FROM jsonb_array_elements(s->'required') LOOP
      IF jsonb_typeof(item) <> 'string' THEN RETURN false; END IF;
    END LOOP;
    IF (SELECT count(*) <> count(DISTINCT value) FROM jsonb_array_elements(s->'required')) THEN RETURN false; END IF;
  END IF;
  IF s ? 'additionalProperties' AND jsonb_typeof(s->'additionalProperties') <> 'boolean' THEN RETURN false; END IF;
  IF s ? 'items' AND NOT flint_files.schema_valid(s->'items',depth+1) THEN RETURN false; END IF;
  IF s ? 'enum' THEN
    IF jsonb_typeof(s->'enum') <> 'array' OR jsonb_array_length(s->'enum') = 0 THEN RETURN false; END IF;
    IF (SELECT count(*) <> count(DISTINCT value) FROM jsonb_array_elements(s->'enum')) THEN RETURN false; END IF;
  END IF;
  RETURN true;
END $$;

CREATE FUNCTION flint_files.value_valid(s jsonb, v jsonb, depth integer DEFAULT 0)
RETURNS boolean LANGUAGE plpgsql IMMUTABLE STRICT SET search_path = pg_catalog AS $$
DECLARE typ text; k text; child jsonb;
BEGIN
  IF depth > 16 THEN RETURN false; END IF;
  typ := s->>'type';
  IF typ = 'integer' THEN
    IF jsonb_typeof(v) <> 'number' THEN RETURN false; END IF;
    IF (v::text)::numeric <> trunc((v::text)::numeric) THEN RETURN false; END IF;
  ELSIF typ IS DISTINCT FROM jsonb_typeof(v) THEN RETURN false;
  END IF;
  IF s ? 'enum' AND NOT EXISTS (SELECT 1 FROM jsonb_array_elements(s->'enum') e WHERE e = v) THEN RETURN false; END IF;
  -- Keywords retain JSON Schema semantics: object/array-specific constraints
  -- do not apply to other instance types, even if present in their schema.
  IF jsonb_typeof(v) = 'object' THEN
    FOR k IN SELECT jsonb_array_elements_text(COALESCE(s->'required','[]')) LOOP
      IF NOT v ? k THEN RETURN false; END IF;
    END LOOP;
    FOR k,child IN SELECT * FROM jsonb_each(v) LOOP
      IF COALESCE(s->'properties','{}') ? k THEN
        IF NOT flint_files.value_valid(s->'properties'->k,child,depth+1) THEN RETURN false; END IF;
      ELSIF s->'additionalProperties' = 'false'::jsonb THEN RETURN false;
      END IF;
    END LOOP;
  ELSIF jsonb_typeof(v) = 'array' AND s ? 'items' THEN
    FOR child IN SELECT * FROM jsonb_array_elements(v) LOOP
      IF NOT flint_files.value_valid(s->'items',child,depth+1) THEN RETURN false; END IF;
    END LOOP;
  END IF;
  RETURN true;
END $$;

CREATE FUNCTION flint_files.labels_valid(v jsonb)
RETURNS boolean LANGUAGE sql IMMUTABLE STRICT SET search_path = pg_catalog AS $$
 SELECT CASE WHEN jsonb_typeof(v) <> 'array' THEN false ELSE
 jsonb_array_length(v) <= 64 AND NOT EXISTS
 (SELECT 1 FROM jsonb_array_elements(v) x WHERE jsonb_typeof(x) <> 'string' OR length(x #>> '{}') NOT BETWEEN 1 AND 128) END
$$;

CREATE TABLE flint_files.types (
 tenant_id text NOT NULL, subject text NOT NULL,
 type_id text NOT NULL CHECK (type_id ~ '^[A-Za-z0-9._-]{1,128}$'),
 version bigint NOT NULL CHECK (version > 0),
 definition jsonb NOT NULL CHECK (octet_length(definition::text) <= 65536),
 PRIMARY KEY (tenant_id,subject,type_id,version)
);
CREATE TABLE flint_files.records (
 tenant_id text NOT NULL, subject text NOT NULL,
 file_id text NOT NULL CHECK (file_id ~ '^[A-Za-z0-9._-]{1,128}$'),
 revision bigint NOT NULL CHECK (revision > 0),
 type_id text NOT NULL, type_version bigint NOT NULL,
 data jsonb NOT NULL CHECK (octet_length(data::text) <= 65536),
 PRIMARY KEY (tenant_id,subject,file_id),
 FOREIGN KEY (tenant_id,subject,type_id,type_version) REFERENCES flint_files.types
);
CREATE TABLE flint_files.receipts (
 tenant_id text NOT NULL, subject text NOT NULL,
 mutation_id text NOT NULL CHECK (mutation_id ~ '^[A-Za-z0-9._-]{1,128}$'),
 request jsonb NOT NULL, response jsonb NOT NULL,
 PRIMARY KEY (tenant_id,subject,mutation_id)
);
CREATE INDEX file_attributes_gin ON flint_files.records USING gin ((data->'attributes'));
CREATE INDEX file_tags_gin ON flint_files.records USING gin ((data->'tags'));
CREATE INDEX file_categories_gin ON flint_files.records USING gin ((data->'categories'));
CREATE INDEX file_cid_idx ON flint_files.records (tenant_id,subject,(data->>'cid'));
CREATE INDEX file_type_idx ON flint_files.records (tenant_id,subject,type_id,file_id);

CREATE FUNCTION flint_files.check_type() RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE d jsonb; m jsonb; v jsonb; k text;
BEGIN
 IF TG_OP <> 'INSERT' THEN RAISE EXCEPTION 'immutable type version' USING ERRCODE='23514'; END IF;
 d := NEW.definition;
 IF jsonb_typeof(d) <> 'object'
 OR jsonb_typeof(d->'type_id') IS DISTINCT FROM 'string'
 OR d->>'type_id' IS DISTINCT FROM NEW.type_id
 OR jsonb_typeof(d->'version') IS DISTINCT FROM 'number'
 OR (d->>'version')::bigint IS DISTINCT FROM NEW.version
 OR d->>'schema_profile' IS DISTINCT FROM 'file-schema-v1'
 OR d->'schema'->>'type' IS DISTINCT FROM 'object'
 OR NOT COALESCE(flint_files.schema_valid(d->'schema'),false)
 THEN RAISE EXCEPTION 'invalid type definition' USING ERRCODE='23514'; END IF;
 FOR k IN SELECT jsonb_object_keys(d) LOOP
  IF k NOT IN ('type_id','version','schema_profile','schema','match') THEN RAISE EXCEPTION 'unsupported type field' USING ERRCODE='23514'; END IF;
 END LOOP;
 m := d->'match';
 IF jsonb_typeof(m) IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'invalid criteria' USING ERRCODE='23514'; END IF;
 FOR k IN SELECT jsonb_object_keys(m) LOOP
  IF k NOT IN ('mime_types','filename_suffixes') THEN RAISE EXCEPTION 'unsupported criteria' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF NOT COALESCE(flint_files.labels_valid(m->'mime_types'),false) OR NOT COALESCE(flint_files.labels_valid(m->'filename_suffixes'),false)
 THEN RAISE EXCEPTION 'invalid criteria' USING ERRCODE='23514'; END IF;
 FOR v IN SELECT * FROM jsonb_array_elements(m->'mime_types') LOOP
  IF (v #>> '{}') !~ '^[A-Za-z0-9!#$&^_.+-]+/([A-Za-z0-9!#$&^_.+-]+|\*)$' THEN RAISE EXCEPTION 'invalid MIME criteria' USING ERRCODE='23514'; END IF;
 END LOOP;
 RETURN NEW;
END $$;
CREATE TRIGGER file_type_guard BEFORE INSERT OR UPDATE OR DELETE ON flint_files.types FOR EACH ROW EXECUTE FUNCTION flint_files.check_type();

CREATE FUNCTION flint_files.check_record() RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE s jsonb; d jsonb; k text;
BEGIN
 d := NEW.data;
 IF jsonb_typeof(d) IS DISTINCT FROM 'object' THEN RAISE EXCEPTION 'invalid record' USING ERRCODE='23514'; END IF;
 IF TG_OP = 'UPDATE' THEN
  IF (NEW.tenant_id,NEW.subject,NEW.file_id) IS DISTINCT FROM (OLD.tenant_id,OLD.subject,OLD.file_id)
  OR d->>'cid' IS DISTINCT FROM OLD.data->>'cid' OR NEW.revision <> OLD.revision+1
  THEN RAISE EXCEPTION 'immutable identity or stale revision' USING ERRCODE='23514'; END IF;
 ELSIF NEW.revision <> 1 THEN RAISE EXCEPTION 'invalid initial revision' USING ERRCODE='23514'; END IF;
 FOR k IN SELECT jsonb_object_keys(d) LOOP
  IF k NOT IN ('cid','path','mime_type','type_id','type_version','attributes','tags','categories','expected_revision','mutation_id') THEN RAISE EXCEPTION 'unsupported record field' USING ERRCODE='23514'; END IF;
 END LOOP;
 IF jsonb_typeof(d->'type_id') IS DISTINCT FROM 'string'
 OR jsonb_typeof(d->'type_version') IS DISTINCT FROM 'number'
 OR jsonb_typeof(d->'expected_revision') IS DISTINCT FROM 'number'
 OR jsonb_typeof(d->'mutation_id') IS DISTINCT FROM 'string'
 OR d->>'type_id' IS DISTINCT FROM NEW.type_id OR (d->>'type_version')::bigint IS DISTINCT FROM NEW.type_version
 OR (d->>'expected_revision')::bigint IS DISTINCT FROM NEW.revision-1
 OR COALESCE(d->>'mutation_id','') !~ '^[A-Za-z0-9._-]{1,128}$'
 OR jsonb_typeof(d->'cid') IS DISTINCT FROM 'string' OR length(d->>'cid') NOT BETWEEN 1 AND 256 OR d->>'cid' ~ '\s'
 OR jsonb_typeof(d->'mime_type') IS DISTINCT FROM 'string' OR length(d->>'mime_type') NOT BETWEEN 1 AND 256
 OR (d->'path' IS NOT NULL AND d->'path' <> 'null' AND (jsonb_typeof(d->'path') <> 'string' OR length(d->>'path') > 4096))
 OR NOT COALESCE(flint_files.labels_valid(d->'tags'),false) OR NOT COALESCE(flint_files.labels_valid(d->'categories'),false)
 THEN RAISE EXCEPTION 'invalid record data' USING ERRCODE='23514'; END IF;
 SELECT definition->'schema' INTO s FROM flint_files.types WHERE tenant_id=NEW.tenant_id AND subject=NEW.subject AND type_id=NEW.type_id AND version=NEW.type_version;
 IF s IS NULL OR NOT COALESCE(flint_files.value_valid(s,d->'attributes'),false) THEN RAISE EXCEPTION 'invalid attributes' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER file_record_guard BEFORE INSERT OR UPDATE ON flint_files.records FOR EACH ROW EXECUTE FUNCTION flint_files.check_record();

-- Generic GraphQL/REST cannot forge replay success without a matching persisted
-- revision. Receipts are append-only; historical replay remains valid after edits.
CREATE FUNCTION flint_files.check_receipt() RETURNS trigger LANGUAGE plpgsql SET search_path = pg_catalog AS $$
DECLARE actual jsonb;
BEGIN
 IF TG_OP <> 'INSERT' THEN RAISE EXCEPTION 'immutable receipt' USING ERRCODE='23514'; END IF;
 SELECT data || jsonb_build_object('file_id',file_id,'revision',revision) INTO actual
 FROM flint_files.records WHERE tenant_id=NEW.tenant_id AND subject=NEW.subject
 AND file_id=NEW.request->>'file_id' FOR SHARE;
 IF actual IS NULL OR NEW.response IS DISTINCT FROM actual
 OR NEW.request IS DISTINCT FROM jsonb_build_object('file_id',actual->'file_id','data',actual-'file_id'-'revision')
 OR NEW.mutation_id IS DISTINCT FROM actual->>'mutation_id'
 THEN RAISE EXCEPTION 'invalid receipt' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER file_receipt_guard BEFORE INSERT OR UPDATE OR DELETE ON flint_files.receipts FOR EACH ROW EXECUTE FUNCTION flint_files.check_receipt();

ALTER TABLE flint_files.types ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint_files.types FORCE ROW LEVEL SECURITY;
ALTER TABLE flint_files.records ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint_files.records FORCE ROW LEVEL SECURITY;
ALTER TABLE flint_files.receipts ENABLE ROW LEVEL SECURITY;
ALTER TABLE flint_files.receipts FORCE ROW LEVEL SECURITY;
CREATE POLICY private_types ON flint_files.types TO authenticated
 USING (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '')
 WITH CHECK (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '');
CREATE POLICY private_records ON flint_files.records TO authenticated
 USING (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '')
 WITH CHECK (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '');
CREATE POLICY private_receipts ON flint_files.receipts TO authenticated
 USING (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '')
 WITH CHECK (subject = current_setting('request.jwt.claims',true)::jsonb->>'sub' AND tenant_id = current_setting('request.jwt.claims',true)::jsonb->>'tenant_id' AND subject <> '' AND tenant_id <> '');
REVOKE ALL ON ALL FUNCTIONS IN SCHEMA flint_files FROM PUBLIC;
GRANT USAGE ON SCHEMA flint_files TO authenticated;
GRANT SELECT, INSERT ON flint_files.types, flint_files.receipts TO authenticated;
GRANT SELECT, INSERT, UPDATE ON flint_files.records TO authenticated;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA flint_files TO authenticated;
-- No Cedar permit is seeded. An operator must explicitly authorize these resources.
