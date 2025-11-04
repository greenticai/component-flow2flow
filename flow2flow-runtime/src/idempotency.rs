use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdempotencyStoreError {
    #[error("idempotency store error: {0}")]
    Backend(String),
}

pub trait IdempotencyStore: Send + Sync {
    fn load(&self, key: &str) -> Result<Option<Value>, IdempotencyStoreError>;
    fn store(&self, key: &str, value: &Value) -> Result<(), IdempotencyStoreError>;
}

impl<F, G> IdempotencyStore for (F, G)
where
    F: Fn(&str) -> Result<Option<Value>, IdempotencyStoreError> + Send + Sync,
    G: Fn(&str, &Value) -> Result<(), IdempotencyStoreError> + Send + Sync,
{
    fn load(&self, key: &str) -> Result<Option<Value>, IdempotencyStoreError> {
        (self.0)(key)
    }

    fn store(&self, key: &str, value: &Value) -> Result<(), IdempotencyStoreError> {
        (self.1)(key, value)
    }
}
