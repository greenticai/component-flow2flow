# Repository Overview

## 1. High-Level Purpose
- Rust workspace for defining, validating, and executing “flow-to-flow” contracts with supporting runtime, router adapter, CLI tooling, examples, and conformance utilities.
- Hosts a Greentic WASI-P2 component (`component-flow2flow`) that exposes a minimal messaging surface and manifest/schemas suitable for `greentic-component build`.

## 2. Main Components and Functionality
- **Path:** `src/`, `component.manifest.json`, `schemas/`, `wit/`
  - **Role:** Greentic component crate `component-flow2flow` (cdylib/rlib) built to WASI-P2.
  - **Key functionality:** Exposes `get-manifest`, lifecycle hooks, and echo-style `invoke`/`invoke-stream`; manifest declares messaging support, stateless profile, and inferred config schema with default echo prefix; schemas define component/input/output; WIT world defines config record; `flows/default.ygtc` and `flows/custom.ygtc` are scaffolded by the greentic build.
  - **Key dependencies / integration points:** Uses `greentic-interfaces-guest` 0.4 for exports; built via `greentic-component build` targeting `wasm32-wasip2`.

- **Path:** `flow2flow-contract/`
  - **Role:** Contract definitions and validation for flows.
  - **Key functionality:** Data models for params, retry, error handling, joins, scopes; validation errors; schema generation helpers; loaders for JSON/YAML flow specs with deprecation detection.

- **Path:** `flow2flow-runtime/`
  - **Role:** Executes validated flow specs.
  - **Key functionality:** Context/meta management; inbound validation (`exec_in`), call execution with retries/fallbacks/idempotency (`exec_call`), outbound validation (`exec_out`); template engine for param/result mapping; legacy runtime builder for simple step lists.
  - **Key dependencies / integration points:** Consumes `flow2flow-contract`; relies on a `Resolver` to route calls and optional `IdempotencyStore`.

- **Path:** `flow2flow-router-adapter/`
  - **Role:** Adapter to register and resolve flow signatures against scopes/versions.
  - **Key functionality:** In-memory registry, scope resolution with wildcard matching, version requirement parsing, signature building from specs; implements `Resolver` to return routing payloads.
  - **Key dependencies / integration points:** Uses `flow2flow-contract` specs and `flow2flow-runtime::Resolver` trait; semver-based versioning.

- **Path:** `flow2flow-cli/`
  - **Role:** Developer CLI.
  - **Key functionality:** Commands to validate flow definitions, publish to in-memory registry, resolve flows by scope/version, and run flows locally using stub resolver; snapshot-based integration tests.
  - **Key dependencies / integration points:** Integrates contracts/runtime/router adapter; clap-based interface; optional in-memory registry feature.

- **Path:** `examples/`
  - **Role:** Sample flow runtimes.
  - **Key functionality:** Constructs simple runtimes (weather/order/faq) via legacy helper; verifies execution traces.

- **Path:** `conformance/`
  - **Role:** Conformance helpers and tests.
  - **Key functionality:** Idempotency and trace uniqueness checks; test support for registering signatures, mock resolvers, and in-memory idempotency store.

- **Other:** `docs/` holds design/policy markdown; `Makefile` drives greentic builds and workspace tasks; GitHub Actions pin Rust 1.89.0.

## 3. Work In Progress, TODOs, and Stubs
- None detected (no TODO/FIXME markers or `todo!/unimplemented!` stubs).

## 4. Broken, Failing, or Conflicting Areas
- None observed. `cargo test --workspace --all-targets` passes (including component, contracts, runtime, router adapter, CLI, examples, conformance).

## 5. Notes for Future Work
- After rebuilding the component, refresh the manifest hash via `greentic-component inspect --json target/wasm32-wasip2/release/component_flow2flow.wasm`.
- Keep Rust toolchain at 1.89+ (aligned with greentic scaffold and CI).***
