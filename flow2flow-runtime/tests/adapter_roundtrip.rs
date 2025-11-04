use std::collections::BTreeMap;

use flow2flow_contract::{CallMode, CallSpec, ParamKind, Scope, ValidatorSpec};
use flow2flow_router_adapter::{
    can_call, FlowScopeRef, FlowSignature, InMemoryRegistry, RouterAdapter,
};
use flow2flow_runtime::{
    exec_call, Ctx, IdempotencyStore, IdempotencyStoreError, Meta, TemplateEngine,
};
use semver::Version;
use serde_json::json;

fn scope(tenant: &str, team: Option<&str>) -> Scope {
    Scope { tenant: tenant.into(), team: team.map(|s| s.into()), user: None }
}

fn signature(flow_id: &str, allow: Vec<String>) -> FlowSignature {
    FlowSignature {
        flow_id: flow_id.into(),
        version: Version::new(1, 0, 0),
        intent: format!("{flow_id}.in"),
        allow,
        params: BTreeMap::from([(
            "location".into(),
            ValidatorSpec {
                kind: ParamKind::String,
                required: true,
                description: Some("Location".into()),
                default: None,
            },
        )]),
        returns: BTreeMap::from([(
            "payload".into(),
            ValidatorSpec {
                kind: ParamKind::Object,
                required: true,
                description: Some("Weather payload".into()),
                default: None,
            },
        )]),
    }
}

struct NoopStore;

impl IdempotencyStore for NoopStore {
    fn load(&self, _key: &str) -> Result<Option<serde_json::Value>, IdempotencyStoreError> {
        Ok(None)
    }

    fn store(&self, _key: &str, _value: &serde_json::Value) -> Result<(), IdempotencyStoreError> {
        Ok(())
    }
}

#[test]
fn router_adapter_resolves_scoped_signature() {
    let registry = InMemoryRegistry::new();
    let adapter = RouterAdapter::new(registry);

    let tenant_scope = scope("acme", None);
    let team_scope = scope("acme", Some("sales-na"));

    let tenant_signature = signature("assistant.weather.daily", vec!["acme".into()]);
    let team_signature = signature("assistant.weather.daily", vec!["acme:sales-na".into()]);

    adapter
        .register_flow_signature(
            FlowScopeRef::Scoped(&tenant_scope),
            tenant_signature.clone(),
            json!(tenant_signature.returns),
        )
        .expect("tenant registration");

    adapter
        .register_flow_signature(
            FlowScopeRef::Scoped(&team_scope),
            team_signature.clone(),
            json!(team_signature.returns),
        )
        .expect("team registration");

    let resolved = adapter
        .resolve_signature("assistant.weather.daily", None, &team_scope)
        .expect("resolve")
        .expect("signature present");

    assert_eq!(resolved.signature.intent, format!("{}.in", "assistant.weather.daily"));
    assert_eq!(resolved.signature.allow, team_signature.allow);
    assert!(resolved.path.contains("sales-na"));

    assert!(can_call("assistant.shell", &team_signature.allow, &team_scope));

    let tenant_only_scope = tenant_scope.clone();
    assert!(can_call("assistant.shell", &tenant_signature.allow, &tenant_only_scope));
    assert!(!can_call("assistant.shell", &team_signature.allow, &tenant_only_scope));

    // Smoke check: exec_call can call into resolver and obtain metadata payload.
    let meta = Meta::new(team_scope.clone(), None, "corr-123");
    let mut ctx = Ctx::new(meta).with_params(json!({ "location": "NYC" }));

    let call_spec = CallSpec {
        target: "assistant.weather.daily".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: None,
        scope: None,
    };

    let outcome = exec_call(
        "assistant.shell",
        "resolve",
        &mut ctx,
        &call_spec,
        &adapter,
        None::<&NoopStore>,
        &TemplateEngine::default(),
    )
    .expect("exec_call");

    assert_eq!(outcome.value["flow_id"], json!("assistant.weather.daily"));
    assert_eq!(outcome.value["intent"], json!("assistant.weather.daily.in"));
}
