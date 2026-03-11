//! E2E tests for flow2flow components.
//!
//! Tests the full conversion pipeline from input CBOR to output CBOR.

#[cfg(test)]
mod tests {
    use greentic_types::cbor::canonical::{from_cbor, to_canonical_cbor_allow_floats};
    use serde_json::json;

    /// Test event2msg full conversion pipeline
    #[test]
    fn test_event2msg_e2e() {
        // Simulate EventEnvelope input with proper structure
        let input = json!({
            "event": {
                "id": "evt-123",
                "topic": "alerts.critical",
                "type": "greentic.alert.v1",
                "source": "monitoring-service",
                "tenant": {
                    "tenant_id": "demo",
                    "team_id": "default",
                    "env_id": "prod"
                },
                "time": "2026-03-11T10:00:00Z",
                "correlation_id": "corr-456",
                "payload": {
                    "text": "Server CPU at 95%",
                    "severity": "critical"
                },
                "metadata": {
                    "phone": "+1234567890"
                }
            },
            "config": {
                "target_channel": "telegram",
                "destination": {
                    "id": "+1234567890",
                    "kind": "phone"
                }
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = event2msg::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Output is MessageOutput directly (not wrapped in "message")
        assert!(
            output.get("id").is_some(),
            "Output should have 'id' field: {:?}",
            output
        );
        assert_eq!(output["tenant"]["tenant_id"], "demo");
        assert_eq!(output["channel"], "telegram");
        assert!(output["text"]
            .as_str()
            .unwrap()
            .contains("Server CPU at 95%"));
    }

    /// Test msg2event full conversion pipeline
    #[test]
    fn test_msg2event_e2e() {
        // Simulate ChannelMessageEnvelope input with proper structure
        let input = json!({
            "message": {
                "id": "msg-789",
                "tenant": {
                    "tenant_id": "demo",
                    "team_id": "default",
                    "env_id": "prod"
                },
                "channel": "telegram",
                "session_id": "sess-abc",
                "correlation_id": "corr-xyz",
                "from": {
                    "id": "user123",
                    "kind": "user"
                },
                "text": "/webhook trigger-deploy",
                "metadata": {}
            },
            "config": {
                "topic": "commands.deploy",
                "event_type": "greentic.command.v1"
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = msg2event::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Output is EventOutput directly (not wrapped in "event")
        assert!(
            output.get("id").is_some(),
            "Output should have 'id' field: {:?}",
            output
        );
        assert_eq!(output["tenant"]["tenant_id"], "demo");
        assert_eq!(output["topic"], "commands.deploy");
        assert_eq!(output["type"], "greentic.command.v1");
        assert_eq!(output["correlation_id"], "corr-xyz");
    }

    /// Test bidirectional conversion (event -> msg -> event)
    #[test]
    fn test_bidirectional_conversion() {
        // Start with an event
        let original_event = json!({
            "event": {
                "id": "evt-round-trip",
                "topic": "test.roundtrip",
                "type": "greentic.test.v1",
                "source": "test-source",
                "tenant": {
                    "tenant_id": "test-tenant",
                    "team_id": "test-team",
                    "env_id": "test"
                },
                "time": "2026-03-11T10:00:00Z",
                "correlation_id": "round-trip-123",
                "payload": {
                    "text": "Round trip test message"
                },
                "metadata": {}
            },
            "config": {
                "target_channel": "test-channel",
                "destination": {
                    "id": "test-dest",
                    "kind": "test"
                }
            }
        });

        // Convert event to message
        let event_cbor = to_canonical_cbor_allow_floats(&original_event).unwrap();
        let msg_cbor = event2msg::convert::run(event_cbor);
        let msg_output: serde_json::Value = from_cbor(&msg_cbor).unwrap();

        // Verify message was created (output is MessageOutput directly)
        assert!(
            msg_output.get("id").is_some(),
            "Should create message: {:?}",
            msg_output
        );
        assert_eq!(msg_output["tenant"]["tenant_id"], "test-tenant");
        assert_eq!(msg_output["channel"], "test-channel");

        // Build message input for reverse conversion
        let msg_input = json!({
            "message": {
                "id": msg_output["id"],
                "tenant": msg_output["tenant"],
                "channel": msg_output["channel"],
                "session_id": msg_output["session_id"],
                "text": msg_output["text"],
                "metadata": msg_output.get("metadata").cloned().unwrap_or(json!({}))
            },
            "config": {
                "topic": "test.roundtrip.reply",
                "event_type": "greentic.test.reply.v1"
            }
        });

        let msg_input_cbor = to_canonical_cbor_allow_floats(&msg_input).unwrap();
        let event_cbor = msg2event::convert::run(msg_input_cbor);
        let event_output: serde_json::Value = from_cbor(&event_cbor).unwrap();

        // Verify event was created (output is EventOutput directly)
        assert!(
            event_output.get("id").is_some(),
            "Should create event: {:?}",
            event_output
        );
        assert_eq!(event_output["tenant"]["tenant_id"], "test-tenant");
        assert_eq!(event_output["topic"], "test.roundtrip.reply");
        assert_eq!(event_output["type"], "greentic.test.reply.v1");
    }

    /// Test error handling for invalid input
    #[test]
    fn test_event2msg_invalid_input() {
        let invalid_input = json!({
            "invalid": "data"
        });

        let input_cbor = to_canonical_cbor_allow_floats(&invalid_input).unwrap();
        let output_cbor = event2msg::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Should return error
        assert!(
            output.get("error").is_some(),
            "Should return error for invalid input: {:?}",
            output
        );
    }

    /// Test error handling for invalid message input
    #[test]
    fn test_msg2event_invalid_input() {
        let invalid_input = json!({
            "invalid": "data"
        });

        let input_cbor = to_canonical_cbor_allow_floats(&invalid_input).unwrap();
        let output_cbor = msg2event::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Should return error
        assert!(
            output.get("error").is_some(),
            "Should return error for invalid input: {:?}",
            output
        );
    }

    /// Test event2msg with text template
    #[test]
    fn test_event2msg_with_template() {
        let input = json!({
            "event": {
                "id": "evt-tmpl",
                "topic": "alerts.server",
                "type": "greentic.alert.v1",
                "source": "monitoring",
                "tenant": {
                    "tenant_id": "demo",
                    "env_id": "prod"
                },
                "time": "2026-03-11T10:00:00Z",
                "payload": {
                    "server": "web-01",
                    "cpu": 95,
                    "memory": 80
                },
                "metadata": {}
            },
            "config": {
                "target_channel": "slack",
                "destination": {
                    "id": "#alerts"
                },
                "text_template": "Server {{server}} - CPU: {{cpu}}%, Memory: {{memory}}%"
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = event2msg::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert!(output.get("id").is_some());
        let text = output["text"].as_str().unwrap();
        assert!(text.contains("web-01"));
        assert!(text.contains("95"));
        assert!(text.contains("80"));
    }

    /// Test metadata propagation from event to message
    #[test]
    fn test_event2msg_metadata_propagation() {
        let input = json!({
            "event": {
                "id": "evt-meta",
                "topic": "test.metadata",
                "type": "greentic.test.v1",
                "source": "test-source",
                "tenant": {
                    "tenant_id": "demo",
                    "env_id": "prod"
                },
                "time": "2026-03-11T10:00:00Z",
                "payload": { "text": "Hello" },
                "metadata": {
                    "custom_key": "custom_value",
                    "another_key": "another_value"
                }
            },
            "config": {
                "target_channel": "test",
                "destination": { "id": "test" }
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = event2msg::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        // Verify metadata is propagated
        let metadata = &output["metadata"];
        assert_eq!(metadata["custom_key"], "custom_value");
        assert_eq!(metadata["another_key"], "another_value");
        // Also should have event tracking metadata
        assert_eq!(metadata["event_id"], "evt-meta");
        assert_eq!(metadata["event_type"], "greentic.test.v1");
    }

    /// Test msg2event with custom source
    #[test]
    fn test_msg2event_custom_source() {
        let input = json!({
            "message": {
                "id": "msg-src",
                "tenant": {
                    "tenant_id": "demo",
                    "env_id": "prod"
                },
                "channel": "slack",
                "session_id": "sess-123",
                "text": "test message",
                "metadata": {}
            },
            "config": {
                "topic": "custom.topic",
                "event_type": "greentic.custom.v1",
                "source": "my-custom-source"
            }
        });

        let input_cbor = to_canonical_cbor_allow_floats(&input).unwrap();
        let output_cbor = msg2event::convert::run(input_cbor);
        let output: serde_json::Value = from_cbor(&output_cbor).unwrap();

        assert_eq!(output["source"], "my-custom-source");
    }
}
