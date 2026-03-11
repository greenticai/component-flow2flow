# flow2flow Demo: Bidirectional Event ↔ Chat

Demo ini menunjukkan flow2flow bridging antara event domain dan messaging domain.

## Scenarios

### 1. Event → Chat (Alert Notification)
```
Webhook POST /events/alerts
    ↓
EventEnvelope
    ↓
[flow2flow.event2msg]
    ↓
ChannelMessageEnvelope
    ↓
Telegram: "🚨 Alert: Server CPU > 90%"
```

### 2. Chat → Event (Command Trigger)
```
Telegram: "/deploy production"
    ↓
ChannelMessageEnvelope
    ↓
[flow2flow.msg2event]
    ↓
EventEnvelope
    ↓
Webhook POST to external API
```

---

## Prerequisites

1. **Telegram Bot** - Create via @BotFather
2. **Public URL** - ngrok or cloudflare tunnel
3. **greentic-operator** - For running the demo

---

## Step-by-Step Setup

### Step 1: Create Telegram Bot

```bash
# 1. Open Telegram, search @BotFather
# 2. Send: /newbot
# 3. Follow prompts, get your BOT_TOKEN
# 4. Save token for later
```

### Step 2: Setup Public URL

```bash
# Option A: ngrok
ngrok http 8080
# Copy the https URL, e.g., https://abc123.ngrok.io

# Option B: cloudflare tunnel (if installed)
cloudflared tunnel --url http://localhost:8080
```

### Step 3: Create Demo Bundle

```bash
# From greentic root
cd /path/to/greentic

# Create demo directory
mkdir -p demo-flow2flow/{packs,flows,tenants/demo/teams/default}
cd demo-flow2flow
```

### Step 4: Create Configuration

Create `greentic.demo.yaml`:

```yaml
version: "1"
project_root: "./"
tenant: "demo"
team: "default"
environment: "dev"

services:
  nats:
    enabled: true
    spawn:
      enabled: true
      port: 4222

logging:
  level: "debug"
  format: "pretty"

http:
  host: "0.0.0.0"
  port: 8080

packs:
  flow2flow:
    path: "packs/flow2flow.gtpack"
    enabled: true

  messaging-telegram:
    path: "packs/messaging-telegram.gtpack"
    enabled: true

providers:
  messaging:
    telegram:
      enabled: true
```

### Step 5: Create Secrets

Create `.greentic/dev/.secrets.env`:

```bash
mkdir -p .greentic/dev

cat > .greentic/dev/.secrets.env << 'EOF'
TELEGRAM_BOT_TOKEN=your_bot_token_here
PUBLIC_BASE_URL=https://your-ngrok-url.ngrok.io
EOF
```

### Step 6: Copy Packs

```bash
# Build and copy flow2flow
cd ../component-flow2flow
./scripts/e2e.sh build pack
cp dist/flow2flow.gtpack ../demo-flow2flow/packs/

# Copy telegram provider (from messaging-providers)
cp ../greentic-messaging-providers/packs/messaging-telegram.gtpack ../demo-flow2flow/packs/
```

### Step 7: Create Demo Flows

See `flows/` directory in this demo folder.

### Step 8: Start Demo

```bash
cd demo-flow2flow
gtc op demo up
```

### Step 9: Test Event → Chat

```bash
# Trigger alert via webhook
curl -X POST http://localhost:8080/events/alerts \
  -H "Content-Type: application/json" \
  -d '{
    "id": "alert-001",
    "type": "greentic.alert.v1",
    "source": "monitoring",
    "payload": {
      "severity": "critical",
      "message": "Server CPU > 90%",
      "host": "prod-server-01"
    }
  }'

# Check Telegram - should receive alert message!
```

### Step 10: Test Chat → Event

```bash
# In Telegram, send to your bot:
/deploy production

# Check logs - should see event emitted to deployments topic
gtc op demo logs --follow
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Demo Architecture                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │   Webhook    │────▶│  event2msg   │────▶│   Telegram   │    │
│  │   /events/*  │     │  (convert)   │     │   Provider   │    │
│  └──────────────┘     └──────────────┘     └──────────────┘    │
│                                                   │              │
│                                                   ▼              │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────┐    │
│  │   External   │◀────│  msg2event   │◀────│   Telegram   │    │
│  │   Webhook    │     │  (convert)   │     │   Ingress    │    │
│  └──────────────┘     └──────────────┘     └──────────────┘    │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Troubleshooting

### Telegram not receiving messages
- Check bot token is correct
- Verify webhook URL is accessible: `curl https://your-url/health`
- Check logs: `gtc op demo logs`

### Webhook not triggering
- Ensure operator is running on port 8080
- Check NATS is connected
- Verify flow is loaded: `gtc op demo flows`

### Event not emitted
- Check flow routing configuration
- Verify msg2event conversion logic
- Check topic subscription

---

## Next Steps

1. Add more event types (timer, email, SMS)
2. Add fast2flow for intelligent routing
3. Create custom Adaptive Cards for rich messages
4. Add authentication/authorization
