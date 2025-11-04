use std::time::Instant;

use flow2flow_contract::Scope;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub scope: Scope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    pub correlation_id: String,
}

impl Meta {
    pub fn new(scope: Scope, channel: Option<String>, correlation_id: impl Into<String>) -> Self {
        Self { scope, channel, correlation_id: correlation_id.into() }
    }
}

#[derive(Debug, Clone)]
pub struct Ctx {
    pub params: Value,
    pub state: Value,
    pub meta: Meta,
    pub deadline: Option<Instant>,
    pub idempotency_key: Option<String>,
    pub permissions: Vec<String>,
}

impl Ctx {
    pub fn new(meta: Meta) -> Self {
        Self {
            params: Value::Object(Map::new()),
            state: Value::Object(Map::new()),
            meta,
            deadline: None,
            idempotency_key: None,
            permissions: Vec::new(),
        }
    }

    pub fn with_params(mut self, params: Value) -> Self {
        self.params = params;
        self
    }

    pub fn with_state(mut self, state: Value) -> Self {
        self.state = state;
        self
    }

    pub fn with_deadline(mut self, deadline: Option<Instant>) -> Self {
        self.deadline = deadline;
        self
    }

    pub fn with_idempotency_key(mut self, key: Option<String>) -> Self {
        self.idempotency_key = key;
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions = permissions;
        self
    }

    pub fn add_permission(&mut self, permission: impl Into<String>) {
        self.permissions.push(permission.into());
    }

    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.iter().any(|p| p == permission)
    }

    pub fn has_permission_pattern(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        self.permissions.iter().any(|perm| pattern_matches(pattern, perm))
    }

    pub fn ensure_state_object(&mut self) -> &mut Map<String, Value> {
        if !self.state.is_object() {
            self.state = Value::Object(Map::new());
        }
        self.state.as_object_mut().expect("state to be object")
    }

    pub fn template_snapshot(&self) -> Value {
        json!({
            "params": self.params,
            "state": self.state,
            "meta": {
                "scope": self.meta.scope,
                "channel": self.meta.channel,
                "correlation_id": self.meta.correlation_id,
            }
        })
    }

    pub fn get_from_state(&self, path: &str) -> Option<&Value> {
        get_path(&self.state, path)
    }

    pub fn get_from_params(&self, path: &str) -> Option<&Value> {
        get_path(&self.params, path)
    }

    pub fn get_from_state_or_params(&self, path: &str) -> Option<&Value> {
        self.get_from_state(path).or_else(|| self.get_from_params(path))
    }

    pub fn set_state_path(&mut self, path: &str, value: Value) -> Result<(), StatePathError> {
        set_path(self.ensure_state_object(), path, value)
    }
}

#[derive(Debug, Error)]
pub enum StatePathError {
    #[error("path must not be empty")]
    EmptyPath,
    #[error("path segment must not be empty")]
    EmptySegment,
}

pub(crate) fn split_path(path: &str) -> Result<Vec<&str>, StatePathError> {
    if path.is_empty() {
        return Err(StatePathError::EmptyPath);
    }
    let mut segments = Vec::new();
    for segment in path.split('.') {
        if segment.is_empty() {
            return Err(StatePathError::EmptySegment);
        }
        segments.push(segment);
    }
    Ok(segments)
}

pub(crate) fn set_path(
    root: &mut Map<String, Value>,
    path: &str,
    mut value: Value,
) -> Result<(), StatePathError> {
    let segments = split_path(path)?;
    let mut current = root;
    for seg in segments.iter().take(segments.len() - 1) {
        current = current
            .entry((*seg).to_string())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .unwrap();
    }
    if let Some(last) = segments.last() {
        current.insert((*last).to_string(), value.take());
    }
    Ok(())
}

pub fn pattern_matches(pattern: &str, candidate: &str) -> bool {
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
                if pattern_matches(rest, &candidate[split..]) {
                    return true;
                }
            }
            false
        } else if let Some(stripped) = candidate.strip_prefix(prefix) {
            pattern_matches(rest, stripped)
        } else {
            false
        }
    } else {
        pattern == candidate
    }
}

fn get_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let segments = split_path(path).ok()?;
    let mut current = value;
    for seg in segments {
        current = current.as_object()?.get(seg)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow2flow_contract::Scope;

    fn scope() -> Scope {
        Scope { tenant: "tenant".into(), team: Some("team".into()), user: Some("user".into()) }
    }

    #[test]
    fn set_and_get_state_path() {
        let meta = Meta::new(scope(), Some("channel".into()), "corr");
        let mut ctx = Ctx::new(meta);
        ctx.set_state_path("a.b.c", Value::String("value".into())).unwrap();
        let fetched = ctx.get_from_state("a.b.c").unwrap();
        assert_eq!(fetched, &Value::String("value".into()));
    }

    #[test]
    fn split_path_rejects_empty_segment() {
        assert!(matches!(split_path("a..b"), Err(StatePathError::EmptySegment)));
    }
}
