use std::collections::BTreeMap;

use flow2flow_contract::InSpec;
use flow2flow_contract::{FlowSpec, NodeDef, OutSpec, ValidatorSpec};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AdapterError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowSignature {
    pub flow_id: String,
    pub version: Version,
    pub intent: String,
    pub allow: Vec<String>,
    pub params: BTreeMap<String, ValidatorSpec>,
    pub returns: BTreeMap<String, ValidatorSpec>,
}

impl FlowSignature {
    pub fn major(&self) -> u64 {
        self.version.major
    }

    pub fn registration_path_fragment(&self) -> String {
        format!("flows/{}@{}", self.flow_id, self.version.major)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowRecord {
    pub signature: FlowSignature,
    pub returns_schema: Value,
    pub path: String,
}

pub fn signature_from_spec(spec: &FlowSpec) -> Result<(FlowSignature, Value), AdapterError> {
    let flow_id = spec.flow_id.clone();
    let version =
        spec.version.ok_or_else(|| AdapterError::MissingVersion { flow_id: flow_id.clone() })?;
    let version = Version::new(version as u64, 0, 0);

    let in_spec = find_in_spec(spec)
        .ok_or_else(|| AdapterError::MissingInNode { flow_id: flow_id.clone() })?;
    let out_spec = find_out_spec(spec)
        .ok_or_else(|| AdapterError::MissingOutNode { flow_id: flow_id.clone() })?;

    let signature = FlowSignature {
        flow_id: flow_id.clone(),
        version,
        intent: in_spec.intent.clone(),
        allow: in_spec.allow.clone(),
        params: in_spec.params.clone(),
        returns: out_spec.returns.clone(),
    };

    let returns_schema = serde_json::to_value(&signature.returns).map_err(|err| {
        AdapterError::registry(format!("failed to serialize returns schema: {err}"))
    })?;

    Ok((signature, returns_schema))
}

fn find_in_spec(spec: &FlowSpec) -> Option<&InSpec> {
    spec.nodes.values().find_map(|node| match node {
        NodeDef::In { spec } => Some(spec),
        _ => None,
    })
}

fn find_out_spec(spec: &FlowSpec) -> Option<&OutSpec> {
    spec.nodes.values().rev().find_map(|node| match node {
        NodeDef::Out { spec } => Some(spec),
        _ => None,
    })
}
