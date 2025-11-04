use std::collections::{BTreeMap, HashMap, VecDeque};
use std::sync::Mutex;

use flow2flow_contract::{
    CallMode, CallSpec, InSpec, JoinSpec, JoinStrategy, OnErrorSpec, OutSpec, ParamKind, RetrySpec,
    Scope, ValidatorSpec,
};
use flow2flow_runtime::{
    exec_call, exec_in, exec_out, Ctx, ExecError, IdempotencyStore, IdempotencyStoreError, Meta,
    Resolver, ResolverError, ResolverResponse, TemplateEngine,
};
use serde_json::{json, Value};

struct MemoryStore {
    inner: Mutex<HashMap<String, Value>>,
}

impl MemoryStore {
    fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    fn insert(&self, key: &str, value: Value) {
        self.inner.lock().unwrap().insert(key.to_string(), value);
    }
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

#[derive(Clone)]
enum Action {
    Response(ResolverResponse),
    Error(ResolverError),
}

struct ScriptedResolver {
    sequence: Mutex<VecDeque<Action>>,
    fallback_sequence: Mutex<VecDeque<Action>>,
    calls: Mutex<Vec<flow2flow_runtime::CallRequest>>,
    fallback_calls: Mutex<Vec<flow2flow_runtime::CallRequest>>,
}

impl ScriptedResolver {
    fn new(actions: Vec<Action>) -> Self {
        Self {
            sequence: Mutex::new(actions.into_iter().collect()),
            fallback_sequence: Mutex::new(VecDeque::new()),
            calls: Mutex::new(Vec::new()),
            fallback_calls: Mutex::new(Vec::new()),
        }
    }

    fn with_fallback(self, actions: Vec<Action>) -> Self {
        *self.fallback_sequence.lock().unwrap() = actions.into_iter().collect();
        self
    }

    fn take_requests(&self) -> Vec<flow2flow_runtime::CallRequest> {
        self.calls.lock().unwrap().drain(..).collect()
    }

    fn take_fallback_requests(&self) -> Vec<flow2flow_runtime::CallRequest> {
        self.fallback_calls.lock().unwrap().drain(..).collect()
    }
}

impl Resolver for ScriptedResolver {
    fn resolve(
        &self,
        _ctx: &Ctx,
        _spec: &CallSpec,
        request: &flow2flow_runtime::CallRequest,
    ) -> Result<ResolverResponse, ResolverError> {
        self.calls.lock().unwrap().push(request.clone());
        match self.sequence.lock().unwrap().pop_front() {
            Some(Action::Response(resp)) => Ok(resp),
            Some(Action::Error(err)) => Err(err),
            None => panic!("no scripted action remaining"),
        }
    }

    fn resolve_fallback(
        &self,
        _ctx: &Ctx,
        _spec: &CallSpec,
        fallback: &flow2flow_runtime::FallbackContext<'_>,
    ) -> Result<ResolverResponse, ResolverError> {
        self.fallback_calls.lock().unwrap().push(fallback.request.clone());
        match self.fallback_sequence.lock().unwrap().pop_front() {
            Some(Action::Response(resp)) => Ok(resp),
            Some(Action::Error(err)) => Err(err),
            None => panic!("no fallback action scripted"),
        }
    }
}

fn scope() -> Scope {
    Scope { tenant: "tenant".into(), team: Some("team".into()), user: Some("user".into()) }
}

fn meta() -> Meta {
    Meta::new(scope(), Some("channel".into()), "corr-123")
}

fn template_engine() -> TemplateEngine {
    TemplateEngine::new()
}

fn validator(kind: ParamKind, required: bool) -> ValidatorSpec {
    ValidatorSpec { kind, required, description: None, default: None }
}

#[test]
fn exec_in_validates_and_applies_defaults() {
    let mut ctx = Ctx::new(meta())
        .with_params(json!({ "subject": "weather", "optional": 2 }))
        .with_permissions(vec!["flows.read".into()]);

    let spec = InSpec {
        intent: "flow.intent".into(),
        params: BTreeMap::from([
            ("subject".into(), validator(ParamKind::String, true)),
            (
                "optional".into(),
                ValidatorSpec {
                    kind: ParamKind::Integer,
                    required: false,
                    description: None,
                    default: Some(json!(7)),
                },
            ),
        ]),
        router: None,
        visibility: vec![],
        allow: vec!["flows.read".into()],
    };

    exec_in("flow.intent", "n01", &mut ctx, &spec).expect("exec_in ok");
    assert_eq!(ctx.params["subject"], json!("weather"));
    assert_eq!(ctx.params["optional"], json!(2));
}

#[test]
fn exec_in_rejects_missing_permission() {
    let mut ctx = Ctx::new(meta()).with_params(json!({ "subject": "weather" }));
    let spec = InSpec {
        intent: "flow.intent".into(),
        params: BTreeMap::from([("subject".into(), validator(ParamKind::String, true))]),
        router: None,
        visibility: vec![],
        allow: vec!["flows.read".into()],
    };

    let err = exec_in("flow.intent", "n01", &mut ctx, &spec).unwrap_err();
    assert!(matches!(err, ExecError::PermissionDenied { .. }));
}

#[test]
fn exec_call_sync_success() {
    let resolver = ScriptedResolver::new(vec![Action::Response(ResolverResponse::Sync(json!({
        "temp": 20
    })))]);
    let mut ctx =
        Ctx::new(meta()).with_params(json!({ "subject": "weather" })).with_state(Value::Null);
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::from([("payload.subject".into(), "{{ params.subject }}".into())]),
        result_map: BTreeMap::new(),
        on_error: None,
        join: None,
        scope: None,
    };

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("call success");

    assert_eq!(outcome.value, json!({"temp": 20}));
    let requests = resolver.take_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].params["payload"]["subject"], json!("weather"));
    assert_eq!(ctx.state["result"]["temp"], json!(20));
}

#[test]
fn exec_call_retries_then_succeeds() {
    let resolver = ScriptedResolver::new(vec![
        Action::Error(ResolverError::timeout("timeout")),
        Action::Response(ResolverResponse::Sync(json!({"ok": true}))),
    ]);
    let mut ctx = Ctx::new(meta()).with_params(json!({"subject": "weather"}));
    let templates = template_engine();
    let spec = CallSpec {
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

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("call success");

    assert_eq!(outcome.attempts, 2);
}

#[test]
fn exec_call_async_ack() {
    let resolver =
        ScriptedResolver::new(vec![Action::Response(ResolverResponse::AsyncAck(json!({
            "ticket": "123"
        })))]);
    let mut ctx = Ctx::new(meta());
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Async,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: None,
        scope: None,
    };

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("call success");

    assert_eq!(outcome.value["ticket"], json!("123"));
}

#[test]
fn exec_call_fanout_join_all() {
    let resolver = ScriptedResolver::new(vec![Action::Response(ResolverResponse::Fanout(vec![
        json!({"node": "a"}),
        json!({"node": "b"}),
    ]))]);
    let mut ctx = Ctx::new(meta());
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: Some(JoinSpec {
            with: vec!["a".into(), "b".into()],
            strategy: Some(JoinStrategy::All),
        }),
        scope: None,
    };

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("call success");

    assert_eq!(outcome.value, json!([{ "node": "a" }, { "node": "b" }]));
    assert!(outcome.fanout.is_some());
}

#[test]
fn exec_call_fanout_join_any() {
    let resolver = ScriptedResolver::new(vec![Action::Response(ResolverResponse::Fanout(vec![
        Value::Null,
        json!({"node": "b"}),
    ]))]);
    let mut ctx = Ctx::new(meta());
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: Some(JoinSpec {
            with: vec!["a".into(), "b".into()],
            strategy: Some(JoinStrategy::Any),
        }),
        scope: None,
    };

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("call success");

    assert_eq!(outcome.value, json!({"node": "b"}));
}

#[test]
fn exec_call_timeout_then_fallback() {
    let resolver = ScriptedResolver::new(vec![Action::Error(ResolverError::timeout("timeout"))])
        .with_fallback(vec![Action::Response(ResolverResponse::Sync(json!({
            "fallback": true
        })))]);
    let mut ctx = Ctx::new(meta());
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: Some(10),
        retry: Some(RetrySpec { attempts: 1, delay_ms: Some(0), max_delay_ms: Some(0) }),
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: Some(OnErrorSpec {
            route: "fallback".into(),
            mapper: Some("{\"reason\": \"timeout\"}".into()),
        }),
        join: None,
        scope: None,
    };

    let outcome = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .expect("fallback success");

    assert!(outcome.fallback_used);
    let fallback_requests = resolver.take_fallback_requests();
    assert_eq!(fallback_requests.len(), 1);
    assert_eq!(fallback_requests[0].params["reason"], json!("timeout"));
}

#[test]
fn exec_call_scope_mismatch() {
    let resolver =
        ScriptedResolver::new(vec![Action::Response(ResolverResponse::Sync(Value::Null))]);
    let mut ctx = Ctx::new(meta());
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: None,
        scope: Some(Scope { tenant: "other".into(), team: None, user: None }),
    };

    let err = exec_call(
        "flow.intent",
        "n02",
        &mut ctx,
        &spec,
        &resolver,
        None::<&MemoryStore>,
        &templates,
    )
    .unwrap_err();

    assert!(matches!(err, ExecError::ScopeMismatch { .. }));
}

#[test]
fn exec_call_idempotent_replay() {
    let store = MemoryStore::new();
    store.insert("key-1", json!({"cached": true}));
    let resolver = ScriptedResolver::new(vec![]);
    let mut ctx =
        Ctx::new(meta()).with_params(Value::Null).with_idempotency_key(Some("key-1".into()));
    let templates = template_engine();
    let spec = CallSpec {
        target: "component.weather".into(),
        mode: CallMode::Sync,
        timeout_ms: None,
        retry: None,
        params_map: BTreeMap::new(),
        result_map: BTreeMap::new(),
        on_error: None,
        join: None,
        scope: None,
    };

    let outcome =
        exec_call("flow.intent", "n02", &mut ctx, &spec, &resolver, Some(&store), &templates)
            .expect("replay success");

    assert!(outcome.idempotent_replay);
    assert_eq!(outcome.value["cached"], json!(true));
}

#[test]
fn exec_out_validates_returns() {
    let mut ctx = Ctx::new(meta());
    ctx.set_state_path("output.status", json!("ok")).unwrap();
    let spec = OutSpec {
        returns: BTreeMap::from([("output.status".into(), validator(ParamKind::String, true))]),
        docs: None,
    };

    let result = exec_out("flow.intent", "n03", &ctx, &spec).expect("out success");
    assert_eq!(result["output.status"], json!("ok"));
}
