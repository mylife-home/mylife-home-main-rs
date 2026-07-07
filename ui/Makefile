PWD := $(CURDIR)

.PHONY: run build fmt

ui-setup:
	cd web-app && npm install
	cd web-app/bundler && npm install


ui-run:
	cd web-app/bundler && npm run dev

ui-build:
	cd web-app/bundler && npm run build	

run:
	RUST_LOG=$${RUST_LOG:-debug} cargo run --manifest-path mylife-home-ui/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-ui/Cargo.toml

fmt:
	cargo fmt --all