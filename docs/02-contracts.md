# Contracts: In / Call / Out

F2F contracts are defined in YAML/JSON using named maps. Each node is keyed and contains an `f2f.*` payload. Three node kinds are supported:

- `f2f.in` (entry) – describes inbound intent, parameters, and ACL.
- `f2f.call` (step) – invokes a component/flow with mapping, retries, scope.
- `f2f.out` (respond) – normalises the final payload returned to the caller.

## In Nodes

```yaml
entry:
  f2f.in:
    intent: weather.get
    params:
      location:
        kind: string
        required: true
        description: Location identifier
      date:
        kind: string
        required: false
        default: today
    visibility: ["tenant_scoped", "team_overridable"]
    allow: ["*", "assistant.weather.*", "connector.*"]
```

- **Params** are keyed by name; each entry declares type metadata and optional `default`/`description`.
- **Visibility** is a list of tags the runtime interprets (tenant + team overrides today; user overrides are planned).
- **Allow** is a list of caller identifiers. `*` grants universal access, otherwise values follow `tenant:team:user` conventions.

## Call Nodes

```yaml
fetch:
  f2f.call:
    target: connector.weather.get@1
    mode: sync
    timeout_ms: 1000
    retry:
      attempts: 2
      delay_ms: 150
    params_map:
      location: "{{ params.location }}"
      date: "{{ params.date | default('today') }}"
    result_map:
      forecast_date: "{{ result.payload.forecast_date }}"
      temp_c: "{{ result.payload.temp_c }}"
    on_error:
      route: faq.search
      mapper: "{ \"q\": \"weather service is down\" }"
```

- **params_map/result_map** are Jinja-style expressions. The runtime renders them against `ctx` and the latest `result`.
- **retry** currently supports fixed delays; exponential backoff metadata can be layered on top in future iterations.
- **on_error** reroutes to a fallback flow, providing a JSON mapper template as a string.

## Out Nodes

```yaml
respond:
  f2f.out:
    returns:
      forecast_date:
        kind: string
        required: true
      temp_c:
        kind: number
        required: true
      description:
        kind: string
        required: false
```

Out nodes define the response contract. The runtime enforces required fields and applies defaults where provided before completing the flow.
