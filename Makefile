BUILD_FLAGS ?=
CLIPPY_ARGS ?= --workspace --all-targets

.PHONY: build wasm flows check lint fmt test workspace-build workspace-check

default: build

build:
	greentic-component build $(BUILD_FLAGS)

flows:
	greentic-component flow scaffold --force

wasm:
	cargo build --target wasm32-wasip2 --release

check:
	cargo check --target wasm32-wasip2

lint:
	cargo fmt --all
	cargo clippy $(CLIPPY_ARGS) -- -D warnings

fmt:
	cargo fmt --all

test:
	cargo test --workspace --all-targets

workspace-build:
	cargo build --workspace --all-targets

workspace-check:
	cargo check --workspace --all-targets
