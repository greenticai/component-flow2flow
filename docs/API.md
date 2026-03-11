# flow2flow API Reference

## Components

### flow2flow.event2msg

Converts EventEnvelope to ChannelMessageEnvelope.

#### Operations

##### `convert`

Convert an event to a message envelope.

**Input:**
```typescript
interface ConvertInput {
  event: EventEnvelope;
  config: Event2MsgConfig;
}

interface EventEnvelope {
  id: string;
  topic: string;
  type: string;
  source: string;
  tenant: TenantCtx;
  subject?: string;
  time?: string;
  correlation_id?: string;
  payload: any;
  metadata: Record<string, string>;
}

interface Event2MsgConfig {
  target_channel: string;
  destination: {
    id: string;
    kind?: string;
  };
  text_template?: string;
}
```

**Output:**
```typescript
interface ChannelMessageEnvelope {
  id: string;
  tenant: TenantCtx;
  channel: string;
  session_id: string;
  reply_scope?: ReplyScope;
  from?: Actor;
  to: Destination[];
  correlation_id?: string;
  text?: string;
  attachments: Attachment[];
  metadata: Record<string, string>;
}
```

**Example:**
```yaml
flow2flow.event2msg:
  op: convert
  event: "{{input.event}}"
  config:
    target_channel: "telegram"
    destination:
      id: "+1234567890"
    text_template: "Alert: {{payload.message}}"
```

---

### flow2flow.msg2event

Converts ChannelMessageEnvelope to EventEnvelope.

#### Operations

##### `convert`

Convert a message to an event envelope.

**Input:**
```typescript
interface ConvertInput {
  message: ChannelMessageEnvelope;
  config: Msg2EventConfig;
}

interface Msg2EventConfig {
  topic: string;
  event_type: string;
  source?: string;
}
```

**Output:**
```typescript
interface EventEnvelope {
  id: string;
  topic: string;
  type: string;
  source: string;
  tenant: TenantCtx;
  subject?: string;
  time: string;
  correlation_id?: string;
  payload: MessagePayload;
  metadata: Record<string, string>;
}
```

**Example:**
```yaml
flow2flow.msg2event:
  op: convert
  message: "{{input.message}}"
  config:
    topic: "commands.slack"
    event_type: "greentic.command.v1"
```

---

## Schemas

### TenantCtx

```typescript
interface TenantCtx {
  tenant_id: string;
  team_id?: string;
  user_id?: string;
  env_id: string;
}
```

### Actor

```typescript
interface Actor {
  id: string;
  kind?: string;  // "user", "bot", "system"
}
```

### Destination

```typescript
interface Destination {
  id: string;
  kind?: string;  // "phone", "email", "user_id", "channel"
}
```

### Attachment

```typescript
interface Attachment {
  mime_type: string;
  url: string;
  name?: string;
  size_bytes?: number;
}
```

---

## Error Handling

Both components return errors in the output when conversion fails:

```json
{
  "error": "failed to parse input: invalid JSON"
}
```

Common errors:
- `failed to parse input` - Invalid CBOR/JSON input
- `invalid input structure` - Missing required fields
- `template error` - Invalid text template syntax

---

## Template Syntax

The `text_template` field supports simple placeholder substitution:

```
{{field}}              - Access top-level field
{{payload.message}}    - Access nested field
{{input.payload.x}}    - Access with input prefix (normalized)
```

For full Handlebars support, chain with `templating.handlebars` component.
