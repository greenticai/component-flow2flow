# flow2flow Pack

Bridge components for cross-domain flow orchestration in Greentic.

## Overview

This pack provides two components for converting between messaging and events domains:

- **flow2flow.event2msg**: Converts `EventEnvelope` to `ChannelMessageEnvelope`
- **flow2flow.msg2event**: Converts `ChannelMessageEnvelope` to `EventEnvelope`

## Use Cases

### Alert Webhook to WhatsApp Notification
```yaml
# Event flow receives webhook alert, bridges to messaging
nodes:
  receive_alert:
    events.receive: {}
    routing:
      - to: bridge

  bridge:
    flow2flow.event2msg:
      event: "{{receive_alert}}"
      config:
        target_channel: "whatsapp"
        destination:
          id: "+1234567890"
          kind: "phone"
        text_template: "Alert: {{input.payload.message}}"
    routing:
      - to: send

  send:
    emit.response: {}
    routing:
      - out: true
```

### Slack Command to Webhook Event
```yaml
# Messaging flow receives command, bridges to events
nodes:
  receive_command:
    messaging.receive: {}
    routing:
      - to: bridge

  bridge:
    flow2flow.msg2event:
      message: "{{receive_command}}"
      config:
        topic: "integrations.slack.commands"
        event_type: "greentic.slack.command.v1"
    routing:
      - to: publish

  publish:
    events.emit:
      topic: "{{bridge.topic}}"
    routing:
      - out: true
```

### Timer Event to Scheduled Telegram Message
```yaml
# Timer event triggers scheduled notification
nodes:
  timer_trigger:
    events.receive: {}
    routing:
      - to: bridge

  bridge:
    flow2flow.event2msg:
      event: "{{timer_trigger}}"
      config:
        target_channel: "telegram"
        destination:
          id: "@channel_name"
          kind: "channel"
        text_template: |
          Daily Report:
          {{input.payload.report_summary}}
    routing:
      - to: send

  send:
    emit.response: {}
    routing:
      - out: true
```

## Type Conversion Mapping

### EventEnvelope to ChannelMessageEnvelope

| EventEnvelope | ChannelMessageEnvelope |
|---------------|------------------------|
| `id` | `id` (prefixed with "msg-") |
| `tenant` | `tenant` |
| `topic` (last segment) | `channel` |
| `source` | `from.id` |
| `correlation_id` | `correlation_id` |
| `payload.text` | `text` |
| `metadata` | `metadata` (with event info added) |

### ChannelMessageEnvelope to EventEnvelope

| ChannelMessageEnvelope | EventEnvelope |
|------------------------|---------------|
| `id` | `id` (prefixed with "evt-") |
| `tenant` | `tenant` |
| `channel` | `source` (prefixed) |
| `from.id` | `subject` (as "user:...") |
| `correlation_id` | `correlation_id` |
| `text` | `payload.text` |
| `metadata` | `metadata` (with message info added) |

## Installation

```bash
greentic-pack install flow2flow.gtpack
```

## License

MIT
