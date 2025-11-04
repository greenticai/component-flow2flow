# Tenancy & Resolution

Publishing and resolving flows is scope-aware. Flows are registered under namespaces:

```
/tenants/{tenant}/teams/{team}/users/{user}/flows/{flow_id}@{major}
```

Resolution order for a given `{flow_id, caller_scope}`:

1. Tenant + team + user
2. Tenant + team
3. Tenant + user
4. Tenant only
5. Global (`/global/flows/...`)

This allows teams/users to override default behaviour while falling back to tenant/global implementations.

## ACLs

Inbound nodes declare an `allow` list of caller identifiers. Patterns follow a simple convention:

- `*` – allow any caller.
- `acme` – allow tenant `acme`.
- `acme:ops` – allow tenant+team combinations.
- `acme:ops:alice` – allow a specific user.

Publishers can mix wildcard entries with explicit callers (e.g. `connector.weather.*`). The resolver applies these checks during `resolve`, and `can_call` is exposed for diagnostics.

## Publishing Workflow

1. Validate the contract: `f2f validate examples/weather.yml`.
2. Publish to a tenant scope: `f2f publish examples/weather.yml --tenant acme`.
3. Optionally override for a team: `f2f publish examples/weather.yml --tenant acme --team sales-na`.
4. Resolve during execution: `f2f resolve assistant.weather.daily --tenant acme --team sales-na`.

The CLI uses the in-memory registry by default when `--registry` is omitted, but can point to a live service via `--registry URL` once integrated.
