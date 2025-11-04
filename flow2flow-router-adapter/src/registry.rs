use std::collections::BTreeMap;
use std::sync::Mutex;

use semver::{Version, VersionReq};

use crate::error::AdapterError;
use crate::scope::{fallback_order, ScopeKey};
use crate::signature::FlowRecord;
use flow2flow_contract::Scope;

pub trait Registry: Send + Sync + Clone {
    fn register(&self, scope: &ScopeKey, record: FlowRecord) -> Result<(), AdapterError>;

    fn versions_for(&self, scope: &ScopeKey, flow_id: &str) -> Vec<VersionedRecord>;

    fn resolve(
        &self,
        flow_id: &str,
        version_req: Option<&VersionReq>,
        caller_scope: &Scope,
    ) -> Result<Option<VersionedRecord>, AdapterError> {
        for scope_key in fallback_order(caller_scope) {
            let versions = self.versions_for(&scope_key, flow_id);
            if versions.is_empty() {
                continue;
            }
            let resolved = select_version(versions, version_req);
            if let Some(record) = resolved {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }
}

#[derive(Debug, Clone)]
pub struct VersionedRecord {
    pub version: Version,
    pub record: FlowRecord,
}

impl VersionedRecord {
    pub fn new(record: FlowRecord) -> Self {
        let version = record.signature.version.clone();
        Self { version, record }
    }
}

fn select_version(
    mut versions: Vec<VersionedRecord>,
    version_req: Option<&VersionReq>,
) -> Option<VersionedRecord> {
    versions.sort_by(|a, b| b.version.cmp(&a.version));
    if let Some(req) = version_req {
        versions.into_iter().find(|entry| req.matches(&entry.version))
    } else {
        versions.into_iter().next()
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryRegistry {
    inner: std::sync::Arc<Mutex<RegistryInner>>,
}

type RegistryInner = BTreeMap<ScopeKey, BTreeMap<String, Vec<VersionedRecord>>>;

impl InMemoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.clear();
        }
    }
}

impl Registry for InMemoryRegistry {
    fn register(&self, scope: &ScopeKey, record: FlowRecord) -> Result<(), AdapterError> {
        let mut guard =
            self.inner.lock().map_err(|_| AdapterError::registry("registry poisoned"))?;
        let scope_entry = guard.entry(scope.clone()).or_default();
        let versions = scope_entry.entry(record.signature.flow_id.clone()).or_default();

        versions.retain(|entry| entry.version != record.signature.version);
        versions.push(VersionedRecord::new(record));
        versions.sort_by(|a, b| b.version.cmp(&a.version));
        Ok(())
    }

    fn versions_for(&self, scope: &ScopeKey, flow_id: &str) -> Vec<VersionedRecord> {
        let guard = self.inner.lock().expect("registry poisoned");
        guard.get(scope).and_then(|flows| flows.get(flow_id)).cloned().unwrap_or_else(Vec::new)
    }
}
