PWD := $(CURDIR)

.PHONY: run build fmt

ui-setup:
	cd web-app && npm install
	cd web-app/bundler && npm install

ui-gen-api:
	TS_RS_EXPORT_DIR=web-app/src/app/api cargo run --manifest-path web-api/Cargo.toml

ui-run:
	cd web-app/bundler && npm run dev

ui-build:
	cd web-app/bundler && npm run build	

run:
	cargo run --manifest-path mylife-home-ui/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-ui/Cargo.toml

fmt:
	cargo fmt --all