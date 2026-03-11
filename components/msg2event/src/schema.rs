//! Schema definitions for msg2event component.

use std::collections::BTreeMap;

use greentic_types::cbor::canonical;
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};

/// Returns the input schema for the convert operation.
/// Expects a ChannelMessageEnvelope-like structure plus configuration.
pub fn input_schema() -> SchemaIr {
    let message_schema = object_schema(vec![
        ("id", string_schema(1, 256), true),
        ("tenant", tenant_ctx_schema(), true),
        ("channel", string_schema(1, 64), true),
        ("session_id", string_schema(1, 256), true),
        ("reply_scope", any_schema(), false),
        ("from", actor_schema(), false),
        ("to", destinations_array_schema(), false),
        ("correlation_id", string_schema(1, 256), false),
        ("text", string_schema(1, 65536), false),
        ("attachments", attachments_array_schema(), false),
        ("metadata", metadata_schema(), false),
    ]);

    let config_schema = object_schema(vec![
        ("topic", string_schema(1, 512), true),
        ("event_type", string_schema(1, 256), true),
        ("source", string_schema(1, 1024), false),
    ]);

    object_schema(vec![
        ("message", message_schema, true),
        ("config", config_schema, true),
    ])
}

/// Returns the output schema for the convert operation.
/// Produces an EventEnvelope-like structure.
pub fn output_schema() -> SchemaIr {
    object_schema(vec![
        ("id", string_schema(1, 256), true),
        ("topic", string_schema(1, 512), true),
        ("type", string_schema(1, 256), true),
        ("source", string_schema(1, 1024), true),
        ("tenant", tenant_ctx_schema(), true),
        ("subject", string_schema(1, 512), false),
        ("time", string_schema(1, 64), false), // RFC3339 timestamp
        ("correlation_id", string_schema(1, 256), false),
        ("payload", any_schema(), false),
        ("metadata", metadata_schema(), false),
    ])
}

/// Returns the config schema for setup.
pub fn config_schema() -> SchemaIr {
    object_schema(vec![
        ("default_topic", string_schema(1, 512), true),
        ("default_event_type", string_schema(1, 256), true),
    ])
}

/// Serializes input schema to CBOR.
pub fn input_schema_cbor() -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&input_schema()).unwrap_or_default()
}

/// Serializes output schema to CBOR.
pub fn output_schema_cbor() -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&output_schema()).unwrap_or_default()
}

/// Serializes config schema to CBOR.
pub fn config_schema_cbor() -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&config_schema()).unwrap_or_default()
}

// Helper functions for schema construction

fn string_schema(min: u64, max: u64) -> SchemaIr {
    SchemaIr::String {
        min_len: Some(min),
        max_len: Some(max),
        regex: None,
        format: None,
    }
}

/// Creates an object schema with explicit required tracking.
fn object_schema(props: Vec<(&str, SchemaIr, bool)>) -> SchemaIr {
    let mut properties = BTreeMap::new();
    let mut required = Vec::new();
    for (name, schema, is_required) in props {
        properties.insert(String::from(name), schema);
        if is_required {
            required.push(String::from(name));
        }
    }
    SchemaIr::Object {
        properties,
        required,
        additional: AdditionalProperties::Allow,
    }
}

fn tenant_ctx_schema() -> SchemaIr {
    object_schema(vec![
        ("tenant_id", string_schema(1, 128), true),
        ("team_id", string_schema(1, 128), false),
        ("user_id", string_schema(1, 128), false),
        ("env_id", string_schema(1, 64), true),
    ])
}

fn actor_schema() -> SchemaIr {
    object_schema(vec![
        ("id", string_schema(1, 256), true),
        ("kind", string_schema(1, 64), false),
    ])
}

fn destinations_array_schema() -> SchemaIr {
    SchemaIr::Array {
        items: Box::new(object_schema(vec![
            ("id", string_schema(1, 512), true),
            ("kind", string_schema(1, 64), false),
        ])),
        min_items: None,
        max_items: None,
    }
}

fn attachments_array_schema() -> SchemaIr {
    SchemaIr::Array {
        items: Box::new(object_schema(vec![
            ("mime_type", string_schema(1, 128), true),
            ("url", string_schema(1, 2048), true),
            ("name", string_schema(1, 256), false),
            (
                "size_bytes",
                SchemaIr::Int {
                    min: Some(0),
                    max: None,
                },
                false,
            ),
        ])),
        min_items: None,
        max_items: None,
    }
}

/// Creates a schema for metadata (string-to-string map).
/// Uses Object with additional properties allowing string values.
fn metadata_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Schema(Box::new(SchemaIr::String {
            min_len: None,
            max_len: Some(4096),
            regex: None,
            format: None,
        })),
    }
}

/// Creates an "any" schema using OneOf with common types.
fn any_schema() -> SchemaIr {
    SchemaIr::OneOf {
        variants: vec![
            SchemaIr::Null,
            SchemaIr::Bool,
            SchemaIr::String {
                min_len: None,
                max_len: None,
                regex: None,
                format: None,
            },
            SchemaIr::Int {
                min: None,
                max: None,
            },
            SchemaIr::Float {
                min: None,
                max: None,
            },
            SchemaIr::Object {
                properties: BTreeMap::new(),
                required: Vec::new(),
                additional: AdditionalProperties::Allow,
            },
            SchemaIr::Array {
                items: Box::new(SchemaIr::Null), // Placeholder, allows any array
                min_items: None,
                max_items: None,
            },
        ],
    }
}
