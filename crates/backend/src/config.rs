use dotenvy::dotenv;
use std::env;
use std::net::SocketAddr;
use std::sync::OnceLock;

const DEFAULT_DATABASE_URL: &str = "postgres://user:password@localhost:5432/dnd_db";
const DEFAULT_REDIS_URL: &str = "redis://default@localhost:6379";
const DEFAULT_SERVER_HOST: &str = "127.0.0.1";
const DEFAULT_SERVER_PORT: &str = "3000";
const DEFAULT_ALLOWED_ORIGIN: &str = "http://localhost:8080";
const DEFAULT_DATABASE_MAX_CONNECTIONS: u32 = 5;
const DEFAULT_RUST_LOG: &str = "dnd_back=debug,tower_http=debug";

pub struct Config {
    pub server_addr: SocketAddr,
    pub redis_url: String,
    pub database_url: String,
    pub allowed_origin: String,
    pub database_max_connections: u32,
    pub rust_log: String,
}

impl Config {
    pub fn init() -> Self {
        dotenv().ok();

        let host = env::var("SERVER_HOST").unwrap_or_else(|_| DEFAULT_SERVER_HOST.into());
        let port = env::var("SERVER_PORT").unwrap_or_else(|_| DEFAULT_SERVER_PORT.into());
        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.into());
        let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| DEFAULT_REDIS_URL.into());
        let allowed_origin =
            env::var("ALLOWED_ORIGIN").unwrap_or_else(|_| DEFAULT_ALLOWED_ORIGIN.into());
        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .map(|value| {
                value
                    .parse()
                    .expect("DATABASE_MAX_CONNECTIONS must be a valid u32")
            })
            .unwrap_or(DEFAULT_DATABASE_MAX_CONNECTIONS);
        let rust_log = env::var("RUST_LOG").unwrap_or_else(|_| DEFAULT_RUST_LOG.into());

        let server_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .expect("Failed to parse SERVER_HOST:SERVER_PORT");

        Self {
            server_addr,
            redis_url,
            database_url,
            allowed_origin,
            database_max_connections,
            rust_log,
        }
    }
}

pub fn get_secret(var: &str) -> &'static str {
    match var {
        "JWT_SECRET" => {
            static JWT_SECRET: OnceLock<String> = OnceLock::new();
            JWT_SECRET.get_or_init(|| load_secret("JWT_SECRET"))
        }
        "AUTH_SECRET" => {
            static AUTH_SECRET: OnceLock<String> = OnceLock::new();
            AUTH_SECRET.get_or_init(|| load_secret("AUTH_SECRET"))
        }
        _ => panic!("Unsupported secret env var: {var}"),
    }
}

fn load_secret(var: &str) -> String {
    dotenv().ok();
    env::var(var).unwrap_or_else(|_| panic!("{var} must be set"))
}
