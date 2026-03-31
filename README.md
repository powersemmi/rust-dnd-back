# DnD VTT - Virtual Tabletop

Web-based virtual tabletop for D&D sessions. Real-time scene board with tokens, chat, voting, notes, and end-to-end encrypted WebSocket communication.

**Stack:** Rust (Axum backend + Leptos WASM frontend), PostgreSQL, Redis, Docker.

---

## Quick start (Docker Compose)

The fastest way to run everything locally or on a VDS.

### Prerequisites

- [Docker](https://docs.docker.com/get-docker/) >= 24
- [Docker Compose](https://docs.docker.com/compose/install/) >= 2.20

### 1. Clone and configure

```bash
git clone <repo-url>
cd dnd-back
```

Copy the env file and fill in secrets:

```bash
cp .env.example .env   # or copy .env manually - see below
```

Edit `.env`:

```env
# Backend
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
DATABASE_URL=postgres://user:password@postgres:5432/dnd_db
REDIS_URL=redis://redis:6379
JWT_SECRET=change_me_to_a_long_random_string
AUTH_SECRET=change_me_to_32_bytes_hex_string   # AES-256 key for TOTP encryption
RUST_LOG=info

# Frontend (compile-time - baked into the WASM binary)
BACK_URL=http://localhost:3000     # or http://<your-server-ip>:3000
WS_PATH=/ws/room
API_PATH=/api
STATE_SYNC_SHARED_SECRET=change_me_to_a_long_random_string
```

> **ALLOWED_ORIGIN** is set in `docker-compose.yaml` automatically to `http://127.0.0.1:8080`.
> Change it if you expose the service on a different host/port.

Copy the frontend build config:

```bash
cp .env.docker .env.docker   # already present; edit BACK_URL if needed
```

Edit `.env.docker` - set `BACK_URL` to the public address of the backend:

```env
# For local access:
BACK_URL=http://localhost:3000

# For a VDS (replace with your IP or domain):
BACK_URL=http://YOUR_SERVER_IP:3000
```

### 2. Build and run

```bash
docker compose up --build
```

First build takes ~10-15 minutes (Rust compilation). Subsequent builds with changed code are faster thanks to layer caching.

The services start on:

| Service  | URL                          |
|----------|------------------------------|
| Frontend | http://localhost:8080        |
| Backend  | http://localhost:3000        |
| Swagger  | http://localhost:3000/docs   |

### 3. Create an account

Open http://localhost:8080, click **Register**, enter a username.
The registration returns a **TOTP QR code** - scan it with any authenticator app (Google Authenticator, Aegis, etc.).
Use the 6-digit code from the app to log in.

---

## Local development (without Docker)

Requires Rust toolchain and running PostgreSQL + Redis.

### Prerequisites

```bash
# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Trunk (frontend bundler)
cargo install trunk

# SQLx CLI (database migrations)
cargo install sqlx-cli --no-default-features --features postgres

# PostgreSQL and Redis (example for Debian/Ubuntu)
sudo apt install postgresql redis-server
```

### 1. Start PostgreSQL and Redis

```bash
sudo systemctl start postgresql redis
```

Create the database:

```bash
sudo -u postgres psql -c "CREATE USER user WITH PASSWORD 'password';"
sudo -u postgres psql -c "CREATE DATABASE dnd_db OWNER user;"
```

### 2. Configure environment

```bash
cp .env.example .env   # or create .env manually
```

`.env` (backend reads this at startup):

```env
SERVER_HOST=127.0.0.1
SERVER_PORT=3000
DATABASE_URL=postgres://user:password@localhost:5432/dnd_db
REDIS_URL=redis://default@localhost:6379
JWT_SECRET=dev_jwt_secret_change_in_prod
AUTH_SECRET=dev_auth_secret_change_in_prod
RUST_LOG=dnd_back=debug,tower_http=debug
ALLOWED_ORIGIN=http://localhost:8080
```

### 3. Run database migrations

```bash
cd crates/backend
DATABASE_URL=postgres://user:password@localhost:5432/dnd_db sqlx migrate run
cd ../..
```

### 4. Start the backend

```bash
cargo run -p backend
```

### 5. Start the frontend

In a separate terminal, load the build-time env and start Trunk:

```bash
set -a; source .env.docker; set +a
cd crates/frontend
trunk serve --port 8080 --open
```

Open http://localhost:8080.

---

## Deploying on a VDS

### Option A: Docker Compose on the server (recommended)

1. Install Docker on the VDS:

   ```bash
   curl -fsSL https://get.docker.com | sh
   ```

2. Copy the project to the server (git clone or rsync):

   ```bash
   rsync -avz --exclude target --exclude .git . user@YOUR_IP:/opt/dnd-back/
   ```

3. On the server, edit `.env.docker` - set `BACK_URL` to the public backend address:

   ```env
   BACK_URL=http://YOUR_SERVER_IP:3000
   # or with a domain and HTTPS:
   BACK_URL=https://api.example.com
   ```

4. Edit `docker-compose.yaml` - update `ALLOWED_ORIGIN` for the backend:

   ```yaml
   - ALLOWED_ORIGIN=http://YOUR_SERVER_IP:8080
   # or:
   - ALLOWED_ORIGIN=https://example.com
   ```

5. Build and start:

   ```bash
   cd /opt/dnd-back
   docker compose up --build -d
   ```

6. (Optional) Restrict ports with a firewall and put Nginx/Caddy in front for HTTPS.

### Option B: Behind Nginx with HTTPS (production setup)

Use the bundled `conf/nginx.conf` as a reference. With Caddy it is even simpler - it handles TLS automatically:

```
example.com {
    reverse_proxy localhost:8080
}

api.example.com {
    reverse_proxy localhost:3000
}
```

Set environment variables accordingly:

```env
# .env.docker
BACK_URL=https://api.example.com

# docker-compose.yaml backend environment
ALLOWED_ORIGIN=https://example.com
```

---

## Environment variables reference

### Backend (`.env`)

| Variable               | Required | Default                             | Description                                      |
|------------------------|----------|-------------------------------------|--------------------------------------------------|
| `DATABASE_URL`         | yes      | `postgres://user:password@.../dnd_db` | PostgreSQL connection string                     |
| `REDIS_URL`            | yes      | `redis://default@localhost:6379`    | Redis connection string                          |
| `JWT_SECRET`           | yes      | -                                   | Secret for signing JWT tokens                    |
| `AUTH_SECRET`          | yes      | -                                   | AES-256 key for encrypting TOTP secrets          |
| `SERVER_HOST`          | no       | `127.0.0.1`                         | Bind address (`0.0.0.0` to accept all)           |
| `SERVER_PORT`          | no       | `3000`                              | Listen port                                      |
| `ALLOWED_ORIGIN`       | no       | `http://localhost:8080`             | CORS allowed origin (frontend URL)               |
| `RUST_LOG`             | no       | `dnd_back=debug,...`                | Log level filter                                 |

### Frontend (`.env.docker`, compile-time)

| Variable                    | Description                                          |
|-----------------------------|------------------------------------------------------|
| `BACK_URL`                  | Backend base URL (e.g. `http://localhost:3000`)      |
| `WS_PATH`                   | WebSocket endpoint path (`/ws/room`)                 |
| `API_PATH`                  | REST API path prefix (`/api`)                        |
| `STATE_SYNC_SHARED_SECRET`  | Shared secret for client-side snapshot encryption    |
| `MY_CURSOR_COLOR`           | Hex color for your own cursor                        |
| `OTHER_CURSOR_COLOR`        | Hex color for other users' cursors                   |
| `MOUSE_THROTTLE_MS`         | Cursor broadcast throttle in ms (default `50`)       |

---

## Useful commands

```bash
# Run all tests
cargo test --workspace

# Check for lint warnings
cargo clippy --workspace --all-targets --all-features

# Format code
cargo fmt

# Rebuild without clearing cache (fast)
docker compose up --build

# View logs
docker compose logs -f backend
docker compose logs -f frontend

# Stop and remove containers
docker compose down

# Stop and remove containers + volumes (wipes database!)
docker compose down -v
```

---

## Architecture overview

```
Browser (WASM)          Server
+------------------+    +-------------------+    +-----------+
|  Leptos frontend |    |   Axum backend    |    | Redis     |
|  (CSR, /ws/room) |<-->|   /ws/room  WS   |<-->| pub/sub   |
|                  |    |   /api/auth  REST  |    +-----------+
+------------------+    +-------------------+
                              |
                         +-----------+
                         | PostgreSQL|
                         +-----------+
```

- WebSocket messages are **end-to-end encrypted** (X25519 key exchange + ChaCha20-Poly1305).
- The backend is a **relay** - it never sees plaintext chat, notes, or sync snapshots.
- Room state is synced peer-to-peer via a version/hash chain with fork detection.
