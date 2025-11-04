use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub type NodeName = String;
pub type Nodes = BTreeMap<NodeName, NodeDef>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParamKind {
    String,
    Integer,
    Number,
    Boolean,
    Object,
    Array,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ValidatorSpec {
    pub kind: ParamKind,
    #[serde(default)]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
}

impl ValidatorSpec {
    pub fn validate(&self, node: &str, name: &str) -> Result<(), FlowValidationError> {
        if name.trim().is_empty() {
            return Err(FlowValidationError::InvalidParamName {
                node: node.to_string(),
                param: name.to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InSpec {
    pub intent: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, ValidatorSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visibility: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
}

impl InSpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.intent.trim().is_empty() {
            return Err(FlowValidationError::MissingIntent { node: node.to_string() });
        }

        for (name, validator) in &self.params {
            validator.validate(node, name)?;
        }

        if self.allow.iter().any(|entry| entry.trim().is_empty()) {
            return Err(FlowValidationError::InvalidAllowList { node: node.to_string() });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OutSpec {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub returns: BTreeMap<String, ValidatorSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
}

impl OutSpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        for (name, validator) in &self.returns {
            validator.validate(node, name)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RetrySpec {
    pub attempts: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_delay_ms: Option<u64>,
}

impl RetrySpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.attempts == 0 {
            return Err(FlowValidationError::InvalidRetryAttempts { node: node.to_string() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallMode {
    Sync,
    Async,
    FireAndForget,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OnErrorSpec {
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<String>,
}

impl OnErrorSpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.route.trim().is_empty() {
            return Err(FlowValidationError::InvalidOnErrorRoute { node: node.to_string() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct JoinSpec {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub with: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy: Option<JoinStrategy>,
}

impl JoinSpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.with.is_empty() {
            return Err(FlowValidationError::EmptyJoinMembers { node: node.to_string() });
        }
        if self.with.iter().any(|entry| entry.trim().is_empty()) {
            return Err(FlowValidationError::EmptyJoinMembers { node: node.to_string() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum JoinStrategy {
    All,
    Any,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Scope {
    pub tenant: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

impl Scope {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.tenant.trim().is_empty() {
            return Err(FlowValidationError::MissingScopeTenant { node: node.to_string() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CallSpec {
    pub target: String,
    pub mode: CallMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetrySpec>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params_map: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub result_map: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_error: Option<OnErrorSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join: Option<JoinSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
}

impl CallSpec {
    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        if self.target.trim().is_empty() {
            return Err(FlowValidationError::MissingCallTarget { node: node.to_string() });
        }

        if let Some(retry) = &self.retry {
            retry.validate(node)?;
        }

        for (key, value) in &self.params_map {
            if key.trim().is_empty() || value.trim().is_empty() {
                return Err(FlowValidationError::InvalidMappingEntry {
                    node: node.to_string(),
                    key: key.to_string(),
                });
            }
        }

        for (key, value) in &self.result_map {
            if key.trim().is_empty() || value.trim().is_empty() {
                return Err(FlowValidationError::InvalidMappingEntry {
                    node: node.to_string(),
                    key: key.to_string(),
                });
            }
        }

        if let Some(on_error) = &self.on_error {
            on_error.validate(node)?;
        }

        if let Some(join) = &self.join {
            join.validate(node)?;
        }

        if let Some(scope) = &self.scope {
            scope.validate(node)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum NodeDef {
    In {
        #[serde(rename = "f2f.in")]
        spec: InSpec,
    },
    Call {
        #[serde(rename = "f2f.call")]
        spec: CallSpec,
    },
    Out {
        #[serde(rename = "f2f.out")]
        spec: OutSpec,
    },
}

impl NodeDef {
    pub fn kind(&self) -> NodeKind {
        match self {
            NodeDef::In { .. } => NodeKind::In,
            NodeDef::Call { .. } => NodeKind::Call,
            NodeDef::Out { .. } => NodeKind::Out,
        }
    }

    pub fn validate(&self, node: &str) -> Result<(), FlowValidationError> {
        match self {
            NodeDef::In { spec } => spec.validate(node),
            NodeDef::Call { spec } => spec.validate(node),
            NodeDef::Out { spec } => spec.validate(node),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    In,
    Call,
    Out,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ErrorSpec {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Value>,
}

impl ErrorSpec {
    pub fn validate(&self, name: &str) -> Result<(), FlowValidationError> {
        if self.code.trim().is_empty() {
            return Err(FlowValidationError::MissingErrorCode { name: name.to_string() });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FlowSpec {
    pub flow_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    #[serde(default)]
    pub nodes: Nodes,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub errors: BTreeMap<String, ErrorSpec>,
}

impl FlowSpec {
    pub fn validate(&self) -> Result<(), FlowValidationError> {
        if self.nodes.is_empty() {
            return Err(FlowValidationError::MissingNodes);
        }

        for name in self.nodes.keys() {
            if name.trim().is_empty() {
                return Err(FlowValidationError::EmptyNodeName);
            }
        }

        let (first_name, first_node) =
            self.nodes.iter().next().ok_or(FlowValidationError::MissingNodes)?;
        if !matches!(first_node, NodeDef::In { .. }) {
            return Err(FlowValidationError::FirstNodeMustBeIn { node: first_name.clone() });
        }

        let (last_name, last_node) =
            self.nodes.iter().next_back().ok_or(FlowValidationError::MissingNodes)?;
        if !matches!(last_node, NodeDef::Out { .. }) {
            return Err(FlowValidationError::LastNodeMustBeOut { node: last_name.clone() });
        }

        for (name, node) in &self.nodes {
            node.validate(name)?;
        }

        for (name, spec) in &self.errors {
            if name.trim().is_empty() {
                return Err(FlowValidationError::MissingErrorCode { name: name.to_string() });
            }
            spec.validate(name)?;
        }

        Ok(())
    }
}

#[derive(Debug, Error, Clone, PartialEq)]
pub enum FlowValidationError {
    #[error("flow name must not be empty")]
    EmptyName,
    #[error("duplicate step id `{0}` detected")]
    DuplicateStepId(String),
    #[error("flow must contain at least one node")]
    MissingNodes,
    #[error("node name must not be empty")]
    EmptyNodeName,
    #[error("first node `{node}` must be f2f.in")]
    FirstNodeMustBeIn { node: String },
    #[error("last node `{node}` must be f2f.out")]
    LastNodeMustBeOut { node: String },
    #[error("in node `{node}` must declare an intent")]
    MissingIntent { node: String },
    #[error("allow list for node `{node}` contains empty entries")]
    InvalidAllowList { node: String },
    #[error("parameter `{param}` in node `{node}` is invalid")]
    InvalidParamName { node: String, param: String },
    #[error("call node `{node}` must declare a target")]
    MissingCallTarget { node: String },
    #[error("call node `{node}` retry attempts must be greater than zero")]
    InvalidRetryAttempts { node: String },
    #[error("call node `{node}` has on_error route that may not be empty")]
    InvalidOnErrorRoute { node: String },
    #[error("call node `{node}` join clause must specify at least one node")]
    EmptyJoinMembers { node: String },
    #[error("call node `{node}` mapping entry `{key}` is invalid")]
    InvalidMappingEntry { node: String, key: String },
    #[error("scope for node `{node}` must include a tenant")]
    MissingScopeTenant { node: String },
    #[error("error spec `{name}` must include a code")]
    MissingErrorCode { name: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_flow() -> FlowSpec {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "n01".to_string(),
            NodeDef::In {
                spec: InSpec {
                    intent: "weather.fetch".to_string(),
                    params: BTreeMap::new(),
                    router: Some("greentic.weather".to_string()),
                    visibility: vec!["tenant".to_string()],
                    allow: vec!["weather.read".to_string()],
                },
            },
        );
        nodes.insert(
            "n02".to_string(),
            NodeDef::Call {
                spec: CallSpec {
                    target: "weather-service".to_string(),
                    mode: CallMode::Sync,
                    timeout_ms: Some(1_000),
                    retry: Some(RetrySpec {
                        attempts: 3,
                        delay_ms: Some(200),
                        max_delay_ms: Some(2_000),
                    }),
                    params_map: BTreeMap::new(),
                    result_map: BTreeMap::new(),
                    on_error: Some(OnErrorSpec { route: "fallback".to_string(), mapper: None }),
                    join: None,
                    scope: Some(Scope {
                        tenant: "acme".to_string(),
                        team: Some("weather".to_string()),
                        user: None,
                    }),
                },
            },
        );
        nodes.insert(
            "n03".to_string(),
            NodeDef::Out {
                spec: OutSpec {
                    returns: BTreeMap::new(),
                    docs: Some("Return formatted weather payload".to_string()),
                },
            },
        );

        FlowSpec {
            flow_id: "weather".to_string(),
            version: Some(1),
            nodes,
            errors: BTreeMap::new(),
        }
    }

    #[test]
    fn sample_flow_validates() {
        let flow = sample_flow();
        assert!(flow.validate().is_ok());
    }

    #[test]
    fn missing_in_node_fails_validation() {
        let mut flow = sample_flow();
        flow.nodes.remove("n01");
        assert!(matches!(flow.validate(), Err(FlowValidationError::FirstNodeMustBeIn { .. })));
    }
}
