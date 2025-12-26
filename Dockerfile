FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl

FROM chef AS planner
COPY crates crates
COPY Cargo.lock Cargo.toml ./

RUN cargo chef prepare --recipe-path recipe.json

FROM planner AS backend-builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY crates crates
COPY Cargo.lock Cargo.toml ./

RUN cargo build --release --target x86_64-unknown-linux-musl --bin backend

FROM scratch AS backend

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/backend /backend

CMD ["/backend"]

#FROM scratch AS frontend
#
#COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/frontend /frontend
#
#CMD ["/frontend"]