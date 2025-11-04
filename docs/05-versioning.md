# Versioning & Release Policy

We follow semantic versioning for every crate in the workspace starting at `0.1.0`.

- **flow2flow-contract**: bump the **minor** version when adding optional fields or backwards-compatible validators; bump the **major** version when removing/renaming fields or changing validation defaults.
- **flow2flow-runtime** / **router-adapter** / **cli**: bump **minor** for additive behaviour or new flags; bump **major** for breaking API changes, CLI incompatibilities, or new required inputs.
- **conformance**: treated as a library; bump **minor** for new test helpers, **major** for signature changes.

Patch releases (`x.y.z`) are reserved for bug fixes that do not alter public APIs.

## Release Workflow

1. Ensure `cargo test`, `cargo test -p conformance`, and `cargo clippy -- -D warnings` pass locally.
2. Run `cargo publish -p <crate> --dry-run` to confirm the crate metadata before tagging.
3. Tag the repository (`git tag vX.Y.Z`) and push the tag. The `publish` workflow performs a dry-run publish for every crate to enforce workspace consistency.
4. Use the `Publish` workflow (`workflow_dispatch`) to perform the actual publish once the dry-run succeeds.

## Security Guardrails

- CI forbids warnings in Clippy and Rustc (`cargo clippy -- -D warnings`).
- No secrets should be logged; runtime diagnostics only emit identifiers (for example, permission misses return pattern keys, not raw payloads).
- Publishing requires `CARGO_REGISTRY_TOKEN` and runs in an isolated job to keep credentials out of build logs.
