# Execution Semantics

The runtime turns contracts into executable flows. Every invocation starts with a context (`Ctx`) containing:

- `params`: validated inbound payload.
- `state`: mutable per-invocation data (result maps write here).
- `meta`: `{ scope, channel, correlation_id }`.
- `deadline`: optional overall timeout.
- `idempotency_key`: optional replay key.

## Retries and Backoff

Retries are configured per `f2f.call` node:

```yaml
retry:
  max_attempts: 3
  backoff: exp
  base_ms: 200
```

The runtime retries retryable errors (timeout/retryable kinds). Exponential backoff grows until `max_delay_ms` (if specified).

## Join Semantics

Fan-out calls can join using `join.strategy`:

- `all` (default): collect every result into an array.
- `any`: return the first non-null value.

`join.with` enumerates the branches being joined; the runtime tracks them for observability.

## Error Envelope

Call results map into a normalised error envelope when failures propagate:

```json
{
  "code": "connector.timeout",
  "message": "connector.weather.get timed out",
  "retryable": true,
  "detail": { ... }
}
```

Fallback policies (`policy: fallback`) receive the error context in `ctx.error` so they can map diagnostic payloads.

## Idempotency

When `ctx.idempotency_key` is set, the runtime consults the configured `IdempotencyStore`. Cached responses short-circuit execution, ensuring repeated requests from the same upstream don’t double-trigger effects.
