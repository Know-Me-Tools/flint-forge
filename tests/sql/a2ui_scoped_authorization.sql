\set ON_ERROR_STOP on
BEGIN;
-- Run only in an isolated database after migrations 0016 and 0017.
INSERT INTO flint_a2ui.applications(id,slug,name,owner_id) VALUES
 ('10000000-0000-0000-0000-000000000001','rls-fixture-a','A','20000000-0000-0000-0000-000000000001'),
 ('10000000-0000-0000-0000-000000000002','rls-fixture-b','B','20000000-0000-0000-0000-000000000002');
INSERT INTO flint_a2ui.roles(id,application_id,slug,name) VALUES
 ('30000000-0000-0000-0000-000000000001','10000000-0000-0000-0000-000000000001','member','Member'),
 ('30000000-0000-0000-0000-000000000002','10000000-0000-0000-0000-000000000001','app-admin','Administrator'),
 ('30000000-0000-0000-0000-000000000003','10000000-0000-0000-0000-000000000002','member','Member');
INSERT INTO flint_a2ui.role_assignments(application_id,role_id,user_id) VALUES
 ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000001','member-a'),
 ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000002','admin-a');
INSERT INTO flint_a2ui.components(id,slug,category,primitive_type,schema,application_id) VALUES
 ('40000000-0000-0000-0000-000000000001','rls-fixture-a','input','TextField','{}','10000000-0000-0000-0000-000000000001'),
 ('40000000-0000-0000-0000-000000000002','rls-fixture-b','input','TextField','{}','10000000-0000-0000-0000-000000000002');
SET LOCAL ROLE authenticated;
SELECT set_config('request.jwt.claims','{"sub":"member-a"}',true);
DO $$ BEGIN
 IF (SELECT count(*) FROM flint_a2ui.applications WHERE slug LIKE 'rls-fixture-%') <> 1 THEN
   RAISE EXCEPTION 'application isolation failed'; END IF;
 IF (SELECT count(*) FROM flint_a2ui.components WHERE slug LIKE 'rls-fixture-%') <> 1 THEN
   RAISE EXCEPTION 'component isolation failed'; END IF;
 IF EXISTS (SELECT 1 FROM flint_a2ui.resolve_components('10000000-0000-0000-0000-000000000002', '{"flint":{"user_id":"20000000-0000-0000-0000-000000000002"}}') WHERE slug='rls-fixture-b') THEN
   RAISE EXCEPTION 'caller claims bypassed RLS'; END IF;
 IF (SELECT count(*) FROM flint_a2ui.role_assignments WHERE application_id='10000000-0000-0000-0000-000000000001') <> 1 THEN
   RAISE EXCEPTION 'assignment isolation failed'; END IF;
 BEGIN
   INSERT INTO flint_a2ui.role_assignments(application_id,role_id,user_id) VALUES
    ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000002','member-a');
   RAISE EXCEPTION 'membership escalation permitted';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 BEGIN
   INSERT INTO flint_a2ui.design_systems(slug,name,application_id) VALUES
    ('rls-fixture-member-write','Denied','10000000-0000-0000-0000-000000000001');
   RAISE EXCEPTION 'member administration permitted';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
 BEGIN
   INSERT INTO flint_a2ui.embeddings(component_id,embedding) VALUES
    ('40000000-0000-0000-0000-000000000001',array_fill(0.1::real, ARRAY[(SELECT atttypmod FROM pg_attribute WHERE attrelid='flint_a2ui.embeddings'::regclass AND attname='embedding')])::vector);
   RAISE EXCEPTION 'member vector write permitted';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
SELECT set_config('request.jwt.claims','{"sub":"admin-a"}',true);
INSERT INTO flint_a2ui.design_systems(slug,name,application_id) VALUES
 ('rls-fixture-admin-write','Allowed','10000000-0000-0000-0000-000000000001');
DO $$ BEGIN
 BEGIN
   INSERT INTO flint_a2ui.role_assignments(application_id,role_id,user_id) VALUES
    ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000001','stranger');
   RAISE EXCEPTION 'non-owner membership management permitted';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
SELECT set_config('request.jwt.claims','{"sub":"20000000-0000-0000-0000-000000000001"}',true);
INSERT INTO flint_a2ui.role_assignments(application_id,role_id,user_id) VALUES
 ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000001','new-member');
DO $$ BEGIN
 BEGIN
   INSERT INTO flint_a2ui.role_assignments(application_id,role_id,user_id) VALUES
    ('10000000-0000-0000-0000-000000000001','30000000-0000-0000-0000-000000000003','cross-app');
   RAISE EXCEPTION 'cross-application role accepted';
 EXCEPTION WHEN foreign_key_violation THEN NULL; END;
END $$;
SET LOCAL ROLE anon;
DO $$ BEGIN
 BEGIN
   PERFORM 1 FROM flint_a2ui.components;
   RAISE EXCEPTION 'anonymous access permitted';
 EXCEPTION WHEN insufficient_privilege THEN NULL; END;
END $$;
RESET ROLE;
DO $$ BEGIN
 IF EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
   WHERE n.nspname='flint_a2ui' AND c.relkind='r' AND NOT(c.relrowsecurity AND c.relforcerowsecurity)) THEN
   RAISE EXCEPTION 'unprotected catalog table'; END IF;
END $$;
ROLLBACK;
