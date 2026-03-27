//! Component descriptor for msg2event.

use std::collections::BTreeMap;

use greentic_types::cbor::canonical;
use greentic_types::schemas::component::v0_6_0::{
    ComponentDescribe, ComponentInfo, ComponentOperation, ComponentRunInput, ComponentRunOutput,
    RedactionKind, RedactionRule, schema_hash,
};

use crate::schema;

/// Returns component metadata.
pub fn info() -> ComponentInfo {
    ComponentInfo {
        id: "flow2flow.msg2event".to_string(),
        version: "0.1.0".to_string(),
        role: "bridge".to_string(),
        display_name: None,
    }
}

/// Returns serialized component info as CBOR.
pub fn info_cbor() -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&info()).unwrap_or_default()
}

/// Returns full component descriptor.
pub fn describe() -> ComponentDescribe {
    let input_schema = schema::input_schema();
    let output_schema = schema::output_schema();
    let config_schema = schema::config_schema();
    let op_hash = schema_hash(&input_schema, &output_schema, &config_schema).expect("schema hash");

    let operation = ComponentOperation {
        id: "convert".to_string(),
        display_name: None,
        input: ComponentRunInput {
            schema: input_schema,
        },
        output: ComponentRunOutput {
            schema: output_schema,
        },
        defaults: BTreeMap::new(),
        redactions: vec![RedactionRule {
            json_pointer: "/payload".to_string(),
            kind: RedactionKind::Secret,
        }],
        constraints: BTreeMap::new(),
        schema_hash: op_hash,
    };

    ComponentDescribe {
        info: info(),
        provided_capabilities: provided_capabilities(),
        required_capabilities: required_capabilities(),
        metadata: BTreeMap::new(),
        operations: vec![operation],
        config_schema,
    }
}

fn required_capabilities() -> Vec<String> {
    vec![]
}

fn provided_capabilities() -> Vec<String> {
    vec!["flow2flow:msg2event".to_string()]
}

/// Returns serialized component descriptor as CBOR.
pub fn describe_cbor() -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&describe()).unwrap_or_default()
}
