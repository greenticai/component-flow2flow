# component-flow2flow

Flow-to-flow experimentation sandbox that now ships as a Greentic component. The repository includes the component surface (manifest, schemas, WIT) plus supporting crates for contracts, runtime execution, router integration, and a local CLI.

## Greentic component

- Requirements: Rust 1.89+, `rustup target add wasm32-wasip2`, and the `greentic-component` CLI.
- `make build` runs `greentic-component build` against `component.manifest.json` and produces `target/wasm32-wasip2/release/component_flow2flow.wasm`.
- `make wasm` builds the WASI-P2 artifact directly; `make check` performs a target check; `make lint` / `make fmt` cover the workspace.
- Update the manifest hash after rebuilding: `greentic-component inspect --json target/wasm32-wasip2/release/component_flow2flow.wasm`.
- Schemas live in `schemas/`, WIT in `wit/`, and basic conformance tests under `tests/`.

## Workspace layout

- `flow2flow-contract` – Data types, schemas, and validation helpers for flow definitions.
- `flow2flow-runtime` – Core runtime primitives for executing validated flows.
- `flow2flow-router-adapter` – Adapter for registering runtimes with router infrastructure.
- `flow2flow-cli` – Local CLI harness for validating and running flows.
- `examples` – Sample flow definitions used across tooling.
- `conformance` – Early conformance and idempotency utilities.

`make test` exercises the full workspace; `make workspace-build` and `make workspace-check` keep the supporting crates healthy.

## Documentation & Policy

- [Overview](docs/01-overview.md)
- [Contracts](docs/02-contracts.md)
- [Execution](docs/03-execution.md)
- [Tenancy](docs/04-tenancy.md)
- [Versioning & Release Policy](docs/05-versioning.md)
