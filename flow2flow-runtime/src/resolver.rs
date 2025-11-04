use std::time::Duration;

use serde_json::Value;
use thiserror::Error;

use crate::ctx::Ctx;
use flow2flow_contract::CallSpec;

#[derive(Debug, Clone)]
pub struct CallRequest {
    pub params: Value,
    pub timeout: Option<Duration>,
    pub attempt: u32,
}

#[derive(Debug, Clone)]
pub enum ResolverResponse {
    Sync(Value),
    AsyncAck(Value),
    Fanout(Vec<Value>),
}

#[derive(Debug, Clone, Copy)]
pub struct FallbackContext<'a> {
    pub route: &'a str,
    pub request: &'a CallRequest,
}

pub trait Resolver {
    fn resolve(
        &self,
        ctx: &Ctx,
        spec: &CallSpec,
        request: &CallRequest,
    ) -> Result<ResolverResponse, ResolverError>;

    fn resolve_fallback(
        &self,
        _ctx: &Ctx,
        _spec: &CallSpec,
        _fallback: &FallbackContext<'_>,
    ) -> Result<ResolverResponse, ResolverError> {
        Err(ResolverError::unsupported("fallback"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverErrorKind {
    Retryable,
    Fatal,
    Timeout,
    Rejected,
}

#[derive(Debug, Error, Clone)]
#[error("{kind:?}: {message}")]
pub struct ResolverError {
    pub kind: ResolverErrorKind,
    pub message: String,
}

impl ResolverError {
    pub fn retryable(message: impl Into<String>) -> Self {
        Self { kind: ResolverErrorKind::Retryable, message: message.into() }
    }

    pub fn fatal(message: impl Into<String>) -> Self {
        Self { kind: ResolverErrorKind::Fatal, message: message.into() }
    }

    pub fn timeout(message: impl Into<String>) -> Self {
        Self { kind: ResolverErrorKind::Timeout, message: message.into() }
    }

    pub fn rejected(message: impl Into<String>) -> Self {
        Self { kind: ResolverErrorKind::Rejected, message: message.into() }
    }

    pub fn unsupported(operation: &str) -> Self {
        Self::fatal(format!("resolver does not support {operation}"))
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self.kind, ResolverErrorKind::Retryable | ResolverErrorKind::Timeout)
    }
}
