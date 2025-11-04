use std::thread;
use std::time::{Duration, Instant};

use flow2flow_contract::{
    CallMode, CallSpec, InSpec, JoinStrategy, OutSpec, ParamKind, Scope, ValidatorSpec,
};
use serde_json::{json, Map, Value};
use tracing::info_span;

use crate::ctx::Ctx;
use crate::error::ExecError;
use crate::idempotency::IdempotencyStore;
use crate::resolver::{CallRequest, FallbackContext, Resolver, ResolverError, ResolverResponse};
use crate::templating::TemplateEngine;

#[derive(Debug, Clone, PartialEq)]
pub struct CallOutcome {
    pub value: Value,
    pub attempts: u32,
    pub mode: CallMode,
    pub fanout: Option<Vec<Value>>,
    pub fallback_used: bool,
    pub idempotent_replay: bool,
}

impl CallOutcome {
    pub fn new(value: Value, attempts: u32, mode: CallMode) -> Self {
        Self { value, attempts, mode, fanout: None, fallback_used: false, idempotent_replay: false }
    }
}

pub fn exec_in(
    flow_id: &str,
    node_name: &str,
    ctx: &mut Ctx,
    spec: &InSpec,
) -> Result<(), ExecError> {
    let span = info_span!(
        "exec_in",
        flow_id,
        node_name,
        node_type = "f2f.in",
        tenant = ctx.meta.scope.tenant.as_str(),
        team = ctx.meta.scope.team.as_deref().unwrap_or(""),
        user = ctx.meta.scope.user.as_deref().unwrap_or(""),
    );

    span.in_scope(|| {
        if !spec.allow.is_empty() {
            let permitted = spec.allow.iter().any(|pattern| ctx.has_permission_pattern(pattern));
            if !permitted {
                return Err(ExecError::PermissionDenied { required: spec.allow.clone() });
            }
        }

        let input = ctx
            .params
            .as_object()
            .ok_or_else(|| ExecError::Validation("inbound payload must be an object".into()))?;

        let mut validated = Map::new();
        for (name, validator) in &spec.params {
            match input.get(name) {
                Some(value) => {
                    validate_value(name, value, validator)?;
                    validated.insert(name.clone(), value.clone());
                }
                None => {
                    if validator.required {
                        return Err(ExecError::Validation(format!(
                            "missing required param `{name}`"
                        )));
                    }
                    if let Some(default) = &validator.default {
                        validated.insert(name.clone(), default.clone());
                    }
                }
            }
        }

        for (name, value) in input {
            validated.entry(name.clone()).or_insert_with(|| value.clone());
        }

        ctx.params = Value::Object(validated);
        Ok(())
    })
}

pub fn exec_call<R, S>(
    flow_id: &str,
    node_name: &str,
    ctx: &mut Ctx,
    spec: &CallSpec,
    resolver: &R,
    idempotency_store: Option<&S>,
    templates: &TemplateEngine,
) -> Result<CallOutcome, ExecError>
where
    R: Resolver,
    S: IdempotencyStore,
{
    let span = info_span!(
        "exec_call",
        flow_id,
        node_name,
        node_type = "f2f.call",
        tenant = ctx.meta.scope.tenant.as_str(),
        team = ctx.meta.scope.team.as_deref().unwrap_or(""),
        user = ctx.meta.scope.user.as_deref().unwrap_or(""),
    );

    span.in_scope(|| {
        if let Some(spec_scope) = &spec.scope {
            ensure_scope_compatible(spec_scope, &ctx.meta.scope)?;
        }

        if let Some(deadline) = ctx.deadline {
            if deadline <= Instant::now() {
                return Err(ExecError::DeadlineExceeded { node: node_name.to_string() });
            }
        }

        if let Some((store, key)) =
            idempotency_store.and_then(|store| ctx.idempotency_key.as_deref().map(|k| (store, k)))
        {
            match store.load(key) {
                Ok(Some(cached)) => {
                    let mut outcome = CallOutcome::new(cached, 0, spec.mode.clone());
                    outcome.idempotent_replay = true;
                    return Ok(outcome);
                }
                Ok(None) => {}
                Err(err) => {
                    return Err(ExecError::Idempotency { key: key.to_string(), source: err });
                }
            }
        }

        let max_attempts = spec.retry.as_ref().map(|r| r.attempts).unwrap_or(1).max(1);
        let mut attempts = 0_u32;
        let mut delay_ms = spec.retry.as_ref().and_then(|r| r.delay_ms).unwrap_or(0);

        loop {
            attempts += 1;

            let request_payload = build_call_payload(ctx, spec, templates)?;
            let timeout = compute_timeout(ctx.deadline, spec.timeout_ms, node_name)?;
            let call_request =
                CallRequest { params: request_payload.clone(), timeout, attempt: attempts };

            match resolver.resolve(ctx, spec, &call_request) {
                Ok(response) => {
                    let mut outcome = handle_call_response(ctx, spec, response, templates)?;
                    outcome.attempts = attempts;
                    outcome.fallback_used = false;

                    if let Some((store, key)) = idempotency_store
                        .and_then(|store| ctx.idempotency_key.as_deref().map(|k| (store, k)))
                    {
                        if let Err(err) = store.store(key, &outcome.value) {
                            return Err(ExecError::Idempotency {
                                key: key.to_string(),
                                source: err,
                            });
                        }
                    }

                    return Ok(outcome);
                }
                Err(err) => {
                    let retryable = err.is_retryable();

                    if retryable && attempts < max_attempts {
                        let sleep_ms = delay_ms;
                        if sleep_ms > 0 {
                            thread::sleep(Duration::from_millis(sleep_ms));
                        }
                        delay_ms = compute_next_delay(
                            delay_ms,
                            spec.retry.as_ref().and_then(|r| r.max_delay_ms),
                        );
                        continue;
                    }

                    if let Some(on_error) = &spec.on_error {
                        let fallback_request = build_fallback_request(
                            ctx,
                            templates,
                            on_error,
                            &request_payload,
                            &err,
                        )?;
                        let fallback_context =
                            FallbackContext { route: &on_error.route, request: &fallback_request };
                        match resolver.resolve_fallback(ctx, spec, &fallback_context) {
                            Ok(response) => {
                                let mut outcome =
                                    handle_call_response(ctx, spec, response, templates)?;
                                outcome.attempts = attempts;
                                outcome.fallback_used = true;
                                if let Some((store, key)) = idempotency_store.and_then(|store| {
                                    ctx.idempotency_key.as_deref().map(|k| (store, k))
                                }) {
                                    if let Err(err) = store.store(key, &outcome.value) {
                                        return Err(ExecError::Idempotency {
                                            key: key.to_string(),
                                            source: err,
                                        });
                                    }
                                }
                                return Ok(outcome);
                            }
                            Err(fallback_err) => {
                                return Err(ExecError::Fallback {
                                    route: on_error.route.clone(),
                                    source: fallback_err,
                                });
                            }
                        }
                    }

                    return Err(ExecError::from(err));
                }
            }
        }
    })
}

pub fn exec_out(
    flow_id: &str,
    node_name: &str,
    ctx: &Ctx,
    spec: &OutSpec,
) -> Result<Value, ExecError> {
    let span = info_span!(
        "exec_out",
        flow_id,
        node_name,
        node_type = "f2f.out",
        tenant = ctx.meta.scope.tenant.as_str(),
        team = ctx.meta.scope.team.as_deref().unwrap_or(""),
        user = ctx.meta.scope.user.as_deref().unwrap_or(""),
    );

    span.in_scope(|| {
        let mut payload = Map::new();
        for (key, validator) in &spec.returns {
            match ctx.get_from_state_or_params(key) {
                Some(value) => {
                    validate_value(key, value, validator)?;
                    payload.insert(key.clone(), value.clone());
                }
                None => {
                    if validator.required {
                        return Err(ExecError::Validation(format!(
                            "missing required return `{key}`"
                        )));
                    }
                    if let Some(default) = &validator.default {
                        payload.insert(key.clone(), default.clone());
                    }
                }
            }
        }
        Ok(Value::Object(payload))
    })
}

fn compute_timeout(
    deadline: Option<Instant>,
    timeout_ms: Option<u64>,
    node_name: &str,
) -> Result<Option<Duration>, ExecError> {
    let now = Instant::now();

    let deadline_remaining = deadline.and_then(|instant| instant.checked_duration_since(now));
    if deadline.is_some() && deadline_remaining.is_none() {
        return Err(ExecError::DeadlineExceeded { node: node_name.to_string() });
    }

    let node_timeout = timeout_ms.map(Duration::from_millis);
    let effective = match (deadline_remaining, node_timeout) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    Ok(effective)
}

fn compute_next_delay(current: u64, max_opt: Option<u64>) -> u64 {
    let next = if current == 0 { 100 } else { current.saturating_mul(2) };
    match max_opt {
        Some(max) => next.min(max),
        None => next,
    }
}

fn build_call_payload(
    ctx: &Ctx,
    spec: &CallSpec,
    templates: &TemplateEngine,
) -> Result<Value, ExecError> {
    if spec.params_map.is_empty() {
        return Ok(ctx.params.clone());
    }
    let mut root = Value::Object(Map::new());
    for (target, template) in &spec.params_map {
        let value = templates
            .render_value(template, ctx)
            .map_err(|err| ExecError::Template { template: template.clone(), source: err })?;
        set_value_path(&mut root, target, value)
            .map_err(|e| ExecError::StatePath { path: target.clone(), source: e })?;
    }
    Ok(root)
}

fn build_fallback_request(
    ctx: &Ctx,
    templates: &TemplateEngine,
    on_error: &flow2flow_contract::OnErrorSpec,
    last_request: &Value,
    err: &ResolverError,
) -> Result<CallRequest, ExecError> {
    let mut base = ctx.template_snapshot();
    if let Value::Object(ref mut map) = base {
        map.insert(
            "error".into(),
            json!({ "kind": format!("{:?}", err.kind), "message": err.message }),
        );
        map.insert("last_request".into(), last_request.clone());
    }
    let payload = if let Some(mapper) = &on_error.mapper {
        templates
            .render_with_data(mapper, &base)
            .map_err(|err| ExecError::Template { template: mapper.clone(), source: err })?
    } else {
        last_request.clone()
    };
    Ok(CallRequest { params: payload, timeout: None, attempt: 1 })
}

fn handle_call_response(
    ctx: &mut Ctx,
    spec: &CallSpec,
    response: ResolverResponse,
    templates: &TemplateEngine,
) -> Result<CallOutcome, ExecError> {
    match response {
        ResolverResponse::Sync(value) => {
            apply_result_map(ctx, spec, &value, templates)?;
            let mut outcome = CallOutcome::new(value, 1, spec.mode.clone());
            outcome.fanout = None;
            Ok(outcome)
        }
        ResolverResponse::AsyncAck(value) => {
            apply_result_map(ctx, spec, &value, templates)?;
            let mut outcome = CallOutcome::new(value, 1, spec.mode.clone());
            outcome.fanout = None;
            Ok(outcome)
        }
        ResolverResponse::Fanout(values) => {
            let aggregate = aggregate_fanout(spec, &values)?;
            apply_result_map(ctx, spec, &aggregate, templates)?;
            let mut outcome = CallOutcome::new(aggregate, 1, spec.mode.clone());
            outcome.fanout = Some(values);
            Ok(outcome)
        }
    }
}

fn aggregate_fanout(spec: &CallSpec, values: &[Value]) -> Result<Value, ExecError> {
    let strategy =
        spec.join.as_ref().and_then(|join| join.strategy.clone()).unwrap_or(JoinStrategy::All);

    match strategy {
        JoinStrategy::All => Ok(Value::Array(values.to_vec())),
        JoinStrategy::Any => {
            for value in values {
                if !value.is_null() {
                    return Ok(value.clone());
                }
            }
            Ok(Value::Null)
        }
    }
}

fn apply_result_map(
    ctx: &mut Ctx,
    spec: &CallSpec,
    result: &Value,
    templates: &TemplateEngine,
) -> Result<(), ExecError> {
    if spec.result_map.is_empty() {
        ctx.ensure_state_object().insert("result".to_string(), result.clone());
        return Ok(());
    }

    let mut data = ctx.template_snapshot();
    if let Value::Object(ref mut map) = data {
        map.insert("result".into(), result.clone());
    }

    for (path, template) in &spec.result_map {
        let value = templates
            .render_with_data(template, &data)
            .map_err(|err| ExecError::Template { template: template.clone(), source: err })?;
        ctx.set_state_path(path, value)
            .map_err(|e| ExecError::StatePath { path: path.clone(), source: e })?;
    }
    Ok(())
}

fn validate_value(name: &str, value: &Value, validator: &ValidatorSpec) -> Result<(), ExecError> {
    if !matches_kind(&validator.kind, value) {
        return Err(ExecError::Validation(format!("value `{name}` expected {:?}", validator.kind)));
    }
    Ok(())
}

fn matches_kind(kind: &ParamKind, value: &Value) -> bool {
    match kind {
        ParamKind::String => value.is_string(),
        ParamKind::Integer => value.is_i64(),
        ParamKind::Number => value.is_number(),
        ParamKind::Boolean => value.is_boolean(),
        ParamKind::Object => value.is_object(),
        ParamKind::Array => value.is_array(),
    }
}

fn ensure_scope_compatible(expected: &Scope, actual: &Scope) -> Result<(), ExecError> {
    if expected.tenant != actual.tenant {
        return Err(ExecError::ScopeMismatch {
            expected: expected.tenant.clone(),
            actual: actual.tenant.clone(),
        });
    }
    if let Some(team) = &expected.team {
        if actual.team.as_ref() != Some(team) {
            return Err(ExecError::ScopeMismatch {
                expected: team.clone(),
                actual: actual.team.clone().unwrap_or_default(),
            });
        }
    }
    if let Some(user) = &expected.user {
        if actual.user.as_ref() != Some(user) {
            return Err(ExecError::ScopeMismatch {
                expected: user.clone(),
                actual: actual.user.clone().unwrap_or_default(),
            });
        }
    }
    Ok(())
}

fn set_value_path(
    root: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), crate::ctx::StatePathError> {
    if !root.is_object() {
        *root = Value::Object(Map::new());
    }
    let map = root.as_object_mut().expect("object");
    crate::ctx::set_path(map, path, value)
}
