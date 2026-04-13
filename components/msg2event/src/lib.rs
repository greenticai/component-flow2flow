//! Message to Event converter component.
//!
//! Converts ChannelMessageEnvelope to EventEnvelope for cross-domain flow orchestration.

#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]

use greentic_interfaces_guest::component_v0_6::{component_i18n, component_qa, node};

pub mod config;
pub mod convert;
mod descriptor;
mod schema;

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

struct Component;

impl node::Guest for Component {
    fn describe() -> node::ComponentDescriptor {
        let info = descriptor::info();
        node::ComponentDescriptor {
            name: info.id,
            version: info.version,
            summary: Some("Converts ChannelMessageEnvelope to EventEnvelope".to_string()),
            capabilities: Vec::new(),
            ops: vec![node::Op {
                name: "convert".to_string(),
                summary: Some("Convert message to event envelope".to_string()),
                input: node::IoSchema {
                    schema: node::SchemaSource::InlineCbor(schema::input_schema_cbor()),
                    content_type: "application/cbor".to_string(),
                    schema_version: None,
                },
                output: node::IoSchema {
                    schema: node::SchemaSource::InlineCbor(schema::output_schema_cbor()),
                    content_type: "application/cbor".to_string(),
                    schema_version: None,
                },
                examples: Vec::new(),
            }],
            schemas: Vec::new(),
            setup: None,
        }
    }

    fn invoke(
        operation: String,
        envelope: node::InvocationEnvelope,
    ) -> Result<node::InvocationResult, node::NodeError> {
        match operation.as_str() {
            "convert" => {
                let output = convert::run(envelope.payload_cbor);
                Ok(node::InvocationResult {
                    ok: true,
                    output_cbor: output,
                    output_metadata_cbor: None,
                })
            }
            _ => {
                let error_output = greentic_types::cbor::canonical::to_canonical_cbor_allow_floats(
                    &serde_json::json!({
                        "error": format!("unsupported operation: {operation}")
                    }),
                )
                .unwrap_or_default();
                Ok(node::InvocationResult {
                    ok: false,
                    output_cbor: error_output,
                    output_metadata_cbor: None,
                })
            }
        }
    }
}

impl component_qa::Guest for Component {
    fn qa_spec(_mode: component_qa::QaMode) -> Vec<u8> {
        Vec::new()
    }

    fn apply_answers(
        _mode: component_qa::QaMode,
        current_config: Vec<u8>,
        _answers: Vec<u8>,
    ) -> Vec<u8> {
        current_config
    }
}

impl component_i18n::Guest for Component {
    fn i18n_keys() -> Vec<String> {
        Vec::new()
    }
}

#[cfg(target_arch = "wasm32")]
greentic_interfaces_guest::export_component_v060!(
    Component,
    component_qa: Component,
    component_i18n: Component,
);
