use std::fmt;

use serde_json::{Map, Value};

use crate::{FlowSpec, FlowValidationError};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadOutcome {
    pub spec: FlowSpec,
    pub deprecated_nodes_array: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("failed to parse JSON: {0}")]
    Json(serde_json::Error),
    #[error("failed to parse YAML: {0}")]
    Yaml(serde_yaml::Error),
    #[error("validation error: {0}")]
    Validation(FlowValidationError),
}

fn digits(count: usize) -> usize {
    let mut n = count;
    let mut width = 1;
    while n >= 10 {
        width += 1;
        n /= 10;
    }
    width
}

fn prepare_flow_value(mut value: Value) -> (Value, bool) {
    let mut deprecated = false;

    if let Some(nodes_value) = value.get_mut("nodes") {
        if let Value::Array(nodes) = nodes_value {
            deprecated = true;
            let pad_width = if nodes.len() >= 10 { digits(nodes.len()) } else { 1 };
            let mut map = Map::new();
            let legacy = std::mem::take(nodes);
            for (idx, node) in legacy.into_iter().enumerate() {
                let key = if pad_width > 1 {
                    format!("n{:0width$}", idx + 1, width = pad_width)
                } else {
                    format!("n{}", idx + 1)
                };
                map.insert(key, node);
            }
            *nodes_value = Value::Object(map);
        }
    }

    (value, deprecated)
}

fn load_from_value(value: Value) -> Result<LoadOutcome, LoadError> {
    let (value, deprecated) = prepare_flow_value(value);
    let spec: FlowSpec = serde_json::from_value(value).map_err(LoadError::Json)?;
    spec.validate().map_err(LoadError::Validation)?;
    Ok(LoadOutcome { spec, deprecated_nodes_array: deprecated })
}

pub fn load_flow_from_json_str(input: &str) -> Result<LoadOutcome, LoadError> {
    let value: Value = serde_json::from_str(input).map_err(LoadError::Json)?;
    load_from_value(value)
}

pub fn load_flow_from_yaml_str(input: &str) -> Result<LoadOutcome, LoadError> {
    let yaml_value: serde_yaml::Value = serde_yaml::from_str(input).map_err(LoadError::Yaml)?;
    let value = serde_json::to_value(yaml_value).map_err(LoadError::Json)?;
    load_from_value(value)
}

impl fmt::Display for LoadOutcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "flow {} (deprecated_nodes_array={})",
            self.spec.flow_id, self.deprecated_nodes_array
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detects_deprecated_array_nodes() {
        let doc = json!({
            "flow_id": "demo",
            "nodes": [
                { "f2f.in": { "intent": "demo.intent", "params": {} } },
                { "f2f.out": { "returns": {} } }
            ]
        });

        let serialized = serde_json::to_string(&doc).unwrap();
        let outcome = load_flow_from_json_str(&serialized).unwrap();

        assert!(outcome.deprecated_nodes_array);
        let names: Vec<_> = outcome.spec.nodes.keys().cloned().collect();
        assert_eq!(names, vec!["n1".to_string(), "n2".to_string()]);
    }
}
