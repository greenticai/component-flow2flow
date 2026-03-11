#!/usr/bin/env bash
# Setup E2E Test Bundle for flow2flow
# Creates the necessary directory structure and test files

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
ROOT_DIR="$(dirname "$PROJECT_DIR")"
E2E_BUNDLE_DIR="$ROOT_DIR/e2e-test-bundle"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }

setup_directories() {
    log_info "Creating directory structure..."
    mkdir -p "$E2E_BUNDLE_DIR"/{packs,flows,tenants/test/teams/default,state,logs}
    log_success "Directories created"
}

create_test_flow() {
    log_info "Creating flow2flow test flow..."

    cat > "$E2E_BUNDLE_DIR/flows/e2e_event2msg_test.ygtc" << 'EOF'
id: e2e_event2msg_test
title: E2E Test - Event to Message
description: End-to-end test for event2msg conversion
type: messaging
start: simulate_event

parameters:
  tenant_id: "test"
  team_id: "default"

nodes:
  simulate_event:
    templating.handlebars:
      text: |
        {
          "id": "evt-{{uuid}}",
          "topic": "e2e.test.alerts",
          "type": "greentic.alert.v1",
          "source": "e2e-test-runner",
          "tenant": {
            "tenant_id": "{{parameters.tenant_id}}",
            "env_id": "dev"
          },
          "payload": {
            "message": "E2E Test: {{input.message | default: 'Hello from E2E'}}"
          }
        }
    routing:
      - to: convert

  convert:
    flow2flow.event2msg:
      op: convert
      event: "{{simulate_event}}"
      config:
        target_channel: "dummy"
        destination:
          id: "e2e-test-user"
        text_template: "Alert: {{payload.message}}"
    routing:
      - to: verify

  verify:
    templating.handlebars:
      text: |
        {
          "status": "success",
          "message_id": "{{convert.id}}",
          "channel": "{{convert.channel}}",
          "text": "{{convert.text}}"
        }
    routing:
      - out: true
EOF

    log_success "Test flow created: flows/e2e_event2msg_test.ygtc"
}

create_tenant_config() {
    log_info "Creating tenant configuration..."

    cat > "$E2E_BUNDLE_DIR/tenants/test/tenant.gmap" << 'EOF'
{
  "tenant_id": "test",
  "display_name": "E2E Test Tenant",
  "environment": "dev",
  "features": {
    "flow2flow": true
  }
}
EOF

    cat > "$E2E_BUNDLE_DIR/tenants/test/teams/default/team.gmap" << 'EOF'
{
  "team_id": "default",
  "display_name": "Default Team"
}
EOF

    log_success "Tenant configuration created"
}

create_demo_config() {
    log_info "Creating greentic.demo.yaml..."

    # Only create if doesn't exist
    if [ ! -f "$E2E_BUNDLE_DIR/greentic.demo.yaml" ]; then
        cat > "$E2E_BUNDLE_DIR/greentic.demo.yaml" << 'EOF'
version: "1"
project_root: "./"
tenant: "test"
team: "default"
environment: "dev"

services:
  nats:
    enabled: true
    spawn:
      enabled: true
      port: 4222

logging:
  level: "info"
  format: "pretty"

http:
  host: "127.0.0.1"
  port: 8080

packs:
  flow2flow:
    path: "packs/flow2flow.gtpack"
    enabled: true
EOF
        log_success "greentic.demo.yaml created"
    else
        log_info "greentic.demo.yaml already exists, skipping"
    fi
}

main() {
    log_info "Setting up E2E Test Bundle for flow2flow..."

    setup_directories
    create_test_flow
    create_tenant_config
    create_demo_config

    echo ""
    log_success "E2E Test Bundle setup complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Build:   ./scripts/e2e.sh build"
    echo "  2. Pack:    ./scripts/e2e.sh pack"
    echo "  3. Deploy:  ./scripts/e2e.sh deploy"
    echo "  4. Run:     ./scripts/e2e.sh e2e"
    echo ""
}

main
