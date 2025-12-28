# Docker Configuration

## Configuration Files

### `.env`
Used for local development with `cargo dotenv`. Contains variables for both backend and frontend.

### `.env.docker`
Used when building the frontend Docker image. Contains only frontend variables that are embedded into WASM at build time.

Create your `.env.docker` in the project root based on [dotenv/.env.docker.example](dotenv/.env.docker.example) and configure:
- `BACK_URL` - Your API URL (with `https://` for production)
- Theme colors (optional)

## Build and Run

```bash
# Local development
cp docs/dotenv/.env.example .env
cargo dotenv run --bin backend

# docker-compose (development)
cp docs/dotenv/.env.docker.example .env.docker
docker-compose up --build
```

## How It Works

1. **Backend**: Uses `.env` via `env_file` in docker-compose (runtime variables)
2. **Frontend**: Uses `.env.docker` which is loaded via the `conf/load-env.sh` script (compile-time variables)
3. **load-env.sh**: The script reads `.env.docker` line-by-line and exports all variables
```