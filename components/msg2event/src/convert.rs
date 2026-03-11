//! Conversion logic from ChannelMessageEnvelope to EventEnvelope.

use std::collections::BTreeMap;

use greentic_types::cbor::canonical;
use serde_json::Value as JsonValue;

use crate::config::{ConvertInput, EventOutput, TenantOutput};

/// Executes the conversion from message to event.
///
/// Takes CBOR-encoded input containing the message and configuration,
/// returns CBOR-encoded EventEnvelope.
pub fn run(input: Vec<u8>) -> Vec<u8> {
    let result = convert_message_to_event(&input);
    canonical::to_canonical_cbor_allow_floats(&result).unwrap_or_default()
}

fn convert_message_to_event(input: &[u8]) -> JsonValue {
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

    let message = &convert_input.message;
    let config = &convert_input.config;

    // Generate event ID from message ID
    let event_id = format!("evt-{}", message.id);

    // Determine source
    let source = config
        .source
        .clone()
        .unwrap_or_else(|| format!("greentic.messaging.{}", message.channel));

    // Build payload from message content
    let payload = build_event_payload(message);

    // Build tenant context
    let tenant = TenantOutput {
        tenant_id: message.tenant.tenant_id.clone(),
        team_id: message.tenant.team_id.clone(),
        user_id: message.tenant.user_id.clone(),
        env_id: message.tenant.env_id.clone(),
    };

    // Build subject from message actor
    let subject = message
        .from
        .as_ref()
        .map(|actor| format!("user:{}", actor.id));

    // Generate timestamp - placeholder, will be set by host runtime
    // In WASM, we don't have direct clock access
    let time = String::new();

    // Propagate metadata with message source info
    let mut metadata: BTreeMap<String, String> = message.metadata.clone();
    metadata.insert("message_id".to_string(), message.id.clone());
    metadata.insert("message_channel".to_string(), message.channel.clone());
    metadata.insert("message_session_id".to_string(), message.session_id.clone());
    if let Some(ref actor) = message.from {
        metadata.insert("message_from_id".to_string(), actor.id.clone());
        if let Some(ref kind) = actor.kind {
            metadata.insert("message_from_kind".to_string(), kind.clone());
        }
    }

    // Build output
    let output = EventOutput {
        id: event_id,
        topic: config.topic.clone(),
        event_type: config.event_type.clone(),
        source,
        tenant,
        subject,
        time,
        correlation_id: message.correlation_id.clone(),
        payload,
        metadata,
    };

    serde_json::to_value(output).unwrap_or_else(|_| serde_json::json!({}))
}

/// Builds the event payload from message content.
fn build_event_payload(message: &crate::config::MessageInput) -> JsonValue {
    let mut payload = serde_json::Map::new();

    // Include text content
    if let Some(ref text) = message.text {
        payload.insert("text".to_string(), JsonValue::String(text.clone()));
    }

    // Include channel info
    payload.insert(
        "channel".to_string(),
        JsonValue::String(message.channel.clone()),
    );

    // Include session info
    payload.insert(
        "session_id".to_string(),
        JsonValue::String(message.session_id.clone()),
    );

    // Include actor if present
    if let Some(ref actor) = message.from {
        let mut actor_obj = serde_json::Map::new();
        actor_obj.insert("id".to_string(), JsonValue::String(actor.id.clone()));
        if let Some(ref kind) = actor.kind {
            actor_obj.insert("kind".to_string(), JsonValue::String(kind.clone()));
        }
        payload.insert("from".to_string(), JsonValue::Object(actor_obj));
    }

    // Include destinations if present
    if !message.to.is_empty() {
        let destinations: Vec<JsonValue> = message
            .to
            .iter()
            .map(|dest| {
                let mut dest_obj = serde_json::Map::new();
                dest_obj.insert("id".to_string(), JsonValue::String(dest.id.clone()));
                if let Some(ref kind) = dest.kind {
                    dest_obj.insert("kind".to_string(), JsonValue::String(kind.clone()));
                }
                JsonValue::Object(dest_obj)
            })
            .collect();
        payload.insert("to".to_string(), JsonValue::Array(destinations));
    }

    // Include attachments if present
    if !message.attachments.is_empty() {
        let attachments: Vec<JsonValue> = message
            .attachments
            .iter()
            .map(|att| {
                let mut att_obj = serde_json::Map::new();
                att_obj.insert(
                    "mime_type".to_string(),
                    JsonValue::String(att.mime_type.clone()),
                );
                att_obj.insert("url".to_string(), JsonValue::String(att.url.clone()));
                if let Some(ref name) = att.name {
                    att_obj.insert("name".to_string(), JsonValue::String(name.clone()));
                }
                if let Some(size) = att.size_bytes {
                    att_obj.insert(
                        "size_bytes".to_string(),
                        JsonValue::Number(serde_json::Number::from(size)),
                    );
                }
                JsonValue::Object(att_obj)
            })
            .collect();
        payload.insert("attachments".to_string(), JsonValue::Array(attachments));
    }

    // Include reply scope if present
    if let Some(ref reply_scope) = message.reply_scope {
        payload.insert("reply_scope".to_string(), reply_scope.clone());
    }

    JsonValue::Object(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ActorInput, MessageInput, Msg2EventConfig, TenantInput};

    #[test]
    fn test_convert_simple_message() {
        let message = MessageInput {
            id: "msg-123".to_string(),
            tenant: TenantInput {
                tenant_id: "tenant1".to_string(),
                team_id: Some("team1".to_string()),
                user_id: None,
                env_id: "dev".to_string(),
            },
            channel: "slack".to_string(),
            session_id: "session-456".to_string(),
            reply_scope: None,
            from: Some(ActorInput {
                id: "U12345".to_string(),
                kind: Some("user".to_string()),
            }),
            to: vec![],
            correlation_id: Some("corr-789".to_string()),
            text: Some("Hello world".to_string()),
            attachments: vec![],
            metadata: BTreeMap::new(),
        };

        let config = Msg2EventConfig {
            topic: "integrations.slack.message".to_string(),
            event_type: "greentic.slack.message.v1".to_string(),
            source: None,
        };

        let input = ConvertInput { message, config };

        let input_cbor = canonical::to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = run(input_cbor);
        let output: JsonValue = canonical::from_cbor(&output_cbor).unwrap();

        assert_eq!(output["id"], "evt-msg-123");
        assert_eq!(output["topic"], "integrations.slack.message");
        assert_eq!(output["type"], "greentic.slack.message.v1");
        assert_eq!(output["source"], "greentic.messaging.slack");
        assert_eq!(output["payload"]["text"], "Hello world");
        assert_eq!(output["payload"]["channel"], "slack");
    }
}
