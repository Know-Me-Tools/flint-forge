//! Opt-in Cedar authorization for IPFS console private file records.
use cedar_policy::{Context, Expression, RestrictedExpression};
use forge_identity::RlsContext;

/// Render one static policy for an explicitly authorized subject and tenant.
/// Cedar formats both inputs as string literals before substitution. The
/// caller owns approval, opaque policy IDs, persistence and revocation.
#[must_use]
pub fn record_grant(subject: &str, tenant: &str) -> Option<String> {
    if subject.is_empty() || tenant.is_empty() {
        return None;
    }
    Some(format!(
        include_str!("../policies/console-record-v1.cedar.in"),
        subject = Expression::new_string(subject.to_owned()),
        tenant = Expression::new_string(tenant.to_owned()),
    ))
}

/// The privileged `consoleTenant` attribute is only supplied by the dedicated
/// file-record PEP. Generic request context, headers and bodies never feed it.
pub(crate) fn record_context(who: &RlsContext) -> Context {
    let Some(tenant) = console_tenant(who) else {
        return Context::empty();
    };
    Context::from_pairs([(
        "consoleTenant".to_owned(),
        RestrictedExpression::new_string(tenant),
    )])
    .unwrap_or_else(|_| Context::empty())
}

fn console_tenant(who: &RlsContext) -> Option<String> {
    let claims = who.claims()?;
    let extra = &claims.extra;
    if who.role != "authenticated"
        || claims.role.as_deref() != Some("authenticated")
        || claims.sub.is_empty()
        || extra.get("iss")?.as_str()? != "https://gate.know-me.tools"
        || extra.get("aud")?.as_str()? != "flint-core"
        || extra.get("flint_kind")?.as_str()? != "agent"
        || extra.get("act")?.get("sub")?.as_str()? != "ipfs-console-backend"
    {
        return None;
    }
    claims.tenant_id.filter(|tenant| !tenant.is_empty())
}
