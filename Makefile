CLIPPY_ARGS = --workspace --all-targets

.PHONY: build test lint fmt check

build:
	cargo build --workspace --all-targets

test:
	cargo test --workspace

lint:
	cargo clippy $(CLIPPY_ARGS) -- -D warnings

fmt:
	cargo fmt --all

check:
	cargo check --workspace --all-targets
