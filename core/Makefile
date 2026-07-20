PWD := $(CURDIR)

.PHONY: run build fmt

run:
	cargo run --manifest-path mylife-home-core/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-core/Cargo.toml

fmt:
	cargo fmt --all