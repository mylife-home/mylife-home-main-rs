PWD := $(CURDIR)

.PHONY: run build fmt

run:
	RUST_LOG=$${RUST_LOG:-debug} cargo run --manifest-path mylife-home-ui/Cargo.toml

build:
	cargo build --manifest-path mylife-home-ui/Cargo.toml

fmt:
	cargo fmt --all