PWD := $(CURDIR)

.PHONY: run build fmt

ui-setup:
	cd webapp && npm install
	cd webapp/bundler && npm install

ui-run:
	cd webapp/bundler && npm run dev

ui-build:
	cd webapp/bundler && npm run build	

run:
	RUST_LOG=$${RUST_LOG:-debug} cargo run --manifest-path mylife-home-ui/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-ui/Cargo.toml

fmt:
	cargo fmt --all