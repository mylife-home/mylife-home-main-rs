PWD := $(CURDIR)
DOCKER_REPOSITORY ?= vincenttr
DOCKER_PACKAGE_NAME ?= mylife-home-ui
DOCKER_PACKAGE_VERSION ?= $(shell cargo metadata --no-deps --quiet --format-version 1 | jq -r '.packages[] | select(.name == "mylife-home-ui") | .version')
DOCKER_IMAGE_TAG ?= $(DOCKER_REPOSITORY)/$(DOCKER_PACKAGE_NAME):$(DOCKER_PACKAGE_VERSION)
DOCKER_IMAGE_LATEST_TAG ?= $(DOCKER_REPOSITORY)/$(DOCKER_PACKAGE_NAME):latest

.PHONY: ui-setup ui-gen-api ui-run ui-build run build fmt docker-publish docker-build

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
	cargo build --release --target x86_64-unknown-linux-musl --manifest-path mylife-home-ui/Cargo.toml

fmt:
	cargo fmt --all

docker-publish: docker-build
	docker push "$(DOCKER_IMAGE_TAG)"
	docker push "$(DOCKER_IMAGE_LATEST_TAG)"

docker-build:
	docker build --pull -t "$(DOCKER_IMAGE_TAG)" -t "$(DOCKER_IMAGE_LATEST_TAG)" -f Dockerfile ..

docker-run:
	docker run -e BUS_SERVER=rpi-dev-home-main:1883 -p 8001:80 -ti --rm --name mylife-home-ui vincenttr/mylife-home-ui