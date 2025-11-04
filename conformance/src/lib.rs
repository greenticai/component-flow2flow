use std::collections::HashSet;

use flow2flow_runtime::{ExecutionOutcome, FlowRuntime};

/// Simple idempotency check for a flow runtime.
pub fn is_idempotent(runtime: &FlowRuntime, payload: &str) -> bool {
    runtime.execute(payload) == runtime.execute(payload)
}

/// Verify that all steps in the execution trace are unique.
pub fn has_unique_trace(outcome: &ExecutionOutcome) -> bool {
    let mut seen = HashSet::new();
    outcome.trace.iter().all(|entry| seen.insert(entry.clone()))
}

#[cfg(test)]
mod support {
    use flow2flow_contract::{CallMode, CallSpec, ParamKind, Scope, ValidatorSpec};
    use flow2flow_router_adapter::{FlowScopeRef, FlowSignature, InMemoryRegistry, RouterAdapter};
    use flow2flow_runtime::{
        CallRequest, Ctx, IdempotencyStore, IdempotencyStoreError, Meta, Resolver, ResolverError,
        ResolverResponse,
    };
    use semver::Version;
    use serde_json::{json, Value};
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Mutex;

    pub fn test_scope(tenant: &str, team: Option<&str>, user: Option<&str>) -> Scope {
        Scope { tenant: tenant.into(), team: team.map(|s| s.into()), user: user.map(|s| s.into()) }
    }

    pub fn signature(flow_id: &str, allow: &[&str], major: u64) -> FlowSignature {
        let mut params = BTreeMap::new();
        params.insert(
            "location".to_string(),
            ValidatorSpec {
                kind: ParamKind::String,
                required: true,
                description: Some("Location identifier".into()),
                default: None,
            },
        );

        let mut returns = BTreeMap::new();
        returns.insert(
            "payload".to_string(),
            ValidatorSpec {
                kind: ParamKind::Object,
                required: true,
                description: Some("Response payload".into()),
                default: None,
            },
        );

        FlowSignature {
            flow_id: flow_id.into(),
            version: Version::new(major, 0, 0),
            intent: format!("{flow_id}.in"),
            allow: allow.iter().map(|s| s.to_string()).collect(),
            params,
            returns,
        }
    }

    pub fn register_signature(
        adapter: &RouterAdapter<InMemoryRegistry>,
        scope: &Scope,
        signature: &FlowSignature,
    ) -> String {
        adapter
            .register_flow_signature(
                FlowScopeRef::Scoped(scope),
                signature.clone(),
                json!(signature.returns),
            )
            .expect("registration")
    }

    pub fn new_ctx(scope: Scope, params: Value) -> Ctx {
        let meta = Meta::new(scope.clone(), None, "corr-test");
        let mut ctx = Ctx::new(meta).with_params(params);
        ctx.add_permission(scope.tenant.clone());
        if let Some(team) = &scope.team {
            ctx.add_permission(format!("{}:{}", scope.tenant, team));
        }
        if let Some(user) = &scope.user {
            ctx.add_permission(format!("{}::{}", scope.tenant, user));
        }
        ctx
    }

    pub fn simple_call_spec(target: &str) -> CallSpec {
        CallSpec {
            target: target.into(),
            mode: CallMode::Sync,
            timeout_ms: None,
            retry: None,
            params_map: BTreeMap::new(),
            result_map: BTreeMap::new(),
            on_error: None,
            join: None,
            scope: None,
        }
    }

    #[derive(Default)]
    pub struct MemoryStore {
        inner: Mutex<HashMap<String, Value>>,
    }

    impl IdempotencyStore for MemoryStore {
        fn load(&self, key: &str) -> Result<Option<Value>, IdempotencyStoreError> {
            Ok(self.inner.lock().unwrap().get(key).cloned())
        }

        fn store(&self, key: &str, value: &Value) -> Result<(), IdempotencyStoreError> {
            self.inner.lock().unwrap().insert(key.to_string(), value.clone());
            Ok(())
        }
    }

    pub struct CountingResolver {
        calls: Mutex<u32>,
    }

    impl CountingResolver {
        pub fn new() -> Self {
            Self { calls: Mutex::new(0) }
        }

        pub fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    impl Resolver for CountingResolver {
        fn resolve(
            &self,
            _ctx: &Ctx,
            _spec: &flow2flow_contract::CallSpec,
            _request: &CallRequest,
        ) -> Result<ResolverResponse, ResolverError> {
            let mut guard = self.calls.lock().unwrap();
            *guard += 1;
            Ok(ResolverResponse::Sync(json!({ "value": *guard })))
        }
    }

    pub struct RecordingResolver {
        response: Value,
        requests: Mutex<Vec<CallRequest>>,
    }

    impl RecordingResolver {
        pub fn new(response: Value) -> Self {
            Self { response, requests: Mutex::new(Vec::new()) }
        }

        pub fn requests(&self) -> Vec<CallRequest> {
            self.requests.lock().unwrap().clone()
        }
    }

    impl Resolver for RecordingResolver {
        fn resolve(
            &self,
            _ctx: &Ctx,
            spec: &flow2flow_contract::CallSpec,
            request: &CallRequest,
        ) -> Result<ResolverResponse, ResolverError> {
            self.requests.lock().unwrap().push(request.clone());
            if spec.mode == CallMode::Sync {
                Ok(ResolverResponse::Sync(self.response.clone()))
            } else {
                Ok(ResolverResponse::AsyncAck(self.response.clone()))
            }
        }
    }

    pub struct RetryResolver {
        failures_remaining: Mutex<u32>,
        calls: Mutex<u32>,
    }

    impl RetryResolver {
        pub fn new(failures: u32) -> Self {
            Self { failures_remaining: Mutex::new(failures), calls: Mutex::new(0) }
        }

        pub fn calls(&self) -> u32 {
            *self.calls.lock().unwrap()
        }
    }

    impl Resolver for RetryResolver {
        fn resolve(
            &self,
            _ctx: &Ctx,
            _spec: &flow2flow_contract::CallSpec,
            _request: &CallRequest,
        ) -> Result<ResolverResponse, ResolverError> {
            *self.calls.lock().unwrap() += 1;
            let mut remaining = self.failures_remaining.lock().unwrap();
            if *remaining > 0 {
                *remaining -= 1;
                Err(ResolverError::timeout("transient"))
            } else {
                Ok(ResolverResponse::Sync(json!({ "payload": {"ok": true} })))
            }
        }
    }

    pub struct FallbackResolver {
        pub calls: Mutex<u32>,
        fallback_requests: Mutex<Vec<CallRequest>>,
    }

    impl FallbackResolver {
        pub fn new() -> Self {
            Self { calls: Mutex::new(0), fallback_requests: Mutex::new(Vec::new()) }
        }

        pub fn fallback_requests(&self) -> Vec<CallRequest> {
            self.fallback_requests.lock().unwrap().clone()
        }
    }

    impl Resolver for FallbackResolver {
        fn resolve(
            &self,
            _ctx: &Ctx,
            _spec: &flow2flow_contract::CallSpec,
            _request: &CallRequest,
        ) -> Result<ResolverResponse, ResolverError> {
            *self.calls.lock().unwrap() += 1;
            Err(ResolverError::timeout("downstream timeout"))
        }

        fn resolve_fallback(
            &self,
            _ctx: &Ctx,
            _spec: &flow2flow_contract::CallSpec,
            fallback: &flow2flow_runtime::FallbackContext<'_>,
        ) -> Result<ResolverResponse, ResolverError> {
            self.fallback_requests.lock().unwrap().push(fallback.request.clone());
            Ok(ResolverResponse::Sync(json!({ "payload": { "fallback": true } })))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow2flow_contract::{CallMode, CallSpec, OnErrorSpec, RetrySpec};
    use flow2flow_router_adapter::{can_call, FlowScopeRef, InMemoryRegistry, RouterAdapter};
    use flow2flow_runtime::{exec_call, runtime_from_steps, TemplateEngine};
    use serde_json::json;
    use std::collections::BTreeMap;

    use crate::support::*;

    #[test]
    fn legacy_helpers_still_work() {
        let runtime =
            runtime_from_steps("faq", 1, &[("lookup", "Fetch"), ("render", "Render")]).unwrap();
        assert!(is_idempotent(&runtime, "payload"));
        let outcome = runtime.execute("payload");
        assert!(has_unique_trace(&outcome));
    }

    #[test]
    fn scope_resolution_precedence() {
        let adapter = RouterAdapter::new(InMemoryRegistry::new());

        let global_sig = signature("assistant.weather.daily", &["*"], 1);
        adapter
            .register_flow_signature(
                FlowScopeRef::Global,
                global_sig.clone(),
                json!(global_sig.returns),
            )
            .unwrap();

        let tenant_scope = test_scope("acme", None, None);
        let tenant_sig = signature("assistant.weather.daily", &["acme"], 1);
        register_signature(&adapter, &tenant_scope, &tenant_sig);

        let team_scope = test_scope("acme", Some("sales"), None);
        let team_sig = signature("assistant.weather.daily", &["acme:sales"], 1);
        register_signature(&adapter, &team_scope, &team_sig);

        let user_scope = test_scope("acme", Some("sales"), Some("alice"));
        let user_sig = signature("assistant.weather.daily", &["acme:sales:alice"], 1);
        register_signature(&adapter, &user_scope, &user_sig);

        let resolved_team = adapter
            .resolve_signature("assistant.weather.daily", None, &team_scope)
            .unwrap()
            .unwrap();
        assert_eq!(resolved_team.path, "/tenants/acme/teams/sales/flows/assistant.weather.daily@1");

        let resolved_user = adapter
            .resolve_signature("assistant.weather.daily", None, &user_scope)
            .unwrap()
            .unwrap();
        assert_eq!(
            resolved_user.path,
            "/tenants/acme/teams/sales/users/alice/flows/assistant.weather.daily@1"
        );
        assert_eq!(resolved_user.signature.allow, user_sig.allow);
    }

    #[test]
    fn acl_allow_and_deny_matrix() {
        let tenant_scope = test_scope("acme", None, None);
        let team_scope = test_scope("acme", Some("ops"), None);
        let user_scope = test_scope("acme", Some("ops"), Some("bob"));
        let other_scope = test_scope("other", Some("ops"), None);

        assert!(can_call("caller", &["*".into()], &tenant_scope));
        assert!(can_call("caller", &["acme".into()], &team_scope));
        assert!(can_call("caller", &["acme:ops".into()], &team_scope));
        assert!(can_call("caller", &["acme:ops:bob".into()], &user_scope));

        assert!(!can_call("caller", &["acme:ops:bob".into()], &team_scope));
        assert!(!can_call("caller", &["acme".into()], &other_scope));
    }

    #[test]
    fn idempotency_uses_store_cache() {
        let scope = test_scope("acme", Some("ops"), None);
        let mut ctx = new_ctx(scope.clone(), json!({"location": "NYC"}));
        ctx.idempotency_key = Some("key-1".into());

        let call_spec = simple_call_spec("component.weather");
        let resolver = CountingResolver::new();
        let store = MemoryStore::default();
        let templates = TemplateEngine::default();

        let outcome = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx,
            &call_spec,
            &resolver,
            Some(&store),
            &templates,
        )
        .unwrap();
        assert_eq!(resolver.calls(), 1);
        assert_eq!(outcome.value["value"], json!(1));
        assert!(!outcome.idempotent_replay);

        let outcome_cached = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx,
            &call_spec,
            &resolver,
            Some(&store),
            &templates,
        )
        .unwrap();
        assert_eq!(resolver.calls(), 1, "resolver not invoked on replay");
        assert!(outcome_cached.idempotent_replay);

        ctx.idempotency_key = Some("key-2".into());
        let outcome_new = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx,
            &call_spec,
            &resolver,
            Some(&store),
            &templates,
        )
        .unwrap();
        assert_eq!(resolver.calls(), 2);
        assert_eq!(outcome_new.value["value"], json!(2));
    }

    #[test]
    fn params_and_result_mapping_apply_defaults() {
        let scope = test_scope("acme", None, None);
        let mut ctx = new_ctx(scope, json!({"location": "Seattle"}));
        ctx.ensure_state_object();

        let mut params_map = BTreeMap::new();
        params_map.insert("payload.city".into(), "{{ params.location }}".into());
        params_map.insert("payload.country".into(), "{{ params.country | default('US') }}".into());

        let mut result_map = BTreeMap::new();
        result_map.insert(
            "weather.summary".into(),
            "{{ result.payload.summary | default('n/a') }}".into(),
        );
        result_map.insert("weather.temp_c".into(), "{{ result.payload.temp_c }}".into());

        let call_spec = CallSpec {
            target: "component.weather".into(),
            mode: CallMode::Sync,
            timeout_ms: None,
            retry: None,
            params_map,
            result_map,
            on_error: None,
            join: None,
            scope: None,
        };

        let resolver = RecordingResolver::new(json!({
            "payload": {
                "summary": "Sunny",
                "temp_c": 21
            }
        }));

        let templates = TemplateEngine::default();
        let outcome = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx,
            &call_spec,
            &resolver,
            None::<&MemoryStore>,
            &templates,
        )
        .unwrap();
        assert_eq!(outcome.value["payload"]["summary"], json!("Sunny"));

        let reqs = resolver.requests();
        assert_eq!(reqs.len(), 1);
        assert_eq!(reqs[0].params["payload"]["city"], json!("Seattle"));
        assert_eq!(reqs[0].params["payload"]["country"], json!("US"));

        assert_eq!(ctx.get_from_state("weather.summary").unwrap(), &json!("Sunny"));
        assert_eq!(ctx.get_from_state("weather.temp_c").unwrap(), &json!(21));
    }

    #[test]
    fn retry_and_fallback_behaviour() {
        let scope = test_scope("acme", None, None);
        let mut ctx = new_ctx(scope.clone(), json!({"location": "Rome"}));

        let call_spec_retry = CallSpec {
            target: "component.weather".into(),
            mode: CallMode::Sync,
            timeout_ms: None,
            retry: Some(RetrySpec { attempts: 3, delay_ms: Some(0), max_delay_ms: Some(0) }),
            params_map: BTreeMap::new(),
            result_map: BTreeMap::new(),
            on_error: None,
            join: None,
            scope: None,
        };

        let retry_resolver = RetryResolver::new(2);
        let templates = TemplateEngine::default();

        let outcome = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx,
            &call_spec_retry,
            &retry_resolver,
            None::<&MemoryStore>,
            &templates,
        )
        .unwrap();
        assert_eq!(retry_resolver.calls(), 3);
        assert_eq!(outcome.attempts, 3);
        assert!(!outcome.fallback_used);

        let mut ctx_fb = new_ctx(scope, json!({"location": "Berlin"}));
        let call_spec_fb = CallSpec {
            target: "component.weather".into(),
            mode: CallMode::Sync,
            timeout_ms: None,
            retry: Some(RetrySpec { attempts: 1, delay_ms: Some(0), max_delay_ms: Some(0) }),
            params_map: BTreeMap::new(),
            result_map: BTreeMap::new(),
            on_error: Some(OnErrorSpec {
                route: "flows.fallback".into(),
                mapper: Some("{\"reason\": \"{{ error.kind }}\"}".into()),
            }),
            join: None,
            scope: None,
        };

        let fb_resolver = FallbackResolver::new();
        let outcome_fb = exec_call(
            "assistant.weather.daily",
            "call",
            &mut ctx_fb,
            &call_spec_fb,
            &fb_resolver,
            None::<&MemoryStore>,
            &templates,
        )
        .unwrap();
        assert!(outcome_fb.fallback_used);
        let fallback_requests = fb_resolver.fallback_requests();
        assert_eq!(fallback_requests.len(), 1);
        assert_eq!(fallback_requests[0].params["reason"], json!("Timeout"));
    }
}
