#!/usr/bin/env bash
# Setup flow2flow Demo Bundle
# Creates a complete demo environment for testing event↔chat bridging

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$PROJECT_DIR")"
DEMO_BUNDLE_DIR="$ROOT_DIR/demo-flow2flow"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_step() { echo -e "${CYAN}[STEP]${NC} $1"; }

print_banner() {
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║           flow2flow Demo Setup                            ║"
    echo "║     Bidirectional Event ↔ Chat Bridging                   ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

create_directories() {
    log_step "Creating directory structure..."

    mkdir -p "$DEMO_BUNDLE_DIR"/{packs,flows,tenants/demo/teams/default,.greentic/dev}

    log_success "Directories created: $DEMO_BUNDLE_DIR"
}

copy_flows() {
    log_step "Copying demo flows..."

    cp "$SCRIPT_DIR/flows/"*.ygtc "$DEMO_BUNDLE_DIR/flows/" 2>/dev/null || {
        log_warn "No flows to copy, creating from template..."
    }

    # Create a simple test flow if none exist
    if [ ! -f "$DEMO_BUNDLE_DIR/flows/bidirectional_demo.ygtc" ]; then
        cp "$SCRIPT_DIR/flows/bidirectional_demo.ygtc" "$DEMO_BUNDLE_DIR/flows/" 2>/dev/null || true
    fi

    log_success "Flows copied"
}

create_config() {
    log_step "Creating greentic.demo.yaml..."

    cat > "$DEMO_BUNDLE_DIR/greentic.demo.yaml" << 'EOF'
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

  redis:
    enabled: false

logging:
  level: "debug"
  format: "pretty"

telemetry:
  enabled: false

http:
  host: "0.0.0.0"
  port: 8080

packs:
  # Core flow2flow pack
  flow2flow:
    path: "packs/flow2flow.gtpack"
    enabled: true

  # Messaging provider (choose one)
  # messaging-telegram:
  #   path: "packs/messaging-telegram.gtpack"
  #   enabled: true

  # Dummy provider for testing without external services
  messaging-dummy:
    path: "packs/messaging-dummy.gtpack"
    enabled: true

# Provider configuration
providers:
  messaging:
    dummy:
      enabled: true

    # Uncomment for Telegram
    # telegram:
    #   enabled: true
    #   bot_token: "{{env.TELEGRAM_BOT_TOKEN}}"

# Demo-specific settings
demo:
  # Target chat for alerts
  alert_chat_id: "{{env.DEMO_TELEGRAM_CHAT_ID}}"

  # Webhook for deploy commands
  deploy_webhook_url: "https://httpbin.org/post"
EOF

    log_success "greentic.demo.yaml created"
}

create_secrets_template() {
    log_step "Creating secrets template..."

    cat > "$DEMO_BUNDLE_DIR/.greentic/dev/.secrets.env.example" << 'EOF'
# flow2flow Demo Secrets
# Copy this to .secrets.env and fill in real values

# For Telegram integration (optional)
TELEGRAM_BOT_TOKEN=your_bot_token_from_botfather
DEMO_TELEGRAM_CHAT_ID=your_chat_id

# Public URL for webhooks (ngrok/cloudflare)
PUBLIC_BASE_URL=https://your-tunnel-url.ngrok.io

# External webhook for /deploy command
DEPLOY_WEBHOOK_URL=https://httpbin.org/post
EOF

    # Create empty secrets file if not exists
    if [ ! -f "$DEMO_BUNDLE_DIR/.greentic/dev/.secrets.env" ]; then
        cat > "$DEMO_BUNDLE_DIR/.greentic/dev/.secrets.env" << 'EOF'
# Demo secrets (dummy mode - no real secrets needed)
DEPLOY_WEBHOOK_URL=https://httpbin.org/post
EOF
    fi

    log_success "Secrets template created"
}

create_tenant_config() {
    log_step "Creating tenant configuration..."

    cat > "$DEMO_BUNDLE_DIR/tenants/demo/tenant.gmap" << 'EOF'
{
  "tenant_id": "demo",
  "display_name": "flow2flow Demo",
  "environment": "dev",
  "features": {
    "flow2flow": true,
    "events": true,
    "messaging": true
  }
}
EOF

    cat > "$DEMO_BUNDLE_DIR/tenants/demo/teams/default/team.gmap" << 'EOF'
{
  "team_id": "default",
  "display_name": "Default Team"
}
EOF

    log_success "Tenant configuration created"
}

build_and_copy_pack() {
    log_step "Building flow2flow pack..."

    cd "$PROJECT_DIR"

    # Build components if not built
    if [ ! -f "target/wasm32-wasip2/release/flow2flow_event2msg.wasm" ]; then
        log_info "Building WASM components..."
        ./scripts/e2e.sh build 2>/dev/null || {
            log_warn "Build failed or scripts not available, skipping..."
        }
    fi

    # Build pack if tools available
    if command -v greentic-pack &> /dev/null; then
        ./scripts/e2e.sh pack 2>/dev/null || true
    fi

    # Copy pack if exists
    if [ -f "$PROJECT_DIR/dist/flow2flow.gtpack" ]; then
        cp "$PROJECT_DIR/dist/flow2flow.gtpack" "$DEMO_BUNDLE_DIR/packs/"
        log_success "flow2flow.gtpack copied"
    else
        log_warn "flow2flow.gtpack not found - build it first with ./scripts/e2e.sh build pack"
    fi

    # Try to copy dummy provider from messaging-providers
    local DUMMY_PACK="$ROOT_DIR/greentic-messaging-providers/packs/messaging-dummy.gtpack"
    if [ -f "$DUMMY_PACK" ]; then
        cp "$DUMMY_PACK" "$DEMO_BUNDLE_DIR/packs/"
        log_success "messaging-dummy.gtpack copied"
    fi
}

create_test_script() {
    log_step "Creating test script..."

    cat > "$DEMO_BUNDLE_DIR/test.sh" << 'EOF'
#!/usr/bin/env bash
# Quick test script for flow2flow demo

set -euo pipefail

BLUE='\033[0;34m'
GREEN='\033[0;32m'
NC='\033[0m'

echo -e "${BLUE}[TEST]${NC} Testing Event → Chat..."
curl -s -X POST http://localhost:8080/events/test \
  -H "Content-Type: application/json" \
  -d '{
    "id": "test-event-001",
    "topic": "demo.alerts",
    "type": "greentic.alert.v1",
    "source": "test-script",
    "tenant": {"tenant_id": "demo", "env_id": "dev"},
    "payload": {
      "severity": "warning",
      "message": "This is a test alert",
      "host": "localhost"
    },
    "metadata": {
      "target_channel": "dummy",
      "destination_id": "test-user"
    }
  }' | jq .

echo ""
echo -e "${GREEN}[OK]${NC} Event sent! Check operator logs for result."
echo ""
echo "To test Chat → Event, send a message via the messaging provider."
EOF

    chmod +x "$DEMO_BUNDLE_DIR/test.sh"

    log_success "Test script created: test.sh"
}

print_next_steps() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║                    Setup Complete!                        ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo "Demo bundle created at: $DEMO_BUNDLE_DIR"
    echo ""
    echo -e "${CYAN}Next Steps:${NC}"
    echo ""
    echo "  1. Build flow2flow pack (if not done):"
    echo "     cd $PROJECT_DIR"
    echo "     ./scripts/e2e.sh build pack"
    echo ""
    echo "  2. Start the demo:"
    echo "     cd $DEMO_BUNDLE_DIR"
    echo "     gtc op demo up"
    echo ""
    echo "  3. Test Event → Chat:"
    echo "     ./test.sh"
    echo ""
    echo "  4. (Optional) Enable Telegram:"
    echo "     - Edit .greentic/dev/.secrets.env"
    echo "     - Uncomment telegram in greentic.demo.yaml"
    echo "     - Restart operator"
    echo ""
    echo -e "${CYAN}Architecture:${NC}"
    echo ""
    echo "  Webhook POST → [event2msg] → Telegram/Dummy"
    echo "  Telegram msg → [msg2event] → NATS topic → Webhook"
    echo ""
}

main() {
    print_banner

    create_directories
    copy_flows
    create_config
    create_secrets_template
    create_tenant_config
    build_and_copy_pack
    create_test_script

    print_next_steps
}

main "$@"
