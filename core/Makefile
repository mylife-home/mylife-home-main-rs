PWD := $(CURDIR)

.PHONY: run build

run:
	cargo run --manifest-path mylife-home-core/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-core/Cargo.toml

include $(dir $(lastword $(MAKEFILE_LIST)))../rust.mk
