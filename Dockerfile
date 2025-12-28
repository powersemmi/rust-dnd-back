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

# Copy sqlx cashe
ENV SQLX_OFFLINE=true
COPY .sqlx .sqlx

RUN cargo build --release --target x86_64-unknown-linux-musl --bin backend

FROM chef AS builder-frontend
COPY --from=planner /app/recipe.json recipe.json
# Собираем только зависимости frontend для WASM
RUN cargo chef cook --release --target wasm32-unknown-unknown --package frontend --recipe-path recipe.json

COPY crates crates
COPY Cargo.lock Cargo.toml ./

WORKDIR /app/crates/frontend

# Объявляем build-time переменные для frontend
## api
ARG BACK_URL
ARG WS_PATH
ARG API_PATH

ENV BACK_URL=${BACK_URL}
ENV WS_PATH=${WS_PATH}
ENV API_PATH=${API_PATH}

## themes
ARG MY_CURSOR_COLOR
ARG OTHER_CURSOR_COLOR
ARG CURSOR_SIZE
ARG MOUSE_THROTTLE_MS
ARG BACKGROUND_COLOR
ARG CURSOR_TRANSITION

ENV MY_CURSOR_COLOR=${MY_CURSOR_COLOR}
ENV OTHER_CURSOR_COLOR=${OTHER_CURSOR_COLOR}
ENV CURSOR_SIZE=${CURSOR_SIZE}
ENV MOUSE_THROTTLE_MS=${MOUSE_THROTTLE_MS}
ENV BACKGROUND_COLOR=${BACKGROUND_COLOR}
ENV CURSOR_TRANSITION=${CURSOR_TRANSITION}

RUN trunk build --release

FROM scratch AS backend

COPY --from=builder-backend /app/target/x86_64-unknown-linux-musl/release/backend /backend

CMD ["/backend"]

FROM nginx:alpine-slim AS frontend

RUN rm -rf /usr/share/nginx/html/*
COPY --from=builder-frontend /app/crates/frontend/dist /usr/share/nginx/html

COPY conf/nginx.conf /etc/nginx/conf.d/default.conf