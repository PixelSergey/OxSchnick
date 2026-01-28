# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.92.0
ARG APP_NAME=fanschnick-server

FROM rust:${RUST_VERSION}-slim AS build
ARG APP_NAME
WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libpq-dev \
    libssl-dev \
    ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY . /app

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    --mount=type=cache,target=/app/target/ \
    --mount=type=cache,target=/usr/local/cargo/git/db \
    --mount=type=cache,target=/usr/local/cargo/registry/ \
    cargo build --locked --release && \
    cp ./target/release/$APP_NAME /bin/server

FROM debian:bookworm-slim AS final

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=build /bin/server /bin/server

ARG BASE_URL="http://localhost:8080"
ENV BASE_URL=$BASE_URL
ARG BIND_PORT=8080
ENV BIND_PORT=$BIND_PORT

EXPOSE $BIND_PORT

CMD ["/bin/sh", "-c", "/bin/server $BASE_URL 0.0.0.0:$BIND_PORT"]
