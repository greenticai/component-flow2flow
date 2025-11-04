use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use flow2flow_contract::{
    flow_spec_schema, load_flow_from_yaml_str, schema_artifacts, CallMode, CallSpec, FlowSpec,
    InSpec, NodeDef, OutSpec, Scope, ValidatorSpec,
};
use jsonschema::JSONSchema;
use serde_json::Value;

fn sample_flow() -> FlowSpec {
    let mut nodes = BTreeMap::new();
    nodes.insert(
        "n01".to_string(),
        NodeDef::In {
            spec: InSpec {
                intent: "flow.intent".to_string(),
                params: BTreeMap::from([(
                    "subject".to_string(),
                    ValidatorSpec {
                        kind: flow2flow_contract::ParamKind::String,
                        required: true,
                        description: Some("Subject for the flow".to_string()),
                        default: None,
                    },
                )]),
                router: Some("router.topic".to_string()),
                visibility: vec!["tenant".to_string()],
                allow: vec!["flow.run".to_string()],
            },
        },
    );
    nodes.insert(
        "n02".to_string(),
        NodeDef::Call {
            spec: CallSpec {
                target: "component.fetch".to_string(),
                mode: CallMode::Sync,
                timeout_ms: Some(2_000),
                retry: None,
                params_map: BTreeMap::from([(
                    "input.subject".to_string(),
                    "context.subject".to_string(),
                )]),
                result_map: BTreeMap::new(),
                on_error: None,
                join: None,
                scope: Some(Scope {
                    tenant: "acme".to_string(),
                    team: Some("flows".to_string()),
                    user: None,
                }),
            },
        },
    );
    nodes.insert(
        "n03".to_string(),
        NodeDef::Out {
            spec: OutSpec {
                returns: BTreeMap::from([(
                    "payload".to_string(),
                    ValidatorSpec {
                        kind: flow2flow_contract::ParamKind::Object,
                        required: true,
                        description: Some("Normalized payload".to_string()),
                        default: None,
                    },
                )]),
                docs: Some("Return normalized payload".to_string()),
            },
        },
    );

    FlowSpec {
        flow_id: "flow.intent".to_string(),
        version: Some(1),
        nodes,
        errors: BTreeMap::new(),
    }
}

fn schema_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("schemas")
}

#[test]
fn round_trip_serialization() {
    let flow = sample_flow();
    let json = serde_json::to_string_pretty(&flow).expect("serialize");
    let parsed: FlowSpec = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(flow, parsed);
}

#[test]
fn legacy_nodes_upgrade_sets_flag() {
    let legacy = r#"
flow_id: legacy
nodes:
  - f2f.in:
      intent: legacy.in
      params: {}
  - f2f.out:
      returns: {}
"#;
    let outcome = load_flow_from_yaml_str(legacy).expect("load legacy");
    assert!(outcome.deprecated_nodes_array);
    let keys: Vec<_> = outcome.spec.nodes.keys().cloned().collect();
    assert_eq!(keys, vec!["n1".to_string(), "n2".to_string()]);
}

#[test]
fn schema_fixtures_are_current() {
    for artifact in schema_artifacts() {
        let path = schema_dir().join(artifact.file_name);
        let fixture = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing fixture {}", path.display()));
        let fixture_json: Value = serde_json::from_str(&fixture).expect("fixture to be valid json");
        let generated = serde_json::to_value(&artifact.schema).expect("schema to serialize");
        assert_eq!(generated, fixture_json, "schema fixture out of date: {}", artifact.file_name);
    }
}

#[test]
fn schema_validates_sample_flow() {
    let schema = flow_spec_schema();
    let schema_json = serde_json::to_value(schema).expect("schema to serialize");
    let validator = JSONSchema::compile(&schema_json).expect("compile schema");
    let instance = serde_json::to_value(sample_flow()).expect("spec to serialize");
    let result = validator.validate(&instance);
    if let Err(errors) = result {
        let messages: Vec<String> = errors.map(|err| err.to_string()).collect();
        panic!("schema validation failed: {}", messages.join(", "));
    }
}
