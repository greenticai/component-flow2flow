# Flow-to-Flow Overview

Flow-to-Flow (F2F) is a composition layer for assistant and automation runtimes. It sits one level above component invocation, providing typed contracts, tenancy-aware resolution, and runtime semantics such as retries, joins, and fallbacks. The goal is to make flows a first-class artifact that can be published, discovered, and executed in a consistent way across teams.

Key ideas:

- **Typed contracts**: every flow declares inbound parameters, outbound returns, and validator metadata. This keeps flows honest and tooling-friendly.
- **Named-map syntax**: nodes are keyed maps (`entry`, `fetch`, `respond`) instead of positional arrays; this improves readability and enables targeted overrides.
- **Tenancy-aware publishing**: flows are namespaced by tenant/team/user scopes with deterministic resolution order.
- **Runtime guarantees**: retries, fan-out/fan-in strategies, idempotency, and error envelopes are handled by the shared runtime so flow authors focus on business logic.

The rest of the guide walks through contracts, execution semantics, and tenancy features, culminating in a concrete weather flow example.
