FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS planner

WORKDIR /app

RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl

COPY src src
COPY Cargo.lock Cargo.toml ./

RUN cargo chef prepare --recipe-path recipe.json

FROM planner AS builder

COPY --from=planner /app/recipe.json recipe.json

RUN cargo chef cook --release --recipe-path recipe.json

COPY src src
COPY Cargo.lock Cargo.toml ./

RUN cargo build --target x86_64-unknown-linux-musl --release --bin dnd-back

FROM scratch AS runtime

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/dnd-back /usr/local/bin/

CMD ["app"]