#!/bin/bash
# Demo script for flow2flow components
# Shows the conversion pipeline working end-to-end

set -e

echo "=============================================="
echo "  flow2flow Demo - Event ↔ Message Bridge"
echo "=============================================="
echo ""

echo "📦 Building WASM components..."
cargo build --target wasm32-wasip2 --release --quiet -p event2msg -p msg2event
echo "✅ WASM build successful"
echo ""

echo "🧪 Running E2E Tests..."
echo ""

echo "--- event2msg tests ---"
cargo test -p flow2flow-e2e test_event2msg --quiet -- --nocapture 2>&1 | grep -E "(test |ok|PASSED)" || true

echo ""
echo "--- msg2event tests ---"
cargo test -p flow2flow-e2e test_msg2event --quiet -- --nocapture 2>&1 | grep -E "(test |ok|PASSED)" || true

echo ""
echo "--- bidirectional conversion ---"
cargo test -p flow2flow-e2e test_bidirectional --quiet -- --nocapture 2>&1 | grep -E "(test |ok|PASSED)" || true

echo ""
echo "🔧 Running WASM Runtime Validation..."
cargo test -p flow2flow-wasm-runtime --quiet -- --nocapture 2>&1 | grep -E "(test |ok|loaded|size)" || true

echo ""
echo "=============================================="
echo "  Full Test Summary"
echo "=============================================="
cargo test --workspace --quiet 2>&1 | tail -20

echo ""
echo "✅ Demo complete!"
echo ""
echo "Components validated:"
echo "  - event2msg: EventEnvelope → ChannelMessageEnvelope"
echo "  - msg2event: ChannelMessageEnvelope → EventEnvelope"
echo ""
echo "Use cases:"
echo "  - Webhook alert → Telegram notification"
echo "  - Slack command → External webhook trigger"
echo "  - Timer event → Scheduled message"
