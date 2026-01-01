FROM lukemathwalker/cargo-chef:latest-rust-1.92.0 AS chef
WORKDIR /app
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add wasm32-unknown-unknown
RUN cargo install trunk

FROM chef AS planner
# ВАЖНО: Копируем только файлы манифестов и lock файл
# Это предотвращает инвалидацию кэша при изменении исходного кода
COPY Cargo.toml Cargo.lock ./
COPY crates/backend/Cargo.toml crates/backend/Cargo.toml
COPY crates/frontend/Cargo.toml crates/frontend/Cargo.toml
COPY crates/shared/Cargo.toml crates/shared/Cargo.toml

# Создаем заглушки исходного кода, чтобы cargo chef мог работать
# cargo chef prepare требует наличие src/lib.rs или src/main.rs для каждого member
RUN mkdir -p crates/backend/src && echo "fn main() {}" > crates/backend/src/main.rs
RUN mkdir -p crates/frontend/src && echo "fn main() {}" > crates/frontend/src/main.rs
RUN mkdir -p crates/shared/src && echo "fn main() {}" > crates/shared/src/lib.rs

RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder-backend
COPY --from=planner /app/recipe.json recipe.json
# Этот шаг (cook) теперь будет пересобираться ТОЛЬКО если изменились зависимости в Cargo.toml
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

# Копируем исходники ТОЛЬКО backend и shared
# Изменения во frontend не затронут этот слой
COPY crates/backend crates/backend
COPY crates/shared crates/shared
COPY Cargo.lock Cargo.toml ./

# Создаем фейковый frontend, чтобы workspace не сломался при сборке backend
# Cargo требует наличие всех member-ов workspace
RUN mkdir -p crates/frontend/src && \
    echo 'fn main() {}' > crates/frontend/src/main.rs && \
    echo '[package]\nname = "frontend"\nversion = "0.1.0"\nedition = "2021"\n[dependencies]\nleptos = { version = "0.6" }' > crates/frontend/Cargo.toml

ENV SQLX_OFFLINE=true
COPY .sqlx .sqlx

RUN cargo build --release --target x86_64-unknown-linux-musl --bin backend

FROM chef AS builder-frontend
COPY --from=planner /app/recipe.json recipe.json
# Cook frontend зависимостей
RUN cargo chef cook --release --target wasm32-unknown-unknown --package frontend --recipe-path recipe.json

# Для фронтенда копируем всё, так как trunk нужен доступ ко всему контексту
COPY crates crates
COPY Cargo.lock Cargo.toml ./

COPY .env.docker /tmp/.env.docker
COPY conf/load-env.sh /tmp/load-env.sh

WORKDIR /app/crates/frontend

# Используем load-env.sh для загрузки переменных и запуска trunk
RUN chmod +x /tmp/load-env.sh && \
    /tmp/load-env.sh /tmp/.env.docker trunk build --release

FROM scratch AS backend

COPY --from=builder-backend /app/target/x86_64-unknown-linux-musl/release/backend /backend

CMD ["/backend"]

FROM nginx:alpine-slim AS frontend

RUN rm -rf /usr/share/nginx/html/*
COPY --from=builder-frontend /app/crates/frontend/dist /usr/share/nginx/html

COPY conf/nginx.conf /etc/nginx/conf.d/default.conf