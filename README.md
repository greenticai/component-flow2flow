# component-flow2flow

WASM components for bridging messaging and events domains in Greentic.

## Overview

This workspace provides two components:

- **event2msg**: Converts `EventEnvelope` to `ChannelMessageEnvelope`
- **msg2event**: Converts `ChannelMessageEnvelope` to `EventEnvelope`

These components enable cross-domain flow orchestration:
- Alert webhook (event) -> WhatsApp notification (messaging)
- Slack command (messaging) -> Trigger external webhook (event)
- Timer event -> Send scheduled Telegram message

## Architecture

```
EventEnvelope -> [flow2flow.event2msg] -> ChannelMessageEnvelope
ChannelMessageEnvelope -> [flow2flow.msg2event] -> EventEnvelope
```

## Building

```bash
# Build all components
make build

# Build WASM components
make wasm

# Run tests
make test
```

## Pack

The `packs/flow2flow` directory contains the pack manifest and default flows.

## License

MIT
