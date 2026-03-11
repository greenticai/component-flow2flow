//! Conversion logic from EventEnvelope to ChannelMessageEnvelope.

use std::collections::BTreeMap;

use greentic_types::cbor::canonical;
use serde_json::Value as JsonValue;

use crate::config::{
    ActorOutput, ConvertInput, DestinationOutput, MessageOutput, TenantOutput,
};

/// Executes the conversion from event to message.
///
/// Takes CBOR-encoded input containing the event and configuration,
/// returns CBOR-encoded ChannelMessageEnvelope.
pub fn run(input: Vec<u8>) -> Vec<u8> {
    let result = convert_event_to_message(&input);
    canonical::to_canonical_cbor_allow_floats(&result).unwrap_or_default()
}

fn convert_event_to_message(input: &[u8]) -> JsonValue {
    // Parse input
    let input_value: JsonValue = match canonical::from_cbor(input) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("failed to parse input: {}", e)
            });
        }
    };

    let convert_input: ConvertInput = match serde_json::from_value(input_value) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "error": format!("invalid input structure: {}", e)
            });
        }
    };

    let event = &convert_input.event;
    let config = &convert_input.config;

    // Generate message ID from event ID
    let message_id = format!("msg-{}", event.id);

    // Extract session ID from topic (last segment) or use correlation_id
    let session_id = event
        .correlation_id
        .clone()
        .unwrap_or_else(|| extract_session_from_topic(&event.topic));

    // Build text content
    let text = build_text_content(config.text_template.as_deref(), &event.payload);

    // Build destination
    let destination = DestinationOutput {
        id: config.destination.id.clone(),
        kind: config.destination.kind.clone(),
    };

    // Build from actor from event source
    let from = Some(ActorOutput {
        id: event.source.clone(),
        kind: Some("event".to_string()),
    });

    // Build tenant context
    let tenant = TenantOutput {
        tenant_id: event.tenant.tenant_id.clone(),
        team_id: event.tenant.team_id.clone(),
        user_id: event.tenant.user_id.clone(),
        env_id: event.tenant.env_id.clone(),
        trace_id: String::new(),
        correlation_id: event.correlation_id.clone().unwrap_or_default(),
    };

    // Propagate metadata with event source info
    let mut metadata: BTreeMap<String, String> = event.metadata.clone();
    metadata.insert("event_id".to_string(), event.id.clone());
    metadata.insert("event_type".to_string(), event.event_type.clone());
    metadata.insert("event_topic".to_string(), event.topic.clone());
    metadata.insert("event_time".to_string(), event.time.clone());
    if let Some(ref subject) = event.subject {
        metadata.insert("event_subject".to_string(), subject.clone());
    }

    // Build output
    let output = MessageOutput {
        id: message_id,
        tenant,
        channel: config.target_channel.clone(),
        session_id,
        reply_scope: None,
        from,
        to: vec![destination],
        correlation_id: event.correlation_id.clone(),
        text,
        attachments: Vec::new(),
        metadata,
    };

    serde_json::to_value(output).unwrap_or_else(|_| serde_json::json!({}))
}

/// Extracts a session identifier from the event topic.
/// Uses the last segment of the topic path.
fn extract_session_from_topic(topic: &str) -> String {
    topic
        .split('.')
        .last()
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid_v4())
}

/// Builds the text content for the message.
/// If a template is provided, performs simple placeholder substitution.
/// Otherwise, extracts text from common payload fields.
fn build_text_content(template: Option<&str>, payload: &JsonValue) -> Option<String> {
    if let Some(tmpl) = template {
        // Simple template substitution (basic Handlebars-like)
        // For full Handlebars support, integrate with templating.handlebars component
        let text = substitute_placeholders(tmpl, payload);
        Some(text)
    } else {
        // Try to extract text from common payload fields
        extract_text_from_payload(payload)
    }
}

/// Performs simple placeholder substitution in the template.
/// Supports {{field}} and {{input.field}} patterns.
fn substitute_placeholders(template: &str, payload: &JsonValue) -> String {
    let mut result = template.to_string();

    // Find all {{...}} patterns
    let re_pattern = r"\{\{([^}]+)\}\}";
    let captures: Vec<(String, String)> = regex_lite_find_all(&result, re_pattern)
        .into_iter()
        .map(|(full_match, path)| {
            let value = resolve_json_path(payload, &path);
            (full_match, value)
        })
        .collect();

    for (pattern, value) in captures {
        result = result.replace(&pattern, &value);
    }

    result
}

/// Simple regex-like pattern matching for {{...}}.
fn regex_lite_find_all(text: &str, _pattern: &str) -> Vec<(String, String)> {
    let mut results = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("{{") {
        if let Some(end) = remaining[start..].find("}}") {
            let full_match = &remaining[start..start + end + 2];
            let path = &remaining[start + 2..start + end];
            results.push((full_match.to_string(), path.trim().to_string()));
            remaining = &remaining[start + end + 2..];
        } else {
            break;
        }
    }

    results
}

/// Resolves a JSON path like "input.payload.message" or "payload.text".
fn resolve_json_path(value: &JsonValue, path: &str) -> String {
    let parts: Vec<&str> = path.split('.').collect();

    // Skip "input." prefix if present (we already have the payload)
    let effective_parts = if !parts.is_empty() && parts[0] == "input" {
        &parts[1..]
    } else {
        &parts[..]
    };

    let mut current = value;
    for part in effective_parts {
        current = match current.get(part) {
            Some(v) => v,
            None => return String::new(),
        };
    }

    match current {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => String::new(),
        _ => current.to_string(),
    }
}

/// Extracts text from common payload fields.
fn extract_text_from_payload(payload: &JsonValue) -> Option<String> {
    // Try common field names
    for field in &["text", "message", "body", "content", "description"] {
        if let Some(JsonValue::String(s)) = payload.get(*field) {
            return Some(s.clone());
        }
    }

    // If payload is a string directly
    if let JsonValue::String(s) = payload {
        return Some(s.clone());
    }

    // Fallback: serialize payload as JSON string
    Some(payload.to_string())
}

/// Generates a UUID v4 string.
fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_session_from_topic() {
        assert_eq!(
            extract_session_from_topic("greentic.events.alerts.critical"),
            "critical"
        );
        assert_eq!(extract_session_from_topic("single"), "single");
    }

    #[test]
    fn test_substitute_placeholders() {
        let payload = serde_json::json!({
            "message": "Hello",
            "user": { "name": "Alice" }
        });
        let template = "Message: {{message}}, User: {{user.name}}";
        let result = substitute_placeholders(template, &payload);
        assert_eq!(result, "Message: Hello, User: Alice");
    }

    #[test]
    fn test_extract_text_from_payload() {
        let payload = serde_json::json!({ "text": "Hello World" });
        assert_eq!(
            extract_text_from_payload(&payload),
            Some("Hello World".to_string())
        );

        let payload = serde_json::json!({ "message": "Hi there" });
        assert_eq!(
            extract_text_from_payload(&payload),
            Some("Hi there".to_string())
        );
    }
}
