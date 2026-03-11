# flow2flow - Cross-Domain Flow Bridge

Bridge components for converting between EventEnvelope and ChannelMessageEnvelope, enabling cross-domain flow orchestration.

## Overview

flow2flow provides two WASM components:

| Component | Direction | Use Case |
|-----------|-----------|----------|
| `event2msg` | Event → Message | Webhook alerts → Telegram notification |
| `msg2event` | Message → Event | Slack command → External webhook |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Event Domain (webhooks, timers, integrations)               │
│     ↓                                                       │
│ EventEnvelope                                               │
│     ↓                                                       │
│ [flow2flow.event2msg] ← WASM component                      │
│     ↓                                                       │
│ ChannelMessageEnvelope                                      │
│     ↓                                                       │
│ Messaging Domain (Telegram, WhatsApp, Slack, Teams)         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ Messaging Domain (user commands, chat messages)             │
│     ↓                                                       │
│ ChannelMessageEnvelope                                      │
│     ↓                                                       │
│ [flow2flow.msg2event] ← WASM component                      │
│     ↓                                                       │
│ EventEnvelope                                               │
│     ↓                                                       │
│ Event Domain (publish to topic, trigger workflows)          │
└─────────────────────────────────────────────────────────────┘
```

## Installation

### Build from Source

```bash
# Prerequisites
cargo install cargo-component --locked

# Build WASM components
make wasm

# Build pack
make pack

# Output: dist/flow2flow.gtpack
```

### Deploy Pack

```bash
# Copy to demo bundle
cp dist/flow2flow.gtpack ~/my-bundle/packs/

# Or install via operator
gtc op pack install dist/flow2flow.gtpack
```

## Usage

### event2msg - Convert Event to Message

**Flow Definition:**
```yaml
nodes:
  convert:
    flow2flow.event2msg:
      op: convert
      event: "{{input}}"
      config:
        target_channel: "telegram"
        destination:
          id: "+1234567890"
          kind: "phone"
        text_template: "🚨 Alert: {{payload.message}}"
    routing:
      - to: send
```

**Input Schema:**
```json
{
  "event": {
    "id": "evt-001",
    "topic": "alerts.critical",
    "type": "greentic.alert.v1",
    "source": "monitoring-system",
    "tenant": {
      "tenant_id": "acme",
      "env_id": "prod"
    },
    "payload": {
      "message": "Server CPU > 90%",
      "severity": "critical"
    }
  },
  "config": {
    "target_channel": "telegram",
    "destination": {
      "id": "+1234567890"
    },
    "text_template": "{{payload.message}}"
  }
}
```

**Output:**
```json
{
  "id": "msg-evt-001",
  "tenant": { "tenant_id": "acme", "env_id": "prod" },
  "channel": "telegram",
  "session_id": "critical",
  "to": [{ "id": "+1234567890" }],
  "text": "🚨 Alert: Server CPU > 90%",
  "metadata": {
    "event_id": "evt-001",
    "event_type": "greentic.alert.v1",
    "event_topic": "alerts.critical"
  }
}
```

### msg2event - Convert Message to Event

**Flow Definition:**
```yaml
nodes:
  convert:
    flow2flow.msg2event:
      op: convert
      message: "{{input}}"
      config:
        topic: "integrations.slack.command"
        event_type: "greentic.slack.command.v1"
        source: "greentic.messaging.slack"
    routing:
      - to: publish
```

**Input Schema:**
```json
{
  "message": {
    "id": "msg-123",
    "tenant": { "tenant_id": "acme", "env_id": "prod" },
    "channel": "slack",
    "session_id": "C123456",
    "from": { "id": "U789", "kind": "user" },
    "text": "/deploy production",
    "metadata": {}
  },
  "config": {
    "topic": "integrations.slack.command",
    "event_type": "greentic.slack.command.v1"
  }
}
```

**Output:**
```json
{
  "id": "evt-msg-123",
  "topic": "integrations.slack.command",
  "type": "greentic.slack.command.v1",
  "source": "greentic.messaging.slack",
  "tenant": { "tenant_id": "acme", "env_id": "prod" },
  "subject": "user:U789",
  "payload": {
    "text": "/deploy production",
    "channel": "slack",
    "session_id": "C123456",
    "from": { "id": "U789", "kind": "user" }
  },
  "metadata": {
    "message_id": "msg-123",
    "message_channel": "slack"
  }
}
```

## Type Mappings

### EventEnvelope → ChannelMessageEnvelope

| EventEnvelope Field | ChannelMessageEnvelope Field |
|---------------------|------------------------------|
| `id` | `id` (prefixed with "msg-") |
| `tenant` | `tenant` |
| `topic` (last segment) | `session_id` |
| `source` | `from.id` |
| `correlation_id` | `correlation_id` |
| `payload.text` or template | `text` |
| `metadata` | `metadata` (augmented) |

### ChannelMessageEnvelope → EventEnvelope

| ChannelMessageEnvelope Field | EventEnvelope Field |
|------------------------------|---------------------|
| `id` | `id` (prefixed with "evt-") |
| `tenant` | `tenant` |
| `channel` | `source` suffix |
| `from.id` | `subject` (prefixed) |
| `text` + context | `payload` |
| `metadata` | `metadata` (augmented) |

## Configuration

### Default Pack Config

```yaml
# pack.yaml
id: flow2flow
version: 0.1.0
title: Flow to Flow Bridge

components:
  - id: event2msg
    path: components/event2msg.wasm
  - id: msg2event
    path: components/msg2event.wasm

flows:
  - path: flows/event2msg_default.ygtc
  - path: flows/msg2event_default.ygtc
```

## Testing

```bash
# Unit tests
cargo test --workspace

# E2E tests
../scripts/e2e-flow2flow.sh all
```

## Examples

### Alert Webhook → Telegram

```yaml
id: webhook_to_telegram
type: messaging
start: receive

nodes:
  receive:
    events.subscribe:
      topic: "webhooks.alerts.#"
    routing:
      - to: convert

  convert:
    flow2flow.event2msg:
      op: convert
      event: "{{receive}}"
      config:
        target_channel: "telegram"
        destination:
          id: "{{receive.metadata.alert_phone}}"
        text_template: |
          🚨 *Alert: {{payload.title}}*
          Severity: {{payload.severity}}
          Time: {{time}}
    routing:
      - to: send

  send:
    emit.response: {}
    routing:
      - out: true
```

### Slack Command → External API

```yaml
id: slack_deploy_command
type: events
start: receive

nodes:
  receive:
    # Triggered by /deploy command in Slack
    flow2flow.msg2event:
      op: convert
      message: "{{input}}"
      config:
        topic: "deployments.requests"
        event_type: "greentic.deploy.request.v1"
    routing:
      - to: publish

  publish:
    events.emit:
      topic: "{{receive.topic}}"
      payload: "{{receive.payload}}"
    routing:
      - out: true
```

## API Reference

See [API.md](./API.md) for complete API documentation.

## License

MIT
