use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use flow2flow_contract::{
    load_flow_from_json_str, load_flow_from_yaml_str, LoadOutcome, NodeDef, Scope,
};
use serde_json::{json, Value};

use flow2flow_runtime::{
    exec_call, exec_in, exec_out, CallOutcome, Ctx, IdempotencyStore, IdempotencyStoreError, Meta,
    TemplateEngine,
};

#[cfg(feature = "inmem-registry")]
use flow2flow_router_adapter::{can_call, FlowScopeRef, InMemoryRegistry, RouterAdapter};
#[cfg(feature = "inmem-registry")]
use once_cell::sync::Lazy;

/// Flow2Flow CLI command surface.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Flow2Flow developer CLI",
    propagate_version = true,
    disable_version_flag = true
)]
pub struct Cli {
    /// Registry backend (default: in-memory during development)
    #[arg(long, global = true)]
    registry: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Validate a flow definition against schema + structural rules
    Validate { path: PathBuf },

    /// Publish a flow definition into the registry
    Publish {
        path: PathBuf,
        #[arg(long)]
        tenant: String,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        user: Option<String>,
        #[arg(long, default_value_t = false)]
        activate: bool,
    },

    /// Resolve a flow for a given scope
    Resolve {
        flow_id: String,
        #[arg(long)]
        tenant: String,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        user: Option<String>,
        #[arg(long)]
        version: Option<String>,
        #[arg(long, default_value = "assistant.shell")]
        caller: String,
    },

    /// Run a flow locally using the developer stub resolver
    Run {
        path: PathBuf,
        #[arg(short = 'i', long = "input")]
        input: PathBuf,
        #[arg(long)]
        tenant: String,
        #[arg(long)]
        team: Option<String>,
        #[arg(long)]
        user: Option<String>,
    },
}

#[cfg(feature = "inmem-registry")]
static ADAPTER: Lazy<RouterAdapter<InMemoryRegistry>> =
    Lazy::new(|| RouterAdapter::new(InMemoryRegistry::new()));

#[cfg(feature = "inmem-registry")]
fn adapter() -> &'static RouterAdapter<InMemoryRegistry> {
    &ADAPTER
}

#[cfg(feature = "inmem-registry")]
fn ensure_inmem_backend(
    registry: Option<&str>,
) -> Result<&'static RouterAdapter<InMemoryRegistry>> {
    match registry {
        None | Some("inmem") => Ok(adapter()),
        Some(other) => {
            Err(anyhow!("registry `{other}` not supported in developer mode; use --registry inmem"))
        }
    }
}

fn load_flow(path: &PathBuf) -> Result<LoadOutcome> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read flow definition at {}", path.display()))?;
    let outcome = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => load_flow_from_json_str(&contents)?,
        _ => load_flow_from_yaml_str(&contents)?,
    };
    Ok(outcome)
}

fn validate_command(path: PathBuf) -> Result<String> {
    match load_flow(&path) {
        Ok(outcome) => {
            let spec = outcome.spec;
            let output = json!({
                "status": "ok",
                "flow_id": spec.flow_id,
                "version": spec.version,
                "deprecated_nodes_array": outcome.deprecated_nodes_array,
                "path": path,
            });
            Ok(serde_json::to_string_pretty(&output)?)
        }
        Err(err) => {
            let output = json!({
                "status": "error",
                "path": path,
                "message": err.to_string(),
            });
            Ok(serde_json::to_string_pretty(&output)?)
        }
    }
}

#[cfg(feature = "inmem-registry")]
fn publish_command(
    path: PathBuf,
    scope: Scope,
    activate: bool,
    registry: Option<&str>,
) -> Result<String> {
    let adapter = ensure_inmem_backend(registry)?;
    let outcome = load_flow(&path)?;
    let spec = outcome.spec;
    let registration_scope = FlowScopeRef::Scoped(&scope);
    let registered_path = adapter.register_flow_spec(registration_scope, &spec)?;

    let output = json!({
        "status": "published",
        "flow_id": spec.flow_id,
        "version": spec.version,
        "path": registered_path,
        "activated": activate,
        "scope": {
            "tenant": scope.tenant,
            "team": scope.team,
            "user": scope.user,
        },
        "deprecated_nodes_array": outcome.deprecated_nodes_array,
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

#[cfg(feature = "inmem-registry")]
fn resolve_command(
    flow_id: String,
    scope: Scope,
    version: Option<String>,
    caller: String,
    registry: Option<&str>,
) -> Result<String> {
    let adapter = ensure_inmem_backend(registry)?;
    let resolved = adapter
        .resolve_signature(&flow_id, version.as_deref(), &scope)?
        .ok_or_else(|| anyhow!("flow `{flow_id}` not found for scope"))?;

    let caller_allowed = can_call(&caller, &resolved.signature.allow, &scope);

    let output = json!({
        "flow_id": resolved.signature.flow_id,
        "intent": resolved.signature.intent,
        "path": resolved.path,
        "version": resolved.signature.version.to_string(),
        "allow": resolved.signature.allow,
        "caller": caller,
        "caller_allowed": caller_allowed,
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

#[derive(Debug, Default)]
struct NoopStore;

impl IdempotencyStore for NoopStore {
    fn load(&self, _key: &str) -> Result<Option<Value>, IdempotencyStoreError> {
        Ok(None)
    }

    fn store(&self, _key: &str, _value: &Value) -> Result<(), IdempotencyStoreError> {
        Ok(())
    }
}

#[derive(Default)]
struct StubResolver;

impl flow2flow_runtime::Resolver for StubResolver {
    fn resolve(
        &self,
        _ctx: &Ctx,
        spec: &flow2flow_contract::CallSpec,
        _request: &flow2flow_runtime::CallRequest,
    ) -> Result<flow2flow_runtime::ResolverResponse, flow2flow_runtime::ResolverError> {
        Ok(flow2flow_runtime::ResolverResponse::Sync(json!({
            "payload": {
                "forecast_date": "today",
                "temp_c": 21.0,
                "description": "Stub forecast",
                "target": spec.target,
                "mode": spec.mode,
            }
        })))
    }
}

fn read_json(path: &PathBuf) -> Result<Value> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read input at {}", path.display()))?;
    let value = serde_json::from_str(&contents)
        .with_context(|| format!("input at {} is not valid JSON", path.display()))?;
    Ok(value)
}

fn run_command(path: PathBuf, input: PathBuf, scope: Scope) -> Result<String> {
    let outcome = load_flow(&path)?;
    let spec = outcome.spec;

    let params = read_json(&input)?;

    let correlation_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| format!("run-{}", d.as_millis()))
        .unwrap_or_else(|_| "run-0".to_string());

    let meta = Meta::new(scope.clone(), None, correlation_id);
    let mut ctx = Ctx::new(meta).with_params(params);
    ctx.add_permission(scope.tenant.clone());
    if let Some(team) = &scope.team {
        ctx.add_permission(format!("{}:{}", scope.tenant, team));
    }
    if let Some(user) = &scope.user {
        ctx.add_permission(format!("{}::{}", scope.tenant, user));
    }
    let templates = TemplateEngine::default();
    let resolver = StubResolver;
    let store = NoopStore;
    let mut calls = Vec::new();
    let mut result: Option<Value> = None;

    for (node_name, node) in &spec.nodes {
        match node {
            NodeDef::In { spec: in_spec } => {
                exec_in(&spec.flow_id, node_name, &mut ctx, in_spec)?;
            }
            NodeDef::Call { spec: call_spec } => {
                let outcome = exec_call(
                    &spec.flow_id,
                    node_name,
                    &mut ctx,
                    call_spec,
                    &resolver,
                    Some(&store),
                    &templates,
                )?;
                if let Some(payload) = outcome.value.get("payload") {
                    ctx.set_state_path("payload", payload.clone())?;
                }
                calls.push(call_trace(node_name, call_spec, &outcome));
            }
            NodeDef::Out { spec: out_spec } => {
                result = Some(exec_out(&spec.flow_id, node_name, &ctx, out_spec)?);
            }
        }
    }

    let output = json!({
        "status": "ok",
        "flow_id": spec.flow_id,
        "version": spec.version,
        "calls": calls,
        "result": result.unwrap_or(Value::Null),
    });

    Ok(serde_json::to_string_pretty(&output)?)
}

fn call_trace(
    node_name: &str,
    spec: &flow2flow_contract::CallSpec,
    outcome: &CallOutcome,
) -> Value {
    json!({
        "node": node_name,
        "target": spec.target,
        "mode": spec.mode,
        "attempts": outcome.attempts,
        "fanout": outcome.fanout,
        "response": outcome.value,
    })
}

#[cfg(feature = "inmem-registry")]
pub fn run(cli: Cli) -> Result<String> {
    match cli.command {
        Command::Validate { path } => validate_command(path),
        Command::Publish { path, tenant, team, user, activate } => {
            let scope = Scope { tenant, team, user };
            publish_command(path, scope, activate, cli.registry.as_deref())
        }
        Command::Resolve { flow_id, tenant, team, user, version, caller } => {
            let scope = Scope { tenant, team, user };
            resolve_command(flow_id, scope, version, caller, cli.registry.as_deref())
        }
        Command::Run { path, input, tenant, team, user } => {
            let scope = Scope { tenant, team, user };
            run_command(path, input, scope)
        }
    }
}

#[cfg(feature = "inmem-registry")]
pub fn run_from_iter<I, S>(args: I) -> Result<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let argv: Vec<String> = args.into_iter().map(Into::into).collect();
    let cli = Cli::parse_from(argv);
    run(cli)
}

#[cfg(feature = "inmem-registry")]
pub fn run_from_env() -> Result<String> {
    let cli = Cli::parse();
    run(cli)
}

#[cfg(feature = "inmem-registry")]
pub mod testing {
    use super::*;

    pub fn reset_registry() {
        adapter().registry().clear();
    }
}

#[cfg(not(feature = "inmem-registry"))]
pub fn run_from_env() -> Result<String> {
    Err(anyhow!("in-memory registry disabled; rebuild with `--features inmem-registry`"))
}
