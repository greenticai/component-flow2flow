//! Configuration types for event2msg component.

use serde::{Deserialize, Serialize};

/// Configuration for converting an event to a message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event2MsgConfig {
    /// Target messaging channel (e.g., "telegram", "whatsapp", "slack").
    pub target_channel: String,

    /// Destination for the outbound message.
    pub destination: Destination,

    /// Optional Handlebars template for generating message text from event payload.
    /// If not provided, uses the event payload's "text" or "message" field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_template: Option<String>,
}

/// Destination specification for the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Destination {
    /// Destination identifier (provider specific; may be phone number, user id, channel id).
    pub id: String,

    /// Optional destination kind (e.g., "phone", "user", "channel").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Input structure for the convert operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConvertInput {
    /// The source event envelope.
    pub event: EventInput,

    /// Conversion configuration.
    pub config: Event2MsgConfig,
}

/// Simplified EventEnvelope for input parsing.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventInput {
    /// Stable identifier for the event.
    pub id: String,

    /// Logical topic for routing.
    pub topic: String,

    /// Fully qualified event type identifier.
    #[serde(rename = "type")]
    pub event_type: String,

    /// Originator of the event.
    pub source: String,

    /// Tenant context.
    pub tenant: TenantInput,

    /// Optional subject tied to the event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,

    /// Event timestamp in RFC3339 format.
    pub time: String,

    /// Optional correlation identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Event payload as JSON value.
    pub payload: serde_json::Value,

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

/// Output structure matching ChannelMessageEnvelope.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MessageOutput {
    /// Stable identifier for the message.
    pub id: String,

    /// Tenant context propagated with the message.
    pub tenant: TenantOutput,

    /// Abstract channel identifier or type.
    pub channel: String,

    /// Conversation or thread identifier.
    pub session_id: String,

    /// Optional reply scope for resumption.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_scope: Option<serde_json::Value>,

    /// Optional actor (sender/initiator).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<ActorOutput>,

    /// Outbound destinations for egress.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub to: Vec<DestinationOutput>,

    /// Optional correlation identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,

    /// Optional text content.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Attachments included with the message.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<AttachmentOutput>,

    /// Free-form metadata for adapters and flows.
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

    #[serde(default)]
    pub trace_id: String,

    #[serde(default)]
    pub correlation_id: String,
}

/// Actor output for the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorOutput {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Destination output for the message.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DestinationOutput {
    pub id: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Attachment output.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachmentOutput {
    pub mime_type: String,
    pub url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}
