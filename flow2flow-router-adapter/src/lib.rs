mod error;
mod registry;
mod scope;
mod signature;

use flow2flow_contract::{CallSpec, FlowSpec, Scope};
use flow2flow_runtime::{CallRequest, Resolver, ResolverError, ResolverResponse};
use semver::{Version, VersionReq};
use serde_json::{json, Value};

pub use error::AdapterError;
pub use registry::{InMemoryRegistry, Registry, VersionedRecord};
pub use scope::{fallback_order, ScopeKey};
pub use signature::{signature_from_spec, FlowRecord, FlowSignature};

#[derive(Debug, Clone)]
pub enum FlowScopeRef<'a> {
    Global,
    Scoped(&'a Scope),
}

#[derive(Debug, Clone)]
pub struct ResolvedFlow {
    pub signature: FlowSignature,
    pub returns_schema: Value,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct RouterAdapter<R: Registry> {
    registry: R,
}

impl<R: Registry> RouterAdapter<R> {
    pub fn new(registry: R) -> Self {
        Self { registry }
    }

    pub fn registry(&self) -> &R {
        &self.registry
    }

    pub fn register_flow_spec(
        &self,
        scope: FlowScopeRef<'_>,
        spec: &FlowSpec,
    ) -> Result<String, AdapterError> {
        let (signature, returns_schema) = signature_from_spec(spec)?;
        self.register_flow_signature(scope, signature, returns_schema)
    }

    pub fn register_flow_signature(
        &self,
        scope: FlowScopeRef<'_>,
        signature: FlowSignature,
        returns_schema: Value,
    ) -> Result<String, AdapterError> {
        let scope_key = match scope {
            FlowScopeRef::Global => ScopeKey::global(),
            FlowScopeRef::Scoped(scope) => ScopeKey::from_scope(scope),
        };

        let path =
            format!("{}/{}", scope_key.path_prefix(), signature.registration_path_fragment());

        let record =
            FlowRecord { signature: signature.clone(), returns_schema, path: path.clone() };

        self.registry.register(&scope_key, record)?;
        Ok(path)
    }

    pub fn resolve_signature(
        &self,
        flow_id: &str,
        version: Option<&str>,
        caller_scope: &Scope,
    ) -> Result<Option<ResolvedFlow>, AdapterError> {
        let version_req = match version {
            Some(text) => Some(parse_version_req(text)?),
            None => None,
        };

        let resolved = self.registry.resolve(flow_id, version_req.as_ref(), caller_scope)?;

        Ok(resolved.map(|entry| {
            let FlowRecord { signature, returns_schema, path } = entry.record;
            ResolvedFlow { signature, returns_schema, path }
        }))
    }
}

impl<R: Registry> Resolver for RouterAdapter<R> {
    fn resolve(
        &self,
        ctx: &flow2flow_runtime::Ctx,
        spec: &CallSpec,
        _request: &CallRequest,
    ) -> Result<ResolverResponse, ResolverError> {
        let (flow_id, version_hint) = split_target(&spec.target);
        let scope = spec.scope.as_ref().unwrap_or(&ctx.meta.scope);
        let resolved = self
            .resolve_signature(&flow_id, version_hint.as_deref(), scope)
            .map_err(|err| ResolverError::fatal(err.to_string()))?;

        match resolved {
            Some(flow) => {
                let payload = json!({
                    "flow_id": flow.signature.flow_id,
                    "version": flow.signature.version.to_string(),
                    "intent": flow.signature.intent,
                    "path": flow.path,
                    "allow": flow.signature.allow,
                });
                Ok(ResolverResponse::Sync(payload))
            }
            None => Err(ResolverError::fatal(format!(
                "no flow `{flow_id}` found for scope {}",
                scope.tenant
            ))),
        }
    }
}

pub fn can_call(_caller_flow_id: &str, allow: &[String], caller_scope: &Scope) -> bool {
    if allow.is_empty() {
        return true;
    }
    let mut candidates = Vec::new();
    candidates.push("*".to_string());
    candidates.push(caller_scope.tenant.clone());
    if let Some(team) = &caller_scope.team {
        candidates.push(format!("{}:{}", caller_scope.tenant, team));
    }
    if let Some(user) = &caller_scope.user {
        candidates.push(format!("{}::{}", caller_scope.tenant, user));
    }
    if let (Some(team), Some(user)) = (&caller_scope.team, &caller_scope.user) {
        candidates.push(format!("{}:{}:{}", caller_scope.tenant, team, user));
    }

    allow
        .iter()
        .any(|pattern| candidates.iter().any(|candidate| matches_pattern(pattern, candidate)))
}

fn split_target(target: &str) -> (String, Option<String>) {
    let mut parts = target.split('@');
    let flow = parts.next().unwrap_or("").to_string();
    let version = parts.next().map(|s| s.to_string());
    (flow, version)
}

fn parse_version_req(requirement: &str) -> Result<VersionReq, AdapterError> {
    if let Ok(req) = VersionReq::parse(requirement) {
        return Ok(req);
    }
    if let Ok(version) = Version::parse(requirement) {
        return VersionReq::parse(&format!("={}", version)).map_err(|err| {
            AdapterError::VersionReq { requirement: requirement.to_string(), source: err }
        });
    }
    if requirement.chars().all(|c| c.is_ascii_digit()) {
        let formatted = format!("^{}.0.0", requirement);
        return VersionReq::parse(&formatted).map_err(|err| AdapterError::VersionReq {
            requirement: requirement.to_string(),
            source: err,
        });
    }
    Err(AdapterError::VersionReq {
        requirement: requirement.to_string(),
        source: VersionReq::parse(requirement).unwrap_err(),
    })
}

fn matches_pattern(pattern: &str, candidate: &str) -> bool {
    if pattern.is_empty() {
        return candidate.is_empty();
    }
    if pattern == "*" {
        return true;
    }
    if let Some(idx) = pattern.find('*') {
        let prefix = &pattern[..idx];
        let rest = &pattern[idx + 1..];
        if prefix.is_empty() {
            for split in 0..=candidate.len() {
                if matches_pattern(rest, &candidate[split..]) {
                    return true;
                }
            }
            false
        } else if let Some(stripped) = candidate.strip_prefix(prefix) {
            matches_pattern(rest, stripped)
        } else {
            false
        }
    } else {
        pattern == candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow2flow_contract::Scope;
    use semver::Version;
    use std::collections::BTreeMap;

    fn scope(tenant: &str, team: Option<&str>, user: Option<&str>) -> Scope {
        Scope { tenant: tenant.into(), team: team.map(|s| s.into()), user: user.map(|s| s.into()) }
    }

    fn sample_signature(flow_id: &str, version: &str) -> FlowSignature {
        FlowSignature {
            flow_id: flow_id.into(),
            version: Version::parse(version).unwrap(),
            intent: format!("{flow_id}.in"),
            allow: vec![],
            params: BTreeMap::new(),
            returns: BTreeMap::new(),
        }
    }

    #[test]
    fn registration_round_trip() {
        let registry = InMemoryRegistry::new();
        let adapter = RouterAdapter::new(registry.clone());

        let signature = sample_signature("flow.weather", "1.0.0");
        let path = adapter
            .register_flow_signature(
                FlowScopeRef::Scoped(&scope("acme", Some("team"), None)),
                signature.clone(),
                json!({}),
            )
            .expect("register");

        assert_eq!(path, "/tenants/acme/teams/team/flows/flow.weather@1");
        let versions = registry.versions_for(
            &ScopeKey::with_parts(Some("acme".into()), Some("team".into()), None),
            "flow.weather",
        );
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].version, Version::parse("1.0.0").unwrap());
    }

    #[test]
    fn scoped_resolution_precedence() {
        let registry = InMemoryRegistry::new();
        let adapter = RouterAdapter::new(registry.clone());

        adapter
            .register_flow_signature(
                FlowScopeRef::Global,
                sample_signature("flow.weather", "1.0.0"),
                json!({}),
            )
            .unwrap();
        adapter
            .register_flow_signature(
                FlowScopeRef::Scoped(&scope("acme", None, None)),
                sample_signature("flow.weather", "1.1.0"),
                json!({}),
            )
            .unwrap();
        adapter
            .register_flow_signature(
                FlowScopeRef::Scoped(&scope("acme", Some("team"), None)),
                sample_signature("flow.weather", "1.2.0"),
                json!({}),
            )
            .unwrap();

        let resolved = adapter
            .resolve_signature("flow.weather", None, &scope("acme", Some("team"), Some("user")))
            .unwrap()
            .unwrap();

        assert_eq!(resolved.signature.version, Version::parse("1.2.0").unwrap());
    }

    #[test]
    fn version_resolution_exact_and_major() {
        let registry = InMemoryRegistry::new();
        let adapter = RouterAdapter::new(registry.clone());

        adapter
            .register_flow_signature(
                FlowScopeRef::Global,
                sample_signature("flow.weather", "1.0.0"),
                json!({}),
            )
            .unwrap();
        adapter
            .register_flow_signature(
                FlowScopeRef::Global,
                sample_signature("flow.weather", "1.3.0"),
                json!({}),
            )
            .unwrap();
        adapter
            .register_flow_signature(
                FlowScopeRef::Global,
                sample_signature("flow.weather", "2.1.0"),
                json!({}),
            )
            .unwrap();

        let req_major = adapter
            .resolve_signature("flow.weather", Some("1"), &scope("acme", None, None))
            .unwrap()
            .unwrap();
        assert_eq!(req_major.signature.version, Version::parse("1.3.0").unwrap());

        let req_exact = adapter
            .resolve_signature("flow.weather", Some("=2.1.0"), &scope("acme", None, None))
            .unwrap()
            .unwrap();
        assert_eq!(req_exact.signature.version, Version::parse("2.1.0").unwrap());

        let req_latest = adapter
            .resolve_signature("flow.weather", None, &scope("acme", None, None))
            .unwrap()
            .unwrap();
        assert_eq!(req_latest.signature.version, Version::parse("2.1.0").unwrap());
    }

    #[test]
    fn can_call_respects_allow() {
        let scope = scope("acme", Some("team"), Some("user"));
        assert!(can_call("caller", &["*".into()], &scope));
        assert!(can_call("caller", &["acme:team".into()], &scope));
        assert!(!can_call("caller", &["other".into()], &scope));
    }
}
