-- Cedar is a coarse capability gate; the forced RLS policies in 0016 enforce
-- row ownership, application membership, administration and service-only writes.
-- Scope the permit to these catalog resources and mutation actions only.
INSERT INTO flint_meta.cedar_policies(id,name,policy_text,enabled)
VALUES ('a2ui-rls-mutations-v1','A2UI mutations governed by forced RLS',$policy$
permit (
  principal,
  action in [Action::"insert", Action::"update", Action::"delete"],
  resource
) when {
  [
    Resource::"flint_a2ui.applications", Resource::"flint_a2ui.roles",
    Resource::"flint_a2ui.role_assignments", Resource::"flint_a2ui.components",
    Resource::"flint_a2ui.design_systems", Resource::"flint_a2ui.assembly_rules",
    Resource::"flint_a2ui.schemas", Resource::"flint_a2ui.bindings",
    Resource::"flint_a2ui.embeddings", Resource::"flint_a2ui.component_overrides",
    Resource::"flint_a2ui.events"
  ].contains(resource)
};
$policy$,true)
ON CONFLICT (id) DO NOTHING;
