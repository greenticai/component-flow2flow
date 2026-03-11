//! Configuration types for msg2event component.

use serde::{Deserialize, Serialize};

/// Configuration for converting a message to an event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Msg2EventConfig {
    /// Target event topic (e.g., "integrations.slack.command").
    pub topic: String,

    /// Event type identifier (e.g., "greentic.slack.command.v1").
    pub event_type: String,

    /// Optional override for the event source.
    /// If not provided, uses the message channel as source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Input structure for the convert operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConvertInput {
    /// The source message envelope.
    pub message: MessageInput,

    /// Conversion configuration.
    pub config: Msg2EventConfig,
}

/// Simplified ChannelMessageEnvelope for input parsing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageInput {
    /// Stable identifier for the message.
    pub id: String,

    /// Tenant context propagated with the message.
    pub tenant: TenantInput,

    /// Abstract channel identifier or type.
    pub channel: String,

    /// Conversation or thread identifier.
    pub session_id: String,

    /// Optional reply scope for resumption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_scope: Option<serde_json::Value>,

    /// Optional actor (sender/initiator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<ActorInput>,

    /// Outbound destinations for egress.
    #[serde(default)]
    pub to: Vec<DestinationInput>,

    /// Optional correlation identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Optional text content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Attachments included with the message.
    #[serde(default)]
    pub attachments: Vec<AttachmentInput>,

    /// Free-form metadata.
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// Simplified TenantCtx for input parsing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantInput {
    pub tenant_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    #[serde(default)]
    pub env_id: String,
}

/// Actor input from the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorInput {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Destination input from the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DestinationInput {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Attachment input from the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentInput {
    pub mime_type: String,
    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

/// Output structure matching EventEnvelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventOutput {
    /// Stable identifier for the event.
    pub id: String,

    /// Logical topic for routing.
    pub topic: String,

    /// Fully qualified event type identifier.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Originator of the event.
    pub source: String,

    /// Tenant context propagated with the event.
    pub tenant: TenantOutput,

    /// Optional subject tied to the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Event timestamp in RFC3339 format.
    pub time: String,

    /// Optional correlation identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Event payload.
    pub payload: serde_json::Value,

    /// Free-form metadata.
    #[serde(default)]
    pub metadata: std::collections::BTreeMap<String, String>,
}

/// Tenant context for output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenantOutput {
    pub tenant_id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_id: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,

    pub env_id: String,
}
