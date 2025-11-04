# component-flow2flow

Flow-to-flow experimentation sandbox that validates contracts, executes runtimes, and integrates with a router adapter. The workspace is intentionally minimal to keep early iterations fast while providing a place for conformance tests and sample flows.

## Workspace layout

- `flow2flow-contract` – Data types, schemas, and validation helpers for flow definitions.
- `flow2flow-runtime` – Core runtime primitives for executing validated flows.
- `flow2flow-router-adapter` – Adapter for registering runtimes with router infrastructure.
- `flow2flow-cli` – Local CLI harness for validating and running flows.
- `examples` – Sample flow definitions used across tooling.
- `conformance` – Early conformance and idempotency utilities.

## Getting started

```bash
make build  # cargo build --workspace --all-targets
make test   # cargo test --workspace
make fmt    # cargo fmt --all
make lint   # cargo clippy --workspace --all-targets -- -D warnings
make check  # cargo check --workspace --all-targets
```

The workspace targets the Rust 1.74 toolchain and stable. CI mirrors these flows through GitHub Actions.

## Documentation & Policy

- [Overview](docs/01-overview.md)
- [Contracts](docs/02-contracts.md)
- [Execution](docs/03-execution.md)
- [Tenancy](docs/04-tenancy.md)
- [Versioning & Release Policy](docs/05-versioning.md)
