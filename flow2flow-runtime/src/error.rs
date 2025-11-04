use thiserror::Error;

use crate::ctx::StatePathError;
use crate::idempotency::IdempotencyStoreError;
use crate::resolver::ResolverError;

#[derive(Debug, Error)]
pub enum ExecError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("permission denied; required permissions: {required:?}")]
    PermissionDenied { required: Vec<String> },
    #[error("scope mismatch: expected tenant `{expected}`, got `{actual}`")]
    ScopeMismatch { expected: String, actual: String },
    #[error("template `{template}` error: {source}")]
    Template {
        template: String,
        #[source]
        source: minijinja::Error,
    },
    #[error("resolver error: {source}")]
    Resolver {
        #[from]
        source: ResolverError,
    },
    #[error("fallback `{route}` failed: {source}")]
    Fallback {
        route: String,
        #[source]
        source: ResolverError,
    },
    #[error("deadline exceeded for node `{node}`")]
    DeadlineExceeded { node: String },
    #[error("idempotency `{key}` conflict: {source}")]
    Idempotency {
        key: String,
        #[source]
        source: IdempotencyStoreError,
    },
    #[error("state path `{path}` invalid: {source}")]
    StatePath {
        path: String,
        #[source]
        source: StatePathError,
    },
}
