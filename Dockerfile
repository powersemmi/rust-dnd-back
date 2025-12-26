FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

FROM chef AS planner
COPY crates crates
COPY Cargo.lock Cargo.toml ./

RUN cargo chef prepare --recipe-path recipe.json

FROM planner AS builder-backend
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY crates crates
COPY Cargo.lock Cargo.toml ./

RUN cargo build --release --target x86_64-unknown-linux-musl --bin backend

FROM chef AS builder-frontend
COPY --from=planner /app/recipe.json recipe.json
# Собираем только зависимости frontend для WASM
RUN cargo chef cook --release --target wasm32-unknown-unknown --package frontend --recipe-path recipe.json

COPY crates crates
COPY Cargo.lock Cargo.toml ./

WORKDIR /app/crates/frontend
RUN trunk build --release

FROM scratch AS backend

COPY --from=builder-backend /app/target/x86_64-unknown-linux-musl/release/backend /backend

CMD ["/backend"]

FROM nginx:alpine-slim AS frontend

RUN rm -rf /usr/share/nginx/html/*
COPY --from=builder-frontend /app/crates/frontend/dist /usr/share/nginx/html

COPY conf/nginx.conf /etc/nginx/conf.d/default.conf