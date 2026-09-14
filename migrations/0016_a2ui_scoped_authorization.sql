-- A2UI catalog authorization. Helper functions read verified transaction claims;
-- their callers cannot supply another principal's claims as an argument.
CREATE OR REPLACE FUNCTION flint_a2ui.current_subject() RETURNS text
LANGUAGE sql STABLE SET search_path = pg_catalog AS $$
  SELECT COALESCE(
    NULLIF(COALESCE(NULLIF(current_setting('request.jwt.claims', true), ''),
                    NULLIF(current_setting('app.jwt_claims', true), ''), '{}')::jsonb #>> '{flint,user_id}', ''),
    NULLIF(COALESCE(NULLIF(current_setting('request.jwt.claims', true), ''),
                    NULLIF(current_setting('app.jwt_claims', true), ''), '{}')::jsonb ->> 'sub', '')
  );
$$;

CREATE FUNCTION flint_a2ui.owns_application(app uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT EXISTS (SELECT 1 FROM flint_a2ui.applications
                 WHERE id = app AND owner_id::text = flint_a2ui.current_subject());
$$;
CREATE FUNCTION flint_a2ui.is_application_member(app uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT flint_a2ui.owns_application(app) OR EXISTS (
    SELECT 1 FROM flint_a2ui.role_assignments
    WHERE application_id = app AND user_id = flint_a2ui.current_subject());
$$;
CREATE FUNCTION flint_a2ui.is_application_admin(app uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT flint_a2ui.owns_application(app) OR EXISTS (
    SELECT 1 FROM flint_a2ui.role_assignments ra
    JOIN flint_a2ui.roles r ON r.id = ra.role_id AND r.application_id = ra.application_id
    WHERE ra.application_id = app AND ra.user_id = flint_a2ui.current_subject()
      AND r.slug = 'app-admin');
$$;
CREATE FUNCTION flint_a2ui.can_read_component(component uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT EXISTS (SELECT 1 FROM flint_a2ui.components c WHERE c.id = component
    AND (c.is_base OR c.application_id IS NULL OR flint_a2ui.is_application_member(c.application_id)));
$$;
CREATE FUNCTION flint_a2ui.can_manage_component(component uuid) RETURNS boolean
LANGUAGE sql STABLE SECURITY DEFINER SET search_path = pg_catalog AS $$
  SELECT EXISTS (SELECT 1 FROM flint_a2ui.components c WHERE c.id = component
    AND NOT c.is_base AND c.application_id IS NOT NULL
    AND flint_a2ui.is_application_admin(c.application_id));
$$;

-- BYPASSRLS helper ownership avoids recursive policies on membership tables.
DO $$ DECLARE f record; BEGIN
  FOR f IN SELECT p.oid::regprocedure AS signature FROM pg_proc p
    JOIN pg_namespace n ON n.oid=p.pronamespace
    WHERE n.nspname='flint_a2ui' AND p.proname IN
      ('current_subject','owns_application','is_application_member','is_application_admin',
       'can_read_component','can_manage_component')
  LOOP
    EXECUTE format('ALTER FUNCTION %s OWNER TO service_role', f.signature);
    EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC, anon', f.signature);
    EXECUTE format('GRANT EXECUTE ON FUNCTION %s TO authenticated, service_role', f.signature);
  END LOOP;
END $$;

-- Enforce application consistency even for privileged writes.
ALTER TABLE flint_a2ui.roles ADD CONSTRAINT roles_id_application_unique UNIQUE (id, application_id);
ALTER TABLE flint_a2ui.roles ADD CONSTRAINT roles_parent_same_application
  FOREIGN KEY (parent_role_id, application_id) REFERENCES flint_a2ui.roles(id, application_id);
ALTER TABLE flint_a2ui.role_assignments ADD CONSTRAINT assignment_role_same_application
  FOREIGN KEY (role_id, application_id) REFERENCES flint_a2ui.roles(id, application_id);

DO $$ DECLARE t text; p record; BEGIN
  FOREACH t IN ARRAY ARRAY['applications','roles','role_assignments','design_systems',
    'assembly_rules','schemas','bindings','embeddings','components','component_overrides','events']
  LOOP
    EXECUTE format('ALTER TABLE flint_a2ui.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE flint_a2ui.%I FORCE ROW LEVEL SECURITY', t);
    FOR p IN SELECT policyname FROM pg_policies WHERE schemaname='flint_a2ui' AND tablename=t
    LOOP EXECUTE format('DROP POLICY %I ON flint_a2ui.%I',p.policyname,t); END LOOP;
    EXECUTE format('REVOKE ALL ON flint_a2ui.%I FROM PUBLIC, anon',t);
    EXECUTE format('CREATE POLICY service_access ON flint_a2ui.%I TO service_role USING (true) WITH CHECK (true)', t);
  END LOOP;
END $$;

CREATE POLICY application_read ON flint_a2ui.applications FOR SELECT TO authenticated
  USING (is_system OR flint_a2ui.is_application_member(id));
CREATE POLICY application_create ON flint_a2ui.applications FOR INSERT TO authenticated
  WITH CHECK (NOT is_system AND owner_id::text = flint_a2ui.current_subject());
CREATE POLICY application_update ON flint_a2ui.applications FOR UPDATE TO authenticated
  USING (NOT is_system AND flint_a2ui.owns_application(id))
  WITH CHECK (NOT is_system AND owner_id::text = flint_a2ui.current_subject());
CREATE POLICY application_delete ON flint_a2ui.applications FOR DELETE TO authenticated
  USING (NOT is_system AND flint_a2ui.owns_application(id));

CREATE POLICY role_read ON flint_a2ui.roles FOR SELECT TO authenticated
  USING (flint_a2ui.is_application_member(application_id));
CREATE POLICY role_manage ON flint_a2ui.roles FOR ALL TO authenticated
  USING (flint_a2ui.owns_application(application_id)) WITH CHECK (flint_a2ui.owns_application(application_id));
CREATE POLICY assignment_read ON flint_a2ui.role_assignments FOR SELECT TO authenticated
  USING (user_id = flint_a2ui.current_subject() OR flint_a2ui.owns_application(application_id));
CREATE POLICY assignment_manage ON flint_a2ui.role_assignments FOR ALL TO authenticated
  USING (flint_a2ui.owns_application(application_id)) WITH CHECK (flint_a2ui.owns_application(application_id));

CREATE POLICY component_read ON flint_a2ui.components FOR SELECT TO authenticated
  USING (is_base OR application_id IS NULL OR flint_a2ui.is_application_member(application_id));
CREATE POLICY component_manage ON flint_a2ui.components FOR ALL TO authenticated
  USING (NOT is_base AND application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id))
  WITH CHECK (NOT is_base AND application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id));

DO $$ DECLARE t text; BEGIN
  FOREACH t IN ARRAY ARRAY['design_systems','assembly_rules'] LOOP
    EXECUTE format('CREATE POLICY scoped_read ON flint_a2ui.%I FOR SELECT TO authenticated USING (application_id IS NULL OR flint_a2ui.is_application_member(application_id))',t);
    EXECUTE format('CREATE POLICY scoped_manage ON flint_a2ui.%I FOR ALL TO authenticated USING (application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id)) WITH CHECK (application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id))',t);
  END LOOP;
  FOREACH t IN ARRAY ARRAY['schemas','bindings','embeddings'] LOOP
    EXECUTE format('CREATE POLICY component_read ON flint_a2ui.%I FOR SELECT TO authenticated USING (flint_a2ui.can_read_component(component_id))',t);
    IF t <> 'embeddings' THEN
      EXECUTE format('CREATE POLICY component_manage ON flint_a2ui.%I FOR ALL TO authenticated USING (flint_a2ui.can_manage_component(component_id)) WITH CHECK (flint_a2ui.can_manage_component(component_id))',t);
    END IF;
  END LOOP;
END $$;
REVOKE INSERT, UPDATE, DELETE ON flint_a2ui.embeddings FROM authenticated;

CREATE POLICY override_read ON flint_a2ui.component_overrides FOR SELECT TO authenticated
  USING (flint_a2ui.can_read_component(component_id)
    AND (application_id IS NULL OR flint_a2ui.is_application_member(application_id))
    AND (design_system_id IS NULL OR EXISTS (SELECT 1 FROM flint_a2ui.design_systems ds WHERE ds.id=design_system_id)));
CREATE POLICY override_manage ON flint_a2ui.component_overrides FOR ALL TO authenticated
  USING (application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id))
  WITH CHECK (application_id IS NOT NULL AND flint_a2ui.is_application_admin(application_id)
    AND flint_a2ui.can_read_component(component_id)
    AND (design_system_id IS NULL OR EXISTS (SELECT 1 FROM flint_a2ui.design_systems ds
         WHERE ds.id=design_system_id AND ds.application_id=component_overrides.application_id)));
CREATE POLICY event_read ON flint_a2ui.events FOR SELECT TO authenticated
  USING (application_id IS NULL OR flint_a2ui.is_application_member(application_id));
CREATE POLICY event_insert ON flint_a2ui.events FOR INSERT TO authenticated
  WITH CHECK ((application_id IS NULL OR flint_a2ui.is_application_member(application_id))
    AND actor=flint_a2ui.current_subject());

-- Legacy resolver signatures remain compatible, but identity arguments can no
-- longer grant visibility: queries execute under the caller's RLS transaction.
DO $$ DECLARE f record; BEGIN
  FOR f IN SELECT p.oid::regprocedure AS signature FROM pg_proc p
    JOIN pg_namespace n ON n.oid=p.pronamespace
    WHERE n.nspname='flint_a2ui' AND p.proname IN ('resolve_components',
      'resolve_components_with_overrides','resolve_application_roles','resolve_role_descendants')
  LOOP
    EXECUTE format('ALTER FUNCTION %s SECURITY INVOKER',f.signature);
    EXECUTE format('ALTER FUNCTION %s SET search_path = pg_catalog, flint_a2ui',f.signature);
    EXECUTE format('REVOKE ALL ON FUNCTION %s FROM PUBLIC, anon',f.signature);
  END LOOP;
END $$;
