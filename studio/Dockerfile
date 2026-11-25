ARG RUST_VERSION=1.93.1
ARG NODE_VERSION=26.4.0

FROM rust:${RUST_VERSION}-alpine AS apigen
RUN apk add make

WORKDIR /src
COPY . .
WORKDIR /src/studio
RUN make ui-gen-api

FROM node:${NODE_VERSION}-alpine AS frontend
RUN apk add make

WORKDIR /src
COPY . .
COPY --from=apigen /src/studio/web-app/src/app/api/generated /src/studio/web-app/src/app/api/generated
WORKDIR /src/studio
RUN make ui-setup ui-build

FROM rust:${RUST_VERSION}-alpine AS backend
RUN apk add make musl-dev perl

WORKDIR /src
COPY . .
COPY --from=frontend /src/studio/web-app/dist /src/studio/web-app/dist
WORKDIR /src/studio
RUN make build

FROM gcr.io/distroless/static-debian13
WORKDIR /app
COPY --from=backend /src/target/release/mylife-home-studio /app
COPY ./studio/config.docker.toml /app/config.toml
ENTRYPOINT ["/app/mylife-home-studio"]
