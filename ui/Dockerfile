ARG RUST_VERSION=1.93.1
ARG NODE_VERSION=26.4.0

FROM rust:${RUST_VERSION}-alpine AS apigen
RUN apk add make

WORKDIR /src
COPY . .
WORKDIR /src/ui
RUN make ui-gen-api

FROM node:${NODE_VERSION}-alpine AS frontend
RUN apk add make

WORKDIR /src
COPY . .
COPY --from=apigen /src/ui/web-app/src/app/api /src/ui/web-app/src/app/api
WORKDIR /src/ui
RUN make ui-setup ui-build

FROM rust:${RUST_VERSION}-alpine AS backend
RUN apk add make

WORKDIR /src
COPY . .
COPY --from=frontend /src/ui/web-app/dist /src/ui/web-app/dist
WORKDIR /src/ui
RUN make build

FROM gcr.io/distroless/static-debian13
WORKDIR /app
COPY --from=backend /src/target/x86_64-unknown-linux-musl/release/mylife-home-ui /app
COPY ./ui/config.docker.toml /app/config.toml
ENTRYPOINT ["/app/mylife-home-ui"]