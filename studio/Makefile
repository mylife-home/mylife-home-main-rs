PWD := $(CURDIR)
DOCKER_REPOSITORY ?= vincenttr
DOCKER_PACKAGE_NAME ?= mylife-home-studio
DOCKER_PACKAGE_VERSION ?= $(shell cargo metadata --no-deps --quiet --format-version 1 | jq -r '.packages[] | select(.name == "mylife-home-studio") | .version')
DOCKER_IMAGE_TAG ?= $(DOCKER_REPOSITORY)/$(DOCKER_PACKAGE_NAME):$(DOCKER_PACKAGE_VERSION)
DOCKER_IMAGE_LATEST_TAG ?= $(DOCKER_REPOSITORY)/$(DOCKER_PACKAGE_NAME):latest

.PHONY: ui-setup ui-gen-api ui-run ui-check ui-build run build docker-publish docker-build docker-run

ui-setup:
	cd web-app && npm install
	cd web-app/bundler && npm install

ui-gen-api:
	TS_RS_EXPORT_DIR=web-app/src/app/api/generated cargo run --manifest-path web-api/Cargo.toml

ui-run:
	cd web-app/bundler && npm run dev

ui-check:
	cd web-app && npm run typecheck-watch

ui-build:
	cd web-app && npm run typecheck
	cd web-app/bundler && npm run build	

run:
	RUST_LOG=$${RUST_LOG:-debug} cargo run --manifest-path mylife-home-studio/Cargo.toml

build:
	cargo build --release --manifest-path mylife-home-studio/Cargo.toml

docker-publish: docker-build
	docker push "$(DOCKER_IMAGE_TAG)"
	docker push "$(DOCKER_IMAGE_LATEST_TAG)"

docker-build:
	docker build --pull -t "$(DOCKER_IMAGE_TAG)" -t "$(DOCKER_IMAGE_LATEST_TAG)" -f Dockerfile ..

docker-run:
	docker run -e BUS_SERVER=rpi-dev-home-main:1883 -p 8002:80 -v mylife-home-studio-data:/data -ti --rm --name mylife-home-studio vincenttr/mylife-home-studio

include $(dir $(lastword $(MAKEFILE_LIST)))../rust.mk
